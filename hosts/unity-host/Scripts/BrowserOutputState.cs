using System;
using System.Threading;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    public interface ISurroundingTextSnapshotProvider
    {
        bool TryGetSnapshot(out SurroundingTextSnapshot snapshot);
    }

    public readonly struct SurroundingTextSnapshot
    {
        public SurroundingTextSnapshot(
            string text,
            int cursor,
            int anchor,
            byte contentType,
            int version
        )
        {
            Text = text;
            Cursor = cursor;
            Anchor = anchor;
            ContentType = contentType;
            Version = version;
        }

        public string Text { get; }
        public int Cursor { get; }
        public int Anchor { get; }
        public byte ContentType { get; }
        public int Version { get; }
    }

    public sealed class BrowserOutputState : ISurroundingTextSnapshotProvider
    {
        private const byte NormalContentType = 0;
        private readonly object _SnapshotLock = new object();
        private readonly RectTransform _RectTransform;
        private Canvas _Canvas;
        private int _Width;
        private int _Height;
        private int _PendingCaretX = -1;
        private int _PendingCaretY = -1;
        private int _PendingCaretHeight;
        private int _PendingCaretVersion;
        private int _AppliedCaretVersion;
        private string _Text = "";
        private int _Cursor = -1;
        private int _Anchor = -1;
        private int _Version;
        private bool _HasLoggedInvalidOutput;

        public BrowserOutputState(RectTransform rectTransform, int width, int height)
        {
            _RectTransform = rectTransform;
            _Canvas = rectTransform != null ? rectTransform.GetComponentInParent<Canvas>() : null;
            _Width = width;
            _Height = height;
        }

        public void Resize(int width, int height)
        {
            _Width = width;
            _Height = height;
        }

        internal void ApplyCaret(int x, int y, int height, bool visible)
        {
            if (visible)
            {
                UpdatePendingCaret(x, y, height);
            }
        }

        internal void ApplySurroundingText(string text, int cursor, int anchor)
        {
            UpdateSurroundingText(text, cursor, anchor);
        }

        public bool TryGetSnapshot(out SurroundingTextSnapshot snapshot)
        {
            lock (_SnapshotLock)
            {
                if (_Version <= 0)
                {
                    snapshot = new SurroundingTextSnapshot("", -1, -1, NormalContentType, 0);
                    return false;
                }

                snapshot = new SurroundingTextSnapshot(
                    _Text,
                    _Cursor,
                    _Anchor,
                    NormalContentType,
                    _Version
                );
                return true;
            }
        }

        public void ApplyPendingImePosition()
        {
            var pendingVersion = Volatile.Read(ref _PendingCaretVersion);
            if (pendingVersion == _AppliedCaretVersion)
            {
                return;
            }

            _AppliedCaretVersion = pendingVersion;
            UpdateImePosition(_PendingCaretX, _PendingCaretY, _PendingCaretHeight);
        }

        private void UpdatePendingCaret(int x, int y, int height)
        {
            if (_PendingCaretX == x && _PendingCaretY == y && _PendingCaretHeight == height)
            {
                return;
            }

            _PendingCaretX = x;
            _PendingCaretY = y;
            _PendingCaretHeight = height;
            Interlocked.Increment(ref _PendingCaretVersion);
        }

        private void UpdateImePosition(int browserX, int browserY, int browserHeight)
        {
            if (_RectTransform == null || _Width <= 0 || _Height <= 0)
            {
                return;
            }

            var adjustedBrowserY = browserY + Math.Max(6, browserHeight + 2);
            var normalizedX = (float)browserX / _Width;
            var normalizedY = (float)adjustedBrowserY / _Height;
            var rect = _RectTransform.rect;
            var localX = rect.xMin + normalizedX * rect.width;
            var localY = rect.yMax - normalizedY * rect.height;
            var worldPoint = _RectTransform.TransformPoint(new Vector3(localX, localY, 0));

            if (
                _Canvas != null
                && _Canvas.renderMode != RenderMode.ScreenSpaceOverlay
                && _Canvas.worldCamera != null
            )
            {
                var screenPoint = _Canvas.worldCamera.WorldToScreenPoint(worldPoint);
                Input.compositionCursorPos = new Vector2(
                    screenPoint.x,
                    Screen.height - screenPoint.y
                );
            }
            else
            {
                Input.compositionCursorPos = new Vector2(
                    worldPoint.x,
                    Screen.height - worldPoint.y
                );
            }
        }

        private void LogInvalidOutput(string message)
        {
            if (_HasLoggedInvalidOutput)
            {
                return;
            }

            _HasLoggedInvalidOutput = true;
            Debug.LogWarning($"Browser IPC output ignored: {message}");
        }

        private void UpdateSurroundingText(string text, int cursor, int anchor)
        {
            if (text == null)
            {
                text = "";
            }
            if (cursor < 0 || anchor < 0 || cursor > text.Length || anchor > text.Length)
            {
                LogInvalidOutput(
                    $"invalid surrounding text cursor={cursor}, anchor={anchor}, textLength={text.Length}"
                );
                return;
            }

            lock (_SnapshotLock)
            {
                if (_Text == text && _Cursor == cursor && _Anchor == anchor)
                {
                    return;
                }

                _Text = text;
                _Cursor = cursor;
                _Anchor = anchor;
                Interlocked.Increment(ref _Version);
            }
        }
    }
}
