using System;

namespace KimoTech.LichoraHost
{
    public sealed class BrowserIpcOutputReader : IDisposable
    {
        private readonly NativeIpcHandle _Handle;

        internal BrowserIpcOutputReader(NativeIpcHandle handle)
        {
            _Handle = handle ?? throw new ArgumentNullException(nameof(handle));
        }

        public int TryRead(byte[] buffer)
        {
            if (_Handle.IsClosed)
            {
                throw new ObjectDisposedException(nameof(BrowserIpcClient));
            }

            BrowserIpcNative
                .TryReadBrowserOutputLatest(_Handle.DangerousHandle, buffer, out var written)
                .ThrowIfFailed();
            return checked((int)written.ToUInt64());
        }

        public void Dispose()
        {
            _Handle.Dispose();
        }
    }
}
