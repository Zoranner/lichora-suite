using System;

namespace KimoTech.LichoraHost
{
    public sealed class BrowserIpcStatusReader
    {
        private readonly NativeIpcHandle _Handle;

        internal BrowserIpcStatusReader(NativeIpcHandle handle)
        {
            _Handle = handle ?? throw new ArgumentNullException(nameof(handle));
        }

        public int Read(byte[] buffer)
        {
            if (_Handle.IsClosed)
            {
                throw new ObjectDisposedException(nameof(BrowserIpcClient));
            }

            BrowserIpcNative
                .ReadStatus(_Handle.DangerousHandle, buffer, out var written)
                .ThrowIfFailed();
            return checked((int)written.ToUInt64());
        }
    }
}
