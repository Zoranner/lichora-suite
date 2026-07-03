using System;
using System.Collections.Generic;
using System.Text;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    public sealed class LinuxNativeImeModule : IDisposable
    {
        private const uint X11_SHIFT = 1;
        private const uint X11_CTRL = 4;
        private const uint X11_ALT = 8;
        private readonly NativeImeBridge _Bridge;
        private readonly IImeInputSink _ImeInputSink;
        private readonly HashSet<uint> _ConsumedKeys = new HashSet<uint>();
        private Vector2 _LastCursorPosition = new Vector2(float.MinValue, float.MinValue);
        private ISurroundingTextSnapshotProvider _SurroundingTextProvider;
        private int _LastSurroundingTextVersion;
        private string _LastSurroundingText = "";
        private int _LastSurroundingCursor = -1;
        private int _LastSurroundingAnchor = -1;
        private NativeImeContentType? _LastContentType;
        private bool _HasFocus;

        public bool IsAvailable => _Bridge != null && _Bridge.IsAvailable;

        public static LinuxNativeImeModule TryCreate(IImeInputSink imeInputSink)
        {
            var bridge = NativeImeBridge.TryCreate();
            if (bridge == null)
            {
                return null;
            }

            Debug.Log(
                $"[LinuxNativeIme] backend={bridge.BackendKind}, caps={FormatCapabilities(bridge.Capabilities)}"
            );
            return new LinuxNativeImeModule(bridge, imeInputSink);
        }

        private LinuxNativeImeModule(NativeImeBridge bridge, IImeInputSink imeInputSink)
        {
            _Bridge = bridge;
            _ImeInputSink = imeInputSink ?? throw new ArgumentNullException(nameof(imeInputSink));
        }

        public void SetSurroundingTextProvider(
            ISurroundingTextSnapshotProvider surroundingTextProvider
        )
        {
            _SurroundingTextProvider = surroundingTextProvider;
            _LastSurroundingTextVersion = 0;
            _LastSurroundingText = "";
            _LastSurroundingCursor = -1;
            _LastSurroundingAnchor = -1;
            _LastContentType = null;
        }

        public void FocusIn()
        {
            if (!IsAvailable || _HasFocus)
            {
                return;
            }

            _HasFocus = true;
            Input.imeCompositionMode = IMECompositionMode.Off;
            _Bridge.FocusIn();
            Debug.Log(
                $"[LinuxNativeIme] focus in: backend={_Bridge.BackendKind}, caps={FormatCapabilities(_Bridge.Capabilities)}"
            );
            _Bridge.Reset();
            _LastContentType = null;
            UpdateContentTypeFromSnapshotOrDefault();
        }

        public void FocusOut()
        {
            if (!IsAvailable || !_HasFocus)
            {
                return;
            }

            _HasFocus = false;
            _Bridge.FocusOut();
            _Bridge.Reset();
            Input.imeCompositionMode = IMECompositionMode.Auto;
        }

        public bool ProcessKeyEvent(Event currentEvent, KeyEventType eventType)
        {
            if (!IsAvailable || !_HasFocus)
            {
                return false;
            }

            var keyval = KeyCodeMapper.ToX11KeySym(currentEvent.keyCode, currentEvent.character);
            if (keyval == 0)
            {
                return false;
            }

            var state = ToX11ModifierState(currentEvent);
            if (eventType == KeyEventType.KeyUp && _ConsumedKeys.Remove(keyval))
            {
                _Bridge.ProcessKeyEvent(keyval, 0, state, true);
                return true;
            }

            var handled = _Bridge.ProcessKeyEvent(
                keyval,
                0,
                state,
                eventType == KeyEventType.KeyUp
            );
            if (handled && eventType == KeyEventType.KeyDown)
            {
                _ConsumedKeys.Add(keyval);
            }

            return handled;
        }

        public void Update()
        {
            if (!IsAvailable || !_HasFocus)
            {
                return;
            }

            UpdateCursorRect();
            UpdateSurroundingText();
            DrainEvents();
        }

        public void Dispose()
        {
            _Bridge?.Dispose();
        }

        private void UpdateCursorRect()
        {
            var cursorPosition = Input.compositionCursorPos;
            if (cursorPosition == _LastCursorPosition)
            {
                return;
            }

            _LastCursorPosition = cursorPosition;
            _Bridge.SetCursorRect(
                Mathf.RoundToInt(cursorPosition.x),
                Mathf.RoundToInt(cursorPosition.y),
                1,
                20
            );
            Debug.Log(
                $"[LinuxNativeIme] cursor rect sent: x={Mathf.RoundToInt(cursorPosition.x)}, y={Mathf.RoundToInt(cursorPosition.y)}, width=1, height=20"
            );
        }

        private void UpdateSurroundingText()
        {
            if (
                !IsAvailable
                || _SurroundingTextProvider == null
                || !_SurroundingTextProvider.TryGetSnapshot(out var snapshot)
                || snapshot.Version == _LastSurroundingTextVersion
            )
            {
                return;
            }

            _LastSurroundingTextVersion = snapshot.Version;
            UpdateContentType(ParseContentType(snapshot.ContentType));

            if (!_Bridge.HasCapability(NativeImeCapabilities.SurroundingText))
            {
                return;
            }

            if (
                snapshot.Text == _LastSurroundingText
                && snapshot.Cursor == _LastSurroundingCursor
                && snapshot.Anchor == _LastSurroundingAnchor
            )
            {
                return;
            }

            _LastSurroundingText = snapshot.Text;
            _LastSurroundingCursor = snapshot.Cursor;
            _LastSurroundingAnchor = snapshot.Anchor;
            _Bridge.SetSurroundingText(snapshot.Text, snapshot.Cursor, snapshot.Anchor);
            Debug.Log(
                $"[LinuxNativeIme] surrounding text sent: version={snapshot.Version}, utf8Bytes={Utf8ByteLength(snapshot.Text)}, cursor={snapshot.Cursor}, anchor={snapshot.Anchor}, contentType={ParseContentType(snapshot.ContentType)}"
            );
        }

        private void UpdateContentTypeFromSnapshotOrDefault()
        {
            if (
                _SurroundingTextProvider != null
                && _SurroundingTextProvider.TryGetSnapshot(out var snapshot)
            )
            {
                UpdateContentType(ParseContentType(snapshot.ContentType));
                return;
            }

            UpdateContentType(NativeImeContentType.Normal);
        }

        private void UpdateContentType(NativeImeContentType contentType)
        {
            if (
                !IsAvailable
                || !_Bridge.HasCapability(NativeImeCapabilities.ContentType)
                || _LastContentType == contentType
            )
            {
                return;
            }

            _LastContentType = contentType;
            _Bridge.SetContentType(contentType);
            Debug.Log($"[LinuxNativeIme] content type sent: contentType={contentType}");
        }

        private static NativeImeContentType ParseContentType(byte contentType)
        {
            return contentType <= (byte)NativeImeContentType.Email
                ? (NativeImeContentType)contentType
                : NativeImeContentType.Normal;
        }

        private void DrainEvents()
        {
            while (_Bridge.TryPollEvent(out var data))
            {
                var eventType = (NativeImeEventType)data.EventType;
                switch (eventType)
                {
                    case NativeImeEventType.Preedit:
                        var preeditInfo = NativeImeBridge.AnalyzeTextBuffer(data.Text);
                        var preeditText = ReadText(data.Text, preeditInfo);
                        Debug.Log(
                            $"[LinuxNativeIme] event=Preedit utf8Bytes={preeditInfo.ByteLength}, chars={preeditText.Length}, cursorBegin={data.CursorBegin}, cursorEnd={data.CursorEnd}"
                        );
                        LogTextTruncationWarning(eventType, preeditInfo, data);
                        _ImeInputSink.SetImeComposition(
                            preeditText,
                            data.CursorBegin,
                            data.CursorEnd
                        );
                        break;
                    case NativeImeEventType.PreeditEnd:
                        Debug.Log("[LinuxNativeIme] event=PreeditEnd");
                        _ImeInputSink.CancelImeComposition();
                        break;
                    case NativeImeEventType.Commit:
                        var commitInfo = NativeImeBridge.AnalyzeTextBuffer(data.Text);
                        var commitText = ReadText(data.Text, commitInfo);
                        Debug.Log(
                            $"[LinuxNativeIme] event=Commit utf8Bytes={commitInfo.ByteLength}, chars={commitText.Length}"
                        );
                        LogTextTruncationWarning(eventType, commitInfo, data);
                        _ImeInputSink.CommitImeText(commitText);
                        break;
                    case NativeImeEventType.DeleteSurroundingText:
                        Debug.Log(
                            $"[LinuxNativeIme] event=DeleteSurroundingText offset={data.Param1}, length={data.Param2}"
                        );
                        _ImeInputSink.DeleteImeSurroundingText(data.Param1, data.Param2);
                        break;
                    case NativeImeEventType.ForwardKey:
                    case NativeImeEventType.None:
                    default:
                        break;
                }
            }
        }

        private static string ReadText(byte[] bytes, NativeImeTextBufferInfo info)
        {
            if (bytes == null || info.ByteLength == 0)
            {
                return "";
            }

            return Encoding.UTF8.GetString(bytes, 0, info.ByteLength);
        }

        private static void LogTextTruncationWarning(
            NativeImeEventType eventType,
            NativeImeTextBufferInfo info,
            NativeImeEventData data
        )
        {
            if (!info.IsTruncationSuspected)
            {
                return;
            }

            Debug.LogWarning(
                $"[LinuxNativeIme] native text buffer may be truncated: event={eventType}, utf8Bytes={info.ByteLength}, bufferBytes={NativeImeBridge.TEXT_BUFFER_SIZE}, hasNullTerminator={info.HasNullTerminator}, cursorBegin={data.CursorBegin}, cursorEnd={data.CursorEnd}, param1={data.Param1}, param2={data.Param2}"
            );
        }

        private static int Utf8ByteLength(string text)
        {
            return string.IsNullOrEmpty(text) ? 0 : Encoding.UTF8.GetByteCount(text);
        }

        private static uint ToX11ModifierState(Event currentEvent)
        {
            uint state = 0;
            if (currentEvent.shift)
            {
                state |= X11_SHIFT;
            }

            if (currentEvent.control)
            {
                state |= X11_CTRL;
            }

            if (currentEvent.alt)
            {
                state |= X11_ALT;
            }

            return state;
        }

        private static string FormatCapabilities(NativeImeCapabilities capabilities)
        {
            return capabilities == NativeImeCapabilities.None ? "None" : capabilities.ToString();
        }
    }
}
