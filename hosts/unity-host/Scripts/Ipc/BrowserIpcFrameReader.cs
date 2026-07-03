using System;
using Unity.Collections;

namespace KimoTech.EmbeddedBrowser
{
    public sealed class BrowserIpcFrameReader : IDisposable
    {
        private readonly NativeIpcHandle _Handle;

        internal BrowserIpcFrameReader(NativeIpcHandle handle)
        {
            _Handle = handle ?? throw new ArgumentNullException(nameof(handle));
        }

        public bool TryCopyLatest(
            NativeArray<byte> buffer,
            ref ulong lastSequence,
            out int width,
            out int height,
            out ulong sequence,
            out int written
        )
        {
            width = 0;
            height = 0;
            sequence = 0;
            written = 0;

            if (_Handle.IsClosed)
            {
                throw new ObjectDisposedException(nameof(BrowserIpcClient));
            }

            var result = BrowserIpcNative.TryCopyLatestBrowserFrame(
                _Handle.DangerousHandle,
                buffer,
                out width,
                out height,
                out sequence,
                out var nativeWritten
            );

            if (result.Code == NativeIpcErrorCode.BufferTooSmall)
            {
                written = checked((int)nativeWritten.ToUInt64());
                return false;
            }

            result.ThrowIfFailed();
            written = checked((int)nativeWritten.ToUInt64());

            if (written <= 0 || sequence <= lastSequence)
            {
                return false;
            }

            return true;
        }

        public void Ack(ulong sequence)
        {
            if (_Handle.IsClosed)
            {
                throw new ObjectDisposedException(nameof(BrowserIpcClient));
            }

            BrowserIpcNative.AckBrowserFrame(_Handle.DangerousHandle, sequence).ThrowIfFailed();
        }

        public void Dispose()
        {
            _Handle.Dispose();
        }
    }
}
