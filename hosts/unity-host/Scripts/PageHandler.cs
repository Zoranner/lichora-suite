using System;
using System.Collections.Concurrent;
using Unity.Collections;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    public sealed class PageHandler : IImeInputSink, IBrowserFrameSource
    {
        private readonly BrowserIpcInputWriter _IpcInputWriter;
        private readonly BrowserIpcFrameReader _IpcFrameReader;
        private readonly BrowserOutputPump _OutputPump;
        private readonly ImeInputState _ImeInputState;
        private bool _HasMouseState;
        private MouseState _LastMouseState;
        private ulong _IpcInputSequence;
        private ulong _LastNativeFrameSequence;

        public PageHandler(
            string guid,
            string address,
            int width,
            int height,
            RectTransform rectTransform,
            BrowserIpcInputWriter ipcInputWriter,
            BrowserIpcFrameReader ipcFrameReader,
            BrowserIpcOutputReader ipcOutputReader
        )
        {
            if (ipcInputWriter == null)
            {
                throw new ArgumentNullException(nameof(ipcInputWriter));
            }

            if (ipcFrameReader == null)
            {
                throw new ArgumentNullException(nameof(ipcFrameReader));
            }

            if (ipcOutputReader == null)
            {
                throw new ArgumentNullException(nameof(ipcOutputReader));
            }

            GUID = guid;
            Address = address;
            Width = width;
            Height = height;
            NativeFrameWidth = width;
            NativeFrameHeight = height;
            _IpcInputWriter = ipcInputWriter;
            _IpcFrameReader = ipcFrameReader;
            _OutputPump = new BrowserOutputPump(ipcOutputReader, rectTransform, width, height);
            _ImeInputState = new ImeInputState(
                PushImeComposition,
                PushImeCommit,
                PushImeCancel,
                PushImeDeleteSurroundingText
            );
        }

        public string GUID { get; }
        public string Address { get; }
        public int Width { get; private set; }
        public int Height { get; private set; }
        public bool State => BrowserStatic.Instanced && BrowserStatic.Instance.IsIpcOpen;
        public bool IsReady => State;
        public int FrameWidth => NativeFrameWidth > 0 ? NativeFrameWidth : DataWidth;
        public int FrameHeight => NativeFrameHeight > 0 ? NativeFrameHeight : DataHeight;
        public int DataWidth => NativeFrameWidth;
        public int DataHeight => NativeFrameHeight;
        public int NativeFrameWidth { get; private set; }
        public int NativeFrameHeight { get; private set; }
        public int CaptureFrameVersion =>
            checked((int)Math.Min(_LastNativeFrameSequence, int.MaxValue));
        public ISurroundingTextSnapshotProvider SurroundingTextProvider =>
            _OutputPump.SurroundingTextProvider;

        public MouseState MouseState
        {
            get => _LastMouseState;
            set
            {
                if (_HasMouseState && IsSameMouseState(_LastMouseState, value))
                {
                    return;
                }

                SendIpcMouseInput(value);
                _LastMouseState = value;
                _HasMouseState = true;
            }
        }

        public void ProcessKeyboardEvents(ConcurrentQueue<KeyboardState> keyboardEvents)
        {
            if (!State)
            {
                return;
            }

            while (keyboardEvents.TryDequeue(out var keyboardState))
            {
                if (!keyboardState.HasEvent)
                {
                    continue;
                }

                switch (keyboardState.Type)
                {
                    case KeyEventType.KeyDown:
                        SendIpcKeyboardKey(keyboardState, true);
                        break;
                    case KeyEventType.KeyUp:
                        SendIpcKeyboardKey(keyboardState, false);
                        break;
                    case KeyEventType.Char:
                        SendIpcKeyboardChar(keyboardState);
                        break;
                }
            }
        }

        public bool UpdateIme(string compositionString, string inputString)
        {
            return State && _ImeInputState.UpdateComposition(compositionString, inputString);
        }

        public void ApplyMainThreadUpdates()
        {
            if (!State)
            {
                return;
            }

            _OutputPump.ReadLatestAndApplyMainThreadUpdates();
        }

        public void ResetIme()
        {
            _ImeInputState.ResetState();
        }

        public void CommitImeText(string text)
        {
            _ImeInputState.CommitText(text);
        }

        public void SetImeComposition(string text, int selectionStart, int selectionEnd)
        {
            _ImeInputState.SetComposition(text, selectionStart, selectionEnd);
        }

        public void CancelImeComposition()
        {
            _ImeInputState.CancelComposition();
        }

        public void DeleteImeSurroundingText(int before, int after)
        {
            _ImeInputState.DeleteSurroundingText(before, after);
        }

        public void ExecuteScript(string script)
        {
            if (!State)
            {
                Debug.LogWarning($"Page {GUID} is not active, cannot execute script.");
                return;
            }

            if (string.IsNullOrEmpty(script))
            {
                Debug.LogWarning("Script content is empty.");
                return;
            }

            var requestId = NextIpcInputSequence();
            _IpcInputWriter.PushScriptRequest(requestId, requestId, script);
        }

        public void Resize(int width, int height)
        {
            if (width == Width && height == Height)
            {
                return;
            }

            Width = width;
            Height = height;
            _OutputPump.Resize(width, height);
        }

        private bool TryCopyCaptureTo(NativeArray<byte> dest)
        {
            if (!State)
            {
                return false;
            }

            if (
                !_IpcFrameReader.TryCopyLatest(
                    dest,
                    ref _LastNativeFrameSequence,
                    out var width,
                    out var height,
                    out var sequence,
                    out var written
                )
            )
            {
                UpdateNativeFrameSize(width, height, written);
                return false;
            }

            if (
                !IsValidFrameSize(width, height)
                || written != dest.Length
                || written != width * height * 4
            )
            {
                UpdateNativeFrameSize(width, height, written);
                return false;
            }

            _LastNativeFrameSequence = sequence;
            NativeFrameWidth = width;
            NativeFrameHeight = height;
            return true;
        }

        public bool TryCopyFrameTo(NativeArray<byte> destination)
        {
            return TryCopyCaptureTo(destination);
        }

        public void Destroy()
        {
            _IpcInputWriter.Dispose();
            _IpcFrameReader.Dispose();
            _OutputPump.Dispose();
        }

        public void DestroyForBrowserRestart()
        {
            Destroy();
        }

        private void SendIpcMouseInput(MouseState mouseState)
        {
            var mouseX = GetMouseX(mouseState);
            var mouseY = GetMouseY(mouseState);
            var buttons = GetMouseButtons(mouseState);
            var deltaX = GetMouseDeltaX(mouseState);
            var deltaY = GetMouseDeltaY(mouseState);

            _IpcInputWriter.SetMouseLatest(
                mouseX,
                mouseY,
                buttons,
                deltaX,
                deltaY,
                mouseState.Effective
            );

            if (_HasMouseState)
            {
                SendIpcMouseButtonEvents(mouseState, mouseX, mouseY, buttons);
                SendIpcMouseWheelEvent(mouseState, mouseX, mouseY);
            }
        }

        private void SendIpcMouseButtonEvents(
            MouseState mouseState,
            int mouseX,
            int mouseY,
            uint buttons
        )
        {
            if (!_LastMouseState.LeftButton && mouseState.LeftButton)
            {
                PushIpcMouseButtonEvent(1, true, mouseX, mouseY, buttons);
            }

            if (_LastMouseState.LeftButton && !mouseState.LeftButton)
            {
                PushIpcMouseButtonEvent(1, false, mouseX, mouseY, buttons);
            }

            if (!_LastMouseState.RightButton && mouseState.RightButton)
            {
                PushIpcMouseButtonEvent(2, true, mouseX, mouseY, buttons);
            }

            if (_LastMouseState.RightButton && !mouseState.RightButton)
            {
                PushIpcMouseButtonEvent(2, false, mouseX, mouseY, buttons);
            }

            if (!_LastMouseState.MiddleButton && mouseState.MiddleButton)
            {
                PushIpcMouseButtonEvent(3, true, mouseX, mouseY, buttons);
            }

            if (_LastMouseState.MiddleButton && !mouseState.MiddleButton)
            {
                PushIpcMouseButtonEvent(3, false, mouseX, mouseY, buttons);
            }
        }

        private void SendIpcMouseWheelEvent(MouseState mouseState, int mouseX, int mouseY)
        {
            if (mouseState.ScrollDelta.sqrMagnitude > 0.001f)
            {
                _IpcInputWriter.PushMouseWheel(
                    NextIpcInputSequence(),
                    mouseX,
                    mouseY,
                    Mathf.RoundToInt(mouseState.ScrollDelta.x * 40),
                    Mathf.RoundToInt(mouseState.ScrollDelta.y * 40),
                    0
                );
            }
        }

        private void PushIpcMouseButtonEvent(
            byte button,
            bool pressed,
            int mouseX,
            int mouseY,
            uint buttons
        )
        {
            _IpcInputWriter.PushMouseButton(
                NextIpcInputSequence(),
                mouseX,
                mouseY,
                button,
                buttons,
                pressed,
                1,
                buttons
            );
        }

        private void SendIpcKeyboardKey(KeyboardState keyboardState, bool pressed)
        {
            _IpcInputWriter.PushKeyboardKey(
                NextIpcInputSequence(),
                pressed,
                checked((uint)keyboardState.WindowsKeyCode),
                checked((uint)keyboardState.NativeKeyCode),
                (uint)keyboardState.Modifiers
            );
        }

        private void SendIpcKeyboardChar(KeyboardState keyboardState)
        {
            _IpcInputWriter.PushKeyboardChar(
                NextIpcInputSequence(),
                keyboardState.Character,
                (uint)keyboardState.Modifiers
            );
        }

        private void PushImeComposition(string text, int selectionStart, int selectionEnd)
        {
            _IpcInputWriter.PushImeComposition(
                NextIpcInputSequence(),
                text,
                selectionStart,
                selectionEnd
            );
        }

        private void PushImeCommit(string text)
        {
            _IpcInputWriter.PushImeCommit(NextIpcInputSequence(), text);
        }

        private void PushImeCancel()
        {
            _IpcInputWriter.PushImeCancel(NextIpcInputSequence());
        }

        private void PushImeDeleteSurroundingText(int before, int after)
        {
            _IpcInputWriter.PushImeDeleteSurroundingText(NextIpcInputSequence(), before, after);
        }

        private ulong NextIpcInputSequence()
        {
            _IpcInputSequence++;
            return _IpcInputSequence;
        }

        private void UpdateNativeFrameSize(int width, int height, int written)
        {
            if (IsValidFrameSize(width, height) && written == width * height * 4)
            {
                NativeFrameWidth = width;
                NativeFrameHeight = height;
                return;
            }

            if (written == Width * Height * 4)
            {
                NativeFrameWidth = Width;
                NativeFrameHeight = Height;
            }
        }

        private static bool IsValidFrameSize(int width, int height)
        {
            return width > 0 && height > 0;
        }

        private static bool IsSameMouseState(MouseState left, MouseState right)
        {
            return left.Effective == right.Effective
                && left.Position == right.Position
                && left.Delta == right.Delta
                && left.LeftButton == right.LeftButton
                && left.RightButton == right.RightButton
                && left.MiddleButton == right.MiddleButton
                && left.ScrollDelta == right.ScrollDelta;
        }

        private int GetMouseX(MouseState mouseState)
        {
            return Mathf.Clamp(Mathf.RoundToInt(mouseState.Position.x * Width), 0, Width);
        }

        private int GetMouseY(MouseState mouseState)
        {
            return Mathf.Clamp(Mathf.RoundToInt(mouseState.Position.y * Height), 0, Height);
        }

        private int GetMouseDeltaX(MouseState mouseState)
        {
            return Mathf.RoundToInt(mouseState.Delta.x * Width);
        }

        private int GetMouseDeltaY(MouseState mouseState)
        {
            return Mathf.RoundToInt(mouseState.Delta.y * Height);
        }

        private static uint GetMouseButtons(MouseState mouseState)
        {
            uint buttons = 0;
            if (mouseState.LeftButton)
            {
                buttons |= 0x01;
            }

            if (mouseState.RightButton)
            {
                buttons |= 0x02;
            }

            if (mouseState.MiddleButton)
            {
                buttons |= 0x04;
            }

            return buttons;
        }
    }
}
