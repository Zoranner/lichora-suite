using System;
using System.Text;

namespace KimoTech.LichoraHost
{
    [Flags]
    public enum NativeImeFeature : uint
    {
        None = 0,
        Preedit = 1 << 0,
        Commit = 1 << 1,
        ForwardKey = 1 << 2,
        DeleteSurroundingText = 1 << 3,
        SurroundingText = 1 << 4,
        ContentType = 1 << 5,
    }

    public enum NativeImeTextContentType
    {
        Normal = 0,
        Password = 1,
        Number = 2,
        Phone = 3,
        Url = 4,
        Email = 5,
    }

    public enum NativeImeMessageKind
    {
        None = 0,
        Preedit = 1,
        PreeditEnd = 2,
        Commit = 3,
        DeleteSurroundingText = 4,
        ForwardKey = 5,
    }

    public readonly struct NativeImeMessage
    {
        internal NativeImeMessage(
            NativeImeMessageKind kind,
            string text,
            int cursorBegin,
            int cursorEnd,
            int param1,
            int param2,
            bool textTruncationSuspected
        )
        {
            Kind = kind;
            Text = text;
            CursorBegin = cursorBegin;
            CursorEnd = cursorEnd;
            Param1 = param1;
            Param2 = param2;
            TextTruncationSuspected = textTruncationSuspected;
        }

        public NativeImeMessageKind Kind { get; }
        public string Text { get; }
        public int CursorBegin { get; }
        public int CursorEnd { get; }
        public int Param1 { get; }
        public int Param2 { get; }
        public bool TextTruncationSuspected { get; }
    }

    public sealed class NativeImeClient : IDisposable
    {
        private readonly NativeImeBridge _Bridge;

        private NativeImeClient(NativeImeBridge bridge)
        {
            _Bridge = bridge;
        }

        public bool IsAvailable => _Bridge != null && _Bridge.IsAvailable;
        public string BackendName => _Bridge?.BackendKind.ToString() ?? "Unknown";
        public NativeImeFeature Features =>
            _Bridge == null ? NativeImeFeature.None : (NativeImeFeature)_Bridge.Capabilities;

        public static NativeImeClient TryCreate()
        {
            var bridge = NativeImeBridge.TryCreate();
            return bridge == null ? null : new NativeImeClient(bridge);
        }

        public bool HasFeature(NativeImeFeature feature)
        {
            return (Features & feature) == feature;
        }

        public void FocusIn()
        {
            _Bridge?.FocusIn();
        }

        public void FocusOut()
        {
            _Bridge?.FocusOut();
        }

        public void Reset()
        {
            _Bridge?.Reset();
        }

        public void SetCursorRect(int x, int y, int width, int height)
        {
            _Bridge?.SetCursorRect(x, y, width, height);
        }

        public void SetContentType(NativeImeTextContentType contentType)
        {
            _Bridge?.SetContentType((NativeImeContentType)contentType);
        }

        public void SetSurroundingText(string text, int cursor, int anchor)
        {
            _Bridge?.SetSurroundingText(text, cursor, anchor);
        }

        public bool ProcessKeyEvent(uint keyval, uint keycode, uint state, bool isRelease)
        {
            return _Bridge != null && _Bridge.ProcessKeyEvent(keyval, keycode, state, isRelease);
        }

        public bool TryReadMessage(out NativeImeMessage message)
        {
            message = default;
            if (_Bridge == null || !_Bridge.TryPollEvent(out var data))
            {
                return false;
            }

            var textInfo = NativeImeBridge.AnalyzeTextBuffer(data.Text);
            var text =
                data.Text == null || textInfo.ByteLength == 0
                    ? ""
                    : Encoding.UTF8.GetString(data.Text, 0, textInfo.ByteLength);
            message = new NativeImeMessage(
                (NativeImeMessageKind)data.EventType,
                text,
                data.CursorBegin,
                data.CursorEnd,
                data.Param1,
                data.Param2,
                textInfo.IsTruncationSuspected
            );
            return true;
        }

        public void Dispose()
        {
            _Bridge?.Dispose();
        }
    }
}
