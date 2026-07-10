using System;

namespace KimoTech.LichoraHost
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

    internal readonly struct NativeIpcSessionOptions
    {
        public NativeIpcSessionOptions(string sessionId)
        {
            SessionId = sessionId ?? throw new ArgumentNullException(nameof(sessionId));
        }

        public string SessionId { get; }
    }
}
