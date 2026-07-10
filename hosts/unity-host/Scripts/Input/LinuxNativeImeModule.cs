using System;
using System.Collections.Generic;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    public sealed class LinuxNativeImeModule : IDisposable
    {
        private const uint X11_SHIFT = 1;
        private const uint X11_CTRL = 4;
        private const uint X11_ALT = 8;
        private readonly NativeImeClient _Client;
        private readonly IImeInputSink _ImeInputSink;
        private readonly HashSet<uint> _ConsumedKeys = new HashSet<uint>();
        private Vector2 _LastCursorPosition = new Vector2(float.MinValue, float.MinValue);
        private ISurroundingTextSnapshotProvider _SurroundingTextProvider;
        private int _LastSurroundingTextVersion;
        private string _LastSurroundingText = "";
        private int _LastSurroundingCursor = -1;
        private int _LastSurroundingAnchor = -1;
        private NativeImeTextContentType? _LastContentType;
        private bool _HasFocus;

        public bool IsAvailable => _Client != null && _Client.IsAvailable;

        public static LinuxNativeImeModule TryCreate(IImeInputSink imeInputSink)
        {
            var client = NativeImeClient.TryCreate();
            if (client == null)
            {
                return null;
            }

            Debug.Log(
                $"[LinuxNativeIme] backend={client.BackendName}, caps={FormatCapabilities(client.Features)}"
            );
            return new LinuxNativeImeModule(client, imeInputSink);
        }

        private LinuxNativeImeModule(NativeImeClient client, IImeInputSink imeInputSink)
        {
            _Client = client;
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
            _Client.FocusIn();
            Debug.Log(
                $"[LinuxNativeIme] focus in: backend={_Client.BackendName}, caps={FormatCapabilities(_Client.Features)}"
            );
            _Client.Reset();
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
            _Client.FocusOut();
            _Client.Reset();
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
                _Client.ProcessKeyEvent(keyval, 0, state, true);
                return true;
            }

            var handled = _Client.ProcessKeyEvent(
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
            _Client?.Dispose();
        }

        private void UpdateCursorRect()
        {
            var cursorPosition = Input.compositionCursorPos;
            if (cursorPosition == _LastCursorPosition)
            {
                return;
            }

            _LastCursorPosition = cursorPosition;
            _Client.SetCursorRect(
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

            if (!_Client.HasFeature(NativeImeFeature.SurroundingText))
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
            _Client.SetSurroundingText(snapshot.Text, snapshot.Cursor, snapshot.Anchor);
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

            UpdateContentType(NativeImeTextContentType.Normal);
        }

        private void UpdateContentType(NativeImeTextContentType contentType)
        {
            if (
                !IsAvailable
                || !_Client.HasFeature(NativeImeFeature.ContentType)
                || _LastContentType == contentType
            )
            {
                return;
            }

            _LastContentType = contentType;
            _Client.SetContentType(contentType);
            Debug.Log($"[LinuxNativeIme] content type sent: contentType={contentType}");
        }

        private static NativeImeTextContentType ParseContentType(byte contentType)
        {
            return contentType <= (byte)NativeImeTextContentType.Email
                ? (NativeImeTextContentType)contentType
                : NativeImeTextContentType.Normal;
        }

        private void DrainEvents()
        {
            while (_Client.TryReadMessage(out var message))
            {
                switch (message.Kind)
                {
                    case NativeImeMessageKind.Preedit:
                        Debug.Log(
                            $"[LinuxNativeIme] event=Preedit chars={message.Text.Length}, cursorBegin={message.CursorBegin}, cursorEnd={message.CursorEnd}"
                        );
                        LogTextTruncationWarning(message);
                        _ImeInputSink.SetImeComposition(
                            message.Text,
                            message.CursorBegin,
                            message.CursorEnd
                        );
                        break;
                    case NativeImeMessageKind.PreeditEnd:
                        Debug.Log("[LinuxNativeIme] event=PreeditEnd");
                        _ImeInputSink.CancelImeComposition();
                        break;
                    case NativeImeMessageKind.Commit:
                        Debug.Log($"[LinuxNativeIme] event=Commit chars={message.Text.Length}");
                        LogTextTruncationWarning(message);
                        _ImeInputSink.CommitImeText(message.Text);
                        break;
                    case NativeImeMessageKind.DeleteSurroundingText:
                        Debug.Log(
                            $"[LinuxNativeIme] event=DeleteSurroundingText offset={message.Param1}, length={message.Param2}"
                        );
                        _ImeInputSink.DeleteImeSurroundingText(message.Param1, message.Param2);
                        break;
                    case NativeImeMessageKind.ForwardKey:
                    case NativeImeMessageKind.None:
                    default:
                        break;
                }
            }
        }

        private static void LogTextTruncationWarning(NativeImeMessage message)
        {
            if (!message.TextTruncationSuspected)
            {
                return;
            }

            Debug.LogWarning(
                $"[LinuxNativeIme] native text buffer may be truncated: event={message.Kind}, cursorBegin={message.CursorBegin}, cursorEnd={message.CursorEnd}, param1={message.Param1}, param2={message.Param2}"
            );
        }

        private static int Utf8ByteLength(string text)
        {
            return string.IsNullOrEmpty(text) ? 0 : System.Text.Encoding.UTF8.GetByteCount(text);
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

        private static string FormatCapabilities(NativeImeFeature capabilities)
        {
            return capabilities == NativeImeFeature.None ? "None" : capabilities.ToString();
        }
    }
}
