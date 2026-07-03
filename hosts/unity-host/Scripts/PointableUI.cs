using System.Collections.Concurrent;
using UnityEngine;
using UnityEngine.EventSystems;

namespace KimoTech.LichoraHost
{
    /// <summary>
    /// MouseState 使用 struct 以减少 GC 压力（小对象值类型更高效）
    /// </summary>
    public struct MouseState
    {
        public bool Effective;
        public Vector2 Position;
        public Vector2 Delta;
        public bool LeftButton;
        public bool RightButton;
        public bool MiddleButton;
        public Vector2 ScrollDelta;
    }

    [DisallowMultipleComponent]
    [RequireComponent(typeof(RectTransform))]
    public class PointableUI
        : MonoBehaviour,
            IPointerEnterHandler,
            IPointerExitHandler,
            IPointerMoveHandler,
            IPointerDownHandler,
            IPointerUpHandler,
            IScrollHandler
    {
        protected MouseState _MouseState;
        protected readonly ConcurrentQueue<KeyboardState> _KeyboardEvents =
            new ConcurrentQueue<KeyboardState>();
        protected string _CompositionString = "";
        protected string _InputString = "";
        protected bool _HasFocus = false;
        protected bool _ImeActive = false;
        protected IImeInputSink _ImeInputSink;
        protected LinuxNativeImeModule _LinuxNativeImeModule;

        private RectTransform _RectTransform;
        protected RectTransform RectTransform
        {
            get
            {
                if (_RectTransform == null)
                {
                    _RectTransform = GetComponent<RectTransform>();
                }

                return _RectTransform;
            }
        }

        public void OnPointerEnter(PointerEventData eventData)
        {
            _MouseState.Effective = true;
        }

        public void OnPointerExit(PointerEventData eventData)
        {
            _MouseState.Effective = false;
        }

        public void OnPointerMove(PointerEventData eventData)
        {
            _MouseState.Effective = true;
            var position = MapPointerToBrowser(eventData.position);
            _MouseState.Delta += position - _MouseState.Position;
            _MouseState.Position = position;
        }

        public void OnPointerDown(PointerEventData eventData)
        {
            _HasFocus = true;
            Input.imeCompositionMode = IMECompositionMode.On;
            _LinuxNativeImeModule?.FocusIn();

            switch (eventData.button)
            {
                case PointerEventData.InputButton.Left:
                    _MouseState.LeftButton = true;
                    break;
                case PointerEventData.InputButton.Right:
                    _MouseState.RightButton = true;
                    break;
                case PointerEventData.InputButton.Middle:
                    _MouseState.MiddleButton = true;
                    break;
            }
        }

        public void OnPointerUp(PointerEventData eventData)
        {
            switch (eventData.button)
            {
                case PointerEventData.InputButton.Left:
                    _MouseState.LeftButton = false;
                    break;
                case PointerEventData.InputButton.Right:
                    _MouseState.RightButton = false;
                    break;
                case PointerEventData.InputButton.Middle:
                    _MouseState.MiddleButton = false;
                    break;
            }
        }

        public void OnScroll(PointerEventData eventData)
        {
            _MouseState.ScrollDelta = eventData.scrollDelta;
        }

        /// <summary>
        /// 清零滚轮增量，子类应在每帧处理完鼠标状态后调用
        /// </summary>
        protected void ResetScrollDelta()
        {
            _MouseState.ScrollDelta = Vector2.zero;
            _MouseState.Delta = Vector2.zero;
        }

        protected virtual void OnGUI()
        {
            if (!_HasFocus)
            {
                return;
            }

            var currentEvent = Event.current;

            if (currentEvent == null)
            {
                return;
            }

            _CompositionString = Input.compositionString;
            _InputString = Input.inputString;

            // 实时更新 IME 激活状态，避免依赖 LateUpdate 中的延迟更新
            // 当 compositionString 非空时，说明正在使用输入法进行组合输入（如中文拼音）
            _ImeActive = !string.IsNullOrEmpty(_CompositionString);

            // 处理剪贴板粘贴（Ctrl+V）
            if (currentEvent.type == EventType.ValidateCommand)
            {
                if (currentEvent.commandName == "Paste")
                {
                    currentEvent.Use(); // 标记事件为已处理
                }
            }
            else if (currentEvent.type == EventType.ExecuteCommand)
            {
                if (currentEvent.commandName == "Paste")
                {
                    // 从 Unity 剪贴板获取文本并通过 IME 模块发送
                    var clipboardText = GUIUtility.systemCopyBuffer;
                    if (!string.IsNullOrEmpty(clipboardText))
                    {
                        _ImeInputSink?.CommitImeText(clipboardText);
                    }

                    currentEvent.Use(); // 标记事件为已处理
                }
            }
            else if (currentEvent.type == EventType.KeyDown)
            {
                ProcessKeyEvent(currentEvent, KeyEventType.KeyDown);
            }
            else if (currentEvent.type == EventType.KeyUp)
            {
                ProcessKeyEvent(currentEvent, KeyEventType.KeyUp);
            }
        }

        private void ProcessKeyEvent(Event currentEvent, KeyEventType eventType)
        {
            if (
                _LinuxNativeImeModule != null
                && _LinuxNativeImeModule.ProcessKeyEvent(currentEvent, eventType)
            )
            {
                currentEvent.Use();
                return;
            }

            var modifiers = GetCurrentModifiers(currentEvent);
            var windowsKeyCode = KeyCodeMapper.ToVirtualKey(currentEvent.keyCode);
            var nativeKeyCode = KeyCodeMapper.ToNativeKey(
                currentEvent.keyCode,
                currentEvent.character
            );
            var hasValidKey = windowsKeyCode != 0 || currentEvent.keyCode != KeyCode.None;
            var hasValidChar = currentEvent.character != '\0';

            // 如果没有有效的按键也没有有效的字符，跳过
            if (!hasValidKey && !hasValidChar)
            {
                return;
            }

            // 发送 KeyDown/KeyUp 事件（仅当有有效按键时）
            if (hasValidKey)
            {
                var keyboardState = new KeyboardState
                {
                    HasEvent = true,
                    Type = eventType,
                    WindowsKeyCode = windowsKeyCode,
                    NativeKeyCode = nativeKeyCode,
                    Modifiers = modifiers,
                    Character = currentEvent.character,
                };

                _KeyboardEvents.Enqueue(keyboardState);
            }

            // 发送 Char 事件的条件判断：
            // 1. 必须是 KeyDown 事件（不处理 KeyUp 的字符）
            // 2. 必须有有效字符
            // 3. 当前没有 IME 组合正在进行（compositionString 为空）
            // 4. IME 输入状态未处于活跃状态（刚提交/取消的冷却期）
            // 5. 字符必须是 ASCII 字符（< 128）
            //
            // 关键修复：Unity IME 系统中，非 ASCII 字符（如中文、日文等）只会在
            // 选择候选词后出现在 inputString 中。这些字符会在 LateUpdate 中由
            // IImeInputSink.CommitImeText 统一处理，不应该在 OnGUI 中发送（否则会重复）。
            //
            // ASCII 字符（英文字母、数字、标点等）在普通英文输入时不经过 IME 组合，
            // 需要在 OnGUI 中发送；在 IME 拼音输入时会被 compositionString 或 _ImeActive 拦截。
            var isComposingNow = !string.IsNullOrEmpty(_CompositionString);
            var isAsciiChar = currentEvent.character < 128;

            var shouldSendChar =
                eventType == KeyEventType.KeyDown
                && hasValidChar
                && !isComposingNow
                && !_ImeActive
                && isAsciiChar;

            if (shouldSendChar)
            {
                var charState = new KeyboardState
                {
                    HasEvent = true,
                    Type = KeyEventType.Char,
                    WindowsKeyCode = currentEvent.character,
                    NativeKeyCode = currentEvent.character,
                    Modifiers = modifiers,
                    Character = currentEvent.character,
                };

                _KeyboardEvents.Enqueue(charState);
            }
        }

        private static KeyModifiers GetCurrentModifiers(Event e)
        {
            var modifiers = KeyModifiers.None;

            // 使用 Event.current 中的修饰键状态，与当前正在处理的事件精确对应。
            // OnGUI 在同一帧内可能被调用多次（每个事件一次），用 Input.GetKey 会取帧级状态，
            // 可能与当前事件的修饰键不一致。
            if (e.control)
            {
                modifiers |= KeyModifiers.Ctrl;
            }

            if (e.shift)
            {
                modifiers |= KeyModifiers.Shift;
            }

            if (e.alt)
            {
                modifiers |= KeyModifiers.Alt;
            }

            return modifiers;
        }

        public void ReleaseFocus()
        {
            _HasFocus = false;
            Input.imeCompositionMode = IMECompositionMode.Auto;
            _LinuxNativeImeModule?.FocusOut();
        }

        private Vector2 MapPointerToBrowser(Vector2 position)
        {
            RectTransformUtility.ScreenPointToLocalPointInRectangle(
                RectTransform,
                position,
                null,
                out var localPoint
            );

            var browserX = (localPoint.x - RectTransform.rect.min.x) / RectTransform.rect.width;
            var browserY =
                1 - (localPoint.y - RectTransform.rect.min.y) / RectTransform.rect.height;

            return new Vector2(browserX, browserY);
        }
    }
}
