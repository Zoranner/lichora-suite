using System;
using System.Collections.Concurrent;
using System.Runtime.InteropServices;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    public sealed class BrowserFocusController : IDisposable
    {
        private readonly ConcurrentQueue<KeyboardState> _KeyboardEvents;
        private bool _PreviousHasFocus;
        private IImeInputSink _ImeInputSink;
        private LinuxNativeImeModule _LinuxNativeImeModule;

        public BrowserFocusController(ConcurrentQueue<KeyboardState> keyboardEvents)
        {
            _KeyboardEvents =
                keyboardEvents ?? throw new ArgumentNullException(nameof(keyboardEvents));
        }

        public bool HasFocus { get; private set; }
        public bool ImeActive { get; private set; }

        private static bool IsLinux => RuntimeInformation.IsOSPlatform(OSPlatform.Linux);

        public void BindImeInputSink(
            IImeInputSink imeInputSink,
            ISurroundingTextSnapshotProvider surroundingTextProvider
        )
        {
            UnbindImeInputSink();

            _ImeInputSink = imeInputSink;
            if (IsLinux && _ImeInputSink != null)
            {
                _LinuxNativeImeModule = LinuxNativeImeModule.TryCreate(_ImeInputSink);
                _LinuxNativeImeModule?.SetSurroundingTextProvider(surroundingTextProvider);
            }

            if (HasFocus)
            {
                FocusIn();
            }
        }

        public void UnbindImeInputSink()
        {
            _LinuxNativeImeModule?.Dispose();
            _LinuxNativeImeModule = null;
            _ImeInputSink = null;
            ImeActive = false;
            _PreviousHasFocus = false;
        }

        public void FocusIn()
        {
            HasFocus = true;

            if (_LinuxNativeImeModule != null && _LinuxNativeImeModule.IsAvailable)
            {
                _LinuxNativeImeModule.FocusIn();
                ImeActive = false;
                return;
            }

            Input.imeCompositionMode = IMECompositionMode.On;
        }

        public void FocusOut()
        {
            HasFocus = false;
            ImeActive = false;
            _PreviousHasFocus = false;
            _LinuxNativeImeModule?.FocusOut();
            Input.imeCompositionMode = IMECompositionMode.Auto;
        }

        public void Update(PageHandler handler, string compositionString, string inputString)
        {
            _LinuxNativeImeModule?.Update();

            if (handler == null || !handler.State)
            {
                return;
            }

            UpdateKeyboardState(handler);
            UpdateImeState(handler, compositionString, inputString);
        }

        public bool ProcessKeyEvent(Event currentEvent, KeyEventType eventType)
        {
            return _LinuxNativeImeModule != null
                && _LinuxNativeImeModule.ProcessKeyEvent(currentEvent, eventType);
        }

        public void CommitImeText(string text)
        {
            _ImeInputSink?.CommitImeText(text);
        }

        public void SetUnityImeActive(bool active)
        {
            ImeActive = active;
        }

        public void Dispose()
        {
            UnbindImeInputSink();
        }

        private void UpdateKeyboardState(PageHandler handler)
        {
            if (!HasFocus)
            {
                return;
            }

            handler.ProcessKeyboardEvents(_KeyboardEvents);
        }

        private void UpdateImeState(
            PageHandler handler,
            string compositionString,
            string inputString
        )
        {
            if (_LinuxNativeImeModule != null && _LinuxNativeImeModule.IsAvailable)
            {
                ImeActive = false;
                return;
            }

            if (HasFocus && !_PreviousHasFocus)
            {
                handler.ResetIme();
            }

            _PreviousHasFocus = HasFocus;

            if (!HasFocus)
            {
                return;
            }

            ImeActive = handler.UpdateIme(compositionString, inputString);
        }
    }
}
