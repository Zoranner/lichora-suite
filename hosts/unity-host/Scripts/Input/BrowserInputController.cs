using System.Collections.Concurrent;
using UnityEngine;
using UnityEngine.EventSystems;

namespace KimoTech.LichoraHost
{
    [DisallowMultipleComponent]
    [RequireComponent(typeof(RectTransform))]
    public class BrowserInputController
        : MonoBehaviour,
            IPointerEnterHandler,
            IPointerExitHandler,
            IPointerMoveHandler,
            IPointerDownHandler,
            IPointerUpHandler,
            IScrollHandler,
            ICanvasRaycastFilter
    {
        protected MouseState _MouseState;
        protected readonly ConcurrentQueue<KeyboardState> _KeyboardEvents =
            new ConcurrentQueue<KeyboardState>();
        protected string _CompositionString = "";
        protected string _InputString = "";
        private BrowserFocusController _FocusController;
        private BrowserPointerInputSource _PointerInputSource;
        private BrowserCoordinateMapper _CoordinateMapper;
        private PassHitFilter _PassHitFilter;
        private bool _BrowserPointerCaptured;

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

        public MouseState MouseState => _MouseState;
        public string CompositionString => _CompositionString;
        public string InputString => _InputString;

        public BrowserOverlaySettings OverlaySettings
        {
            get => HitFilter.Settings;
            set => HitFilter.Settings = value;
        }

        public bool IsRaycastLocationValid(Vector2 screenPosition, Camera eventCamera)
        {
            if (_BrowserPointerCaptured)
            {
                return true;
            }

            var isBrowserHit = HitFilter.IsBrowserHit(screenPosition, eventCamera);
            if (!isBrowserHit && IsMouseButtonDown())
            {
                _FocusController?.FocusOut();
                PointerInputSource.Exit();
                SyncMouseState();
            }

            return isBrowserHit;
        }

        public void OnPointerEnter(PointerEventData eventData)
        {
            PointerInputSource.Enter();
            SyncMouseState();
        }

        public void OnPointerExit(PointerEventData eventData)
        {
            if (_BrowserPointerCaptured)
            {
                return;
            }

            PointerInputSource.Exit();
            SyncMouseState();
        }

        public void OnPointerMove(PointerEventData eventData)
        {
            if (!ShouldHandlePointerEvent(eventData, allowCapturedPointer: true))
            {
                return;
            }

            PointerInputSource.Move(eventData);
            SyncMouseState();
        }

        public void OnPointerDown(PointerEventData eventData)
        {
            if (!ShouldHandlePointerEvent(eventData, allowCapturedPointer: false))
            {
                return;
            }

            _BrowserPointerCaptured = true;
            FocusController.FocusIn();

            PointerInputSource.Press(eventData);
            SyncMouseState();
        }

        public void OnPointerUp(PointerEventData eventData)
        {
            if (!ShouldHandlePointerEvent(eventData, allowCapturedPointer: true))
            {
                return;
            }

            PointerInputSource.Release(eventData);
            _BrowserPointerCaptured = false;
            SyncMouseState();
        }

        public void OnScroll(PointerEventData eventData)
        {
            if (!ShouldHandlePointerEvent(eventData, allowCapturedPointer: false))
            {
                return;
            }

            PointerInputSource.Scroll(eventData);
            SyncMouseState();
        }

        public void ResetFrameDeltas()
        {
            PointerInputSource.ResetFrameDeltas();
            SyncMouseState();
        }

        public void ResetPointerState()
        {
            _BrowserPointerCaptured = false;
            PointerInputSource.Reset();
            SyncMouseState();
        }

        protected void ResetScrollDelta()
        {
            ResetFrameDeltas();
        }

        protected virtual void OnGUI()
        {
            if (_FocusController == null || !_FocusController.HasFocus)
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

            // 实时更新 IME 激活状态，避免依赖 LateUpdate 中的延迟更新。
            // 当 compositionString 非空时，说明正在使用输入法进行组合输入（如中文拼音）。
            FocusController.SetUnityImeActive(!string.IsNullOrEmpty(_CompositionString));

            // 处理剪贴板粘贴（Ctrl+V）。
            if (currentEvent.type == EventType.ValidateCommand)
            {
                if (currentEvent.commandName == "Paste")
                {
                    currentEvent.Use();
                }
            }
            else if (currentEvent.type == EventType.ExecuteCommand)
            {
                if (currentEvent.commandName == "Paste")
                {
                    // 从 Unity 剪贴板获取文本并通过 IME 模块发送。
                    var clipboardText = GUIUtility.systemCopyBuffer;
                    if (!string.IsNullOrEmpty(clipboardText))
                    {
                        FocusController.CommitImeText(clipboardText);
                    }

                    currentEvent.Use();
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
            if (FocusController.ProcessKeyEvent(currentEvent, eventType))
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

            // 如果没有有效的按键也没有有效的字符，跳过。
            if (!hasValidKey && !hasValidChar)
            {
                return;
            }

            // 发送 KeyDown/KeyUp 事件（仅当有有效按键时）。
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
            // 1. 必须是 KeyDown 事件（不处理 KeyUp 的字符）。
            // 2. 必须有有效字符。
            // 3. 当前没有 IME 组合正在进行（compositionString 为空）。
            // 4. IME 输入状态未处于活跃状态（刚提交/取消的冷却期）。
            // 5. 字符必须是 ASCII 字符（< 128）。
            //
            // 关键修复：Unity IME 系统中，非 ASCII 字符（如中文、日文等）只会在
            // 选择候选词后出现在 inputString 中。这些字符会在 LateUpdate 中由
            // IImeInputSink.CommitImeText 统一处理，不应该在 OnGUI 中发送（否则会重复）。
            //
            // ASCII 字符（英文字母、数字、标点等）在普通英文输入时不经过 IME 组合，
            // 需要在 OnGUI 中发送；在 IME 拼音输入时会被 compositionString 或焦点控制器拦截。
            var isComposingNow = !string.IsNullOrEmpty(_CompositionString);
            var isAsciiChar = currentEvent.character < 128;

            var shouldSendChar =
                eventType == KeyEventType.KeyDown
                && hasValidChar
                && !isComposingNow
                && !FocusController.ImeActive
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

        private static bool IsMouseButtonDown()
        {
            return Input.GetMouseButtonDown(0)
                || Input.GetMouseButtonDown(1)
                || Input.GetMouseButtonDown(2);
        }

        public void ReleaseFocus()
        {
            FocusController.FocusOut();
        }

        public BrowserFocusController FocusController
        {
            get
            {
                if (_FocusController == null)
                {
                    _FocusController = new BrowserFocusController(_KeyboardEvents);
                }

                return _FocusController;
            }
        }

        public void DisposeFocusController()
        {
            _FocusController?.Dispose();
            _FocusController = null;
        }

        private BrowserPointerInputSource PointerInputSource
        {
            get
            {
                if (_PointerInputSource == null)
                {
                    _PointerInputSource = new BrowserPointerInputSource(CoordinateMapper);
                }

                return _PointerInputSource;
            }
        }

        private BrowserCoordinateMapper CoordinateMapper
        {
            get
            {
                if (_CoordinateMapper == null)
                {
                    _CoordinateMapper = new BrowserCoordinateMapper(RectTransform);
                }

                return _CoordinateMapper;
            }
        }

        private PassHitFilter HitFilter
        {
            get
            {
                if (_PassHitFilter == null)
                {
                    _PassHitFilter = new PassHitFilter(CoordinateMapper, null);
                }

                return _PassHitFilter;
            }
        }

        private bool ShouldHandlePointerEvent(PointerEventData eventData, bool allowCapturedPointer)
        {
            if (allowCapturedPointer && _BrowserPointerCaptured)
            {
                return true;
            }

            return HitFilter.IsBrowserHit(
                eventData.position,
                eventData.pressEventCamera != null
                    ? eventData.pressEventCamera
                    : eventData.enterEventCamera
            );
        }

        private void SyncMouseState()
        {
            _MouseState = PointerInputSource.State;
        }
    }
}
