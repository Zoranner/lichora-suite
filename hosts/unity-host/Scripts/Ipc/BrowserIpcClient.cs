using System;

namespace KimoTech.EmbeddedBrowser
{
    public sealed class BrowserIpcClient : IDisposable
    {
        private NativeIpcHandle _Handle;

        private BrowserIpcClient(NativeIpcHandle handle, string sessionId)
        {
            _Handle = handle;
            SessionId = sessionId;
        }

        public string SessionId { get; }
        public bool IsOpen => _Handle != null && !_Handle.IsClosed;
        public BrowserIpcControlWriter Control => CreateControlWriter();
        public BrowserIpcStatusReader Status => CreateStatusReader();

        public static BrowserIpcClient Open(string sessionId)
        {
            return Open(new NativeIpcSessionOptions(sessionId));
        }

        internal void SendControl(ulong sequence, byte[] payload)
        {
            if (!IsOpen)
            {
                throw new ObjectDisposedException(nameof(BrowserIpcClient));
            }

            BrowserIpcNative
                .SendControl(_Handle.DangerousHandle, sequence, payload)
                .ThrowIfFailed();
        }

        public BrowserIpcControlWriter CreateControlWriter()
        {
            if (!IsOpen)
            {
                throw new ObjectDisposedException(nameof(BrowserIpcClient));
            }

            return new BrowserIpcControlWriter(_Handle);
        }

        public BrowserIpcStatusReader CreateStatusReader()
        {
            if (!IsOpen)
            {
                throw new ObjectDisposedException(nameof(BrowserIpcClient));
            }

            return new BrowserIpcStatusReader(_Handle);
        }

        public BrowserIpcInputWriter CreateInputWriter(string browserId)
        {
            if (!IsOpen)
            {
                throw new ObjectDisposedException(nameof(BrowserIpcClient));
            }

            BrowserIpcNative
                .OpenBrowserInput(_Handle.DangerousHandle, browserId, out var handle)
                .ThrowIfFailed();
            return new BrowserIpcInputWriter(
                new NativeIpcHandle(handle, BrowserIpcNative.CloseBrowserInputHandle)
            );
        }

        public BrowserIpcOutputReader CreateOutputReader(string browserId)
        {
            if (!IsOpen)
            {
                throw new ObjectDisposedException(nameof(BrowserIpcClient));
            }

            BrowserIpcNative
                .OpenBrowserOutput(_Handle.DangerousHandle, browserId, out var handle)
                .ThrowIfFailed();
            return new BrowserIpcOutputReader(
                new NativeIpcHandle(handle, BrowserIpcNative.CloseBrowserOutputHandle)
            );
        }

        public BrowserIpcFrameReader CreateFrameReader(string browserId)
        {
            if (!IsOpen)
            {
                throw new ObjectDisposedException(nameof(BrowserIpcClient));
            }

            BrowserIpcNative
                .OpenBrowserFrame(_Handle.DangerousHandle, browserId, out var handle)
                .ThrowIfFailed();
            return new BrowserIpcFrameReader(
                new NativeIpcHandle(handle, BrowserIpcNative.CloseBrowserFrameHandle)
            );
        }

        internal static BrowserIpcClient Open(NativeIpcSessionOptions options)
        {
            var result = BrowserIpcNative.OpenSession(options.SessionId, out var handle);
            result.ThrowIfFailed();
            return new BrowserIpcClient(new NativeIpcHandle(handle), options.SessionId);
        }

        public void Dispose()
        {
            _Handle?.Dispose();
            _Handle = null;
        }
    }
}
