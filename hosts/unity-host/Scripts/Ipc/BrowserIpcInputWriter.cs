using System;

namespace KimoTech.LichoraHost
{
    public sealed class BrowserIpcInputWriter : IDisposable
    {
        private readonly NativeIpcHandle _BrowserHandle;

        internal BrowserIpcInputWriter(NativeIpcHandle browserHandle)
        {
            _BrowserHandle =
                browserHandle ?? throw new ArgumentNullException(nameof(browserHandle));
        }

        public void SetMouseLatest(int x, int y, uint buttons, int deltaX, int deltaY, bool valid)
        {
            if (_BrowserHandle.IsClosed)
            {
                throw new ObjectDisposedException(nameof(BrowserIpcInputWriter));
            }

            BrowserIpcNative
                .SetBrowserInputMouseLatest(
                    _BrowserHandle.DangerousHandle,
                    x,
                    y,
                    buttons,
                    deltaX,
                    deltaY,
                    valid
                )
                .ThrowIfFailed();
        }

        public void PushEvent(NativeIpcInputEventKind kind, ulong sequence, byte[] payload)
        {
            if (_BrowserHandle.IsClosed)
            {
                throw new ObjectDisposedException(nameof(BrowserIpcInputWriter));
            }

            BrowserIpcNative
                .PushBrowserInputEvent(_BrowserHandle.DangerousHandle, kind, sequence, payload)
                .ThrowIfFailed();
        }

        public void PushMouseButton(
            ulong sequence,
            int x,
            int y,
            uint button,
            uint buttons,
            bool pressed,
            byte clickCount,
            uint modifiers
        )
        {
            ThrowIfClosed();
            BrowserIpcNative
                .PushBrowserInputEvent(
                    _BrowserHandle.DangerousHandle,
                    NativeIpcInputEventKind.MouseButton,
                    sequence,
                    BrowserIpcInputPayload.EncodeMouseButton(
                        x,
                        y,
                        button,
                        buttons,
                        pressed,
                        clickCount,
                        modifiers
                    )
                )
                .ThrowIfFailed();
        }

        public void PushMouseWheel(
            ulong sequence,
            int x,
            int y,
            int deltaX,
            int deltaY,
            uint modifiers
        )
        {
            ThrowIfClosed();
            BrowserIpcNative
                .PushBrowserInputEvent(
                    _BrowserHandle.DangerousHandle,
                    NativeIpcInputEventKind.MouseWheel,
                    sequence,
                    BrowserIpcInputPayload.EncodeMouseWheel(x, y, deltaX, deltaY, modifiers)
                )
                .ThrowIfFailed();
        }

        public void PushKeyboardKey(
            ulong sequence,
            bool pressed,
            uint keyCode,
            uint nativeKeyCode,
            uint modifiers
        )
        {
            ThrowIfClosed();
            BrowserIpcNative
                .PushBrowserInputEvent(
                    _BrowserHandle.DangerousHandle,
                    NativeIpcInputEventKind.Keyboard,
                    sequence,
                    BrowserIpcInputPayload.EncodeKeyboardKey(
                        pressed,
                        keyCode,
                        nativeKeyCode,
                        modifiers
                    )
                )
                .ThrowIfFailed();
        }

        public void PushKeyboardChar(ulong sequence, uint codePoint, uint modifiers)
        {
            ThrowIfClosed();
            BrowserIpcNative
                .PushBrowserInputEvent(
                    _BrowserHandle.DangerousHandle,
                    NativeIpcInputEventKind.Keyboard,
                    sequence,
                    BrowserIpcInputPayload.EncodeKeyboardChar(codePoint, modifiers)
                )
                .ThrowIfFailed();
        }

        public void PushImeComposition(
            ulong sequence,
            string text,
            int selectionStart,
            int selectionEnd
        )
        {
            ThrowIfClosed();
            BrowserIpcNative
                .PushBrowserInputEvent(
                    _BrowserHandle.DangerousHandle,
                    NativeIpcInputEventKind.Ime,
                    sequence,
                    BrowserIpcInputPayload.EncodeImeComposition(text, selectionStart, selectionEnd)
                )
                .ThrowIfFailed();
        }

        public void PushImeCommit(ulong sequence, string text)
        {
            ThrowIfClosed();
            BrowserIpcNative
                .PushBrowserInputEvent(
                    _BrowserHandle.DangerousHandle,
                    NativeIpcInputEventKind.Ime,
                    sequence,
                    BrowserIpcInputPayload.EncodeImeCommit(text)
                )
                .ThrowIfFailed();
        }

        public void PushImeCancel(ulong sequence)
        {
            ThrowIfClosed();
            BrowserIpcNative
                .PushBrowserInputEvent(
                    _BrowserHandle.DangerousHandle,
                    NativeIpcInputEventKind.Ime,
                    sequence,
                    BrowserIpcInputPayload.EncodeImeCancel()
                )
                .ThrowIfFailed();
        }

        public void PushImeDeleteSurroundingText(ulong sequence, int before, int after)
        {
            ThrowIfClosed();
            BrowserIpcNative
                .PushBrowserInputEvent(
                    _BrowserHandle.DangerousHandle,
                    NativeIpcInputEventKind.Ime,
                    sequence,
                    BrowserIpcInputPayload.EncodeImeDeleteSurroundingText(before, after)
                )
                .ThrowIfFailed();
        }

        public void PushScriptRequest(ulong sequence, ulong requestId, string script)
        {
            ThrowIfClosed();
            BrowserIpcNative
                .PushBrowserInputEvent(
                    _BrowserHandle.DangerousHandle,
                    NativeIpcInputEventKind.Script,
                    sequence,
                    BrowserIpcInputPayload.EncodeScriptRequest(requestId, script)
                )
                .ThrowIfFailed();
        }

        private void ThrowIfClosed()
        {
            if (_BrowserHandle.IsClosed)
            {
                throw new ObjectDisposedException(nameof(BrowserIpcInputWriter));
            }
        }

        public void Dispose()
        {
            _BrowserHandle.Dispose();
        }
    }
}
