using System;

namespace KimoTech.EmbeddedBrowser
{
    internal enum NativeIpcErrorCode
    {
        Unknown = -1,
        Ok = 0,
        InvalidArgument = 1,
        NotImplemented = 2,
        Io = 3,
        BufferTooSmall = 4,
        QueueFull = 5,
        Panic = 100,
    }

    public enum NativeIpcInputEventKind : uint
    {
        MouseButton = 1,
        MouseWheel = 2,
        Keyboard = 3,
        Ime = 4,
        Script = 5,
    }

    internal enum BrowserIpcInputPayloadKind : ushort
    {
        MouseButton = 1,
        MouseWheel = 2,
        KeyboardKeyDown = 3,
        KeyboardKeyUp = 4,
        KeyboardChar = 5,
        ImeComposition = 6,
        ImeCommit = 7,
        ImeCancel = 8,
        ImeDeleteSurroundingText = 9,
        ScriptRequest = 10,
    }

    internal readonly struct NativeIpcSessionOptions
    {
        public NativeIpcSessionOptions(string sessionId)
        {
            SessionId = sessionId ?? throw new ArgumentNullException(nameof(sessionId));
        }

        public string SessionId { get; }
    }
}
