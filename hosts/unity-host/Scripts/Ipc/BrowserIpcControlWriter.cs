using System;

namespace KimoTech.EmbeddedBrowser
{
    public sealed class BrowserIpcControlWriter
    {
        private readonly NativeIpcHandle _Handle;

        internal BrowserIpcControlWriter(NativeIpcHandle handle)
        {
            _Handle = handle ?? throw new ArgumentNullException(nameof(handle));
        }

        public void AddBrowser(
            ulong sequence,
            string browserId,
            int width,
            int height,
            string address
        )
        {
            if (_Handle.IsClosed)
            {
                throw new ObjectDisposedException(nameof(BrowserIpcControlWriter));
            }

            BrowserIpcNative
                .AddBrowserControl(
                    _Handle.DangerousHandle,
                    sequence,
                    browserId,
                    width,
                    height,
                    address
                )
                .ThrowIfFailed();
        }

        public void RemoveBrowser(ulong sequence, string browserId)
        {
            if (_Handle.IsClosed)
            {
                throw new ObjectDisposedException(nameof(BrowserIpcControlWriter));
            }

            BrowserIpcNative
                .RemoveBrowserControl(_Handle.DangerousHandle, sequence, browserId)
                .ThrowIfFailed();
        }

        public void ResizeBrowser(ulong sequence, string browserId, int width, int height)
        {
            if (_Handle.IsClosed)
            {
                throw new ObjectDisposedException(nameof(BrowserIpcControlWriter));
            }

            BrowserIpcNative
                .ResizeBrowserControl(_Handle.DangerousHandle, sequence, browserId, width, height)
                .ThrowIfFailed();
        }

        public void Shutdown(ulong sequence)
        {
            if (_Handle.IsClosed)
            {
                throw new ObjectDisposedException(nameof(BrowserIpcControlWriter));
            }

            BrowserIpcNative.ShutdownControl(_Handle.DangerousHandle, sequence).ThrowIfFailed();
        }
    }
}
