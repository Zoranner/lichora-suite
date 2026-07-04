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
        private const int OutputBufferSize = 64 * 1024;

        private readonly object _SnapshotLock = new object();
        private readonly byte[] _OutputBuffer = new byte[OutputBufferSize];
        private readonly RectTransform _RectTransform;
        private readonly BrowserOverlaySettings _OverlaySettings;
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

        public BrowserOutputState(
            RectTransform rectTransform,
            int width,
            int height,
            BrowserOverlaySettings overlaySettings
        )
        {
            _RectTransform = rectTransform;
            _OverlaySettings = overlaySettings;
            _Canvas = rectTransform != null ? rectTransform.GetComponentInParent<Canvas>() : null;
            _Width = width;
            _Height = height;
        }

        public void Resize(int width, int height)
        {
            _Width = width;
            _Height = height;
            ClearOverlayPassMap();
        }

        public void ReadLatest(BrowserIpcOutputReader outputReader)
        {
            int written;
            try
            {
                written = outputReader.TryRead(_OutputBuffer);
            }
            catch (Exception exception)
            {
                LogInvalidOutput($"read failed: {exception.Message}");
                return;
            }

            if (written <= 0)
            {
                return;
            }

            if (
                !BrowserIpcOutputPayload.TryDecode(
                    _OutputBuffer,
                    written,
                    out var payload,
                    out var error
                )
            )
            {
                LogInvalidOutput(error);
                return;
            }

            Apply(payload);
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

        private void Apply(BrowserIpcOutputPayload payload)
        {
            switch (payload.Kind)
            {
                case BrowserIpcOutputPayloadKind.Caret:
                    if (payload.CaretVisible)
                    {
                        UpdatePendingCaret(payload.CaretX, payload.CaretY, payload.CaretHeight);
                    }
                    break;
                case BrowserIpcOutputPayloadKind.SurroundingText:
                    UpdateSurroundingText(
                        payload.Text,
                        payload.SelectionStart,
                        payload.SelectionEnd
                    );
                    break;
                case BrowserIpcOutputPayloadKind.ScriptResult:
                case BrowserIpcOutputPayloadKind.PageEvent:
                    break;
                case BrowserIpcOutputPayloadKind.OverlayPassMap:
                    UpdateOverlayPassMap(payload.OverlayPassMap);
                    break;
            }
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

        private void UpdateOverlayPassMap(BrowserOverlayPassMapPayload payload)
        {
            if (_OverlaySettings == null)
            {
                return;
            }

            if (!IsValidOverlayPassMapPayload(payload))
            {
                _OverlaySettings.ClearDynamicPassRects();
                return;
            }

            var passRects = BuildDynamicPassRects(payload);
            _OverlaySettings.SetDynamicPassRects(passRects);
        }

        public void ClearOverlayPassMap()
        {
            _OverlaySettings?.ClearDynamicPassRects();
        }

        private static PassRegion[] BuildDynamicPassRects(BrowserOverlayPassMapPayload payload)
        {
            var regions = payload.Regions ?? Array.Empty<BrowserOverlayPassRegionPayload>();
            var passRects = new PassRegion[regions.Length];
            var count = 0;

            foreach (var region in regions)
            {
                if (!TryCreateDynamicPassRect(payload, region, out var passRect))
                {
                    continue;
                }

                passRects[count++] = passRect;
            }

            if (count == passRects.Length)
            {
                return passRects;
            }

            if (count == 0)
            {
                return Array.Empty<PassRegion>();
            }

            var compactPassRects = new PassRegion[count];
            Array.Copy(passRects, compactPassRects, count);
            return compactPassRects;
        }

        private static bool TryCreateDynamicPassRect(
            BrowserOverlayPassMapPayload payload,
            BrowserOverlayPassRegionPayload region,
            out PassRegion passRect
        )
        {
            passRect = default;

            if (region.Disabled || region.Shape != 1)
            {
                return false;
            }

            if (
                !IsFinite(region.X)
                || !IsFinite(region.Y)
                || !IsFinite(region.Width)
                || !IsFinite(region.Height)
                || region.Width <= 0f
                || region.Height <= 0f
            )
            {
                return false;
            }

            var normalizedRect = new Rect(
                region.X / payload.ViewportWidth,
                region.Y / payload.ViewportHeight,
                region.Width / payload.ViewportWidth,
                region.Height / payload.ViewportHeight
            );
            passRect = new PassRegion(normalizedRect);
            return passRect.IsValid;
        }

        private static bool IsValidOverlayPassMapPayload(BrowserOverlayPassMapPayload payload)
        {
            return payload.Enabled
                && payload.ViewportWidth > 0
                && payload.ViewportHeight > 0
                && IsFinite(payload.DeviceScaleFactor)
                && payload.DeviceScaleFactor > 0f;
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

        private static bool IsFinite(float value)
        {
            return !float.IsNaN(value) && !float.IsInfinity(value);
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
    }
}
