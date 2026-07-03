using System;

namespace KimoTech.LichoraHost
{
    internal sealed class NativeIpcHandle : IDisposable
    {
        private readonly Action<IntPtr> _Close;
        private IntPtr _Handle;

        public NativeIpcHandle(IntPtr handle, Action<IntPtr> close = null)
        {
            if (handle == IntPtr.Zero)
            {
                throw new ArgumentException("Native IPC handle cannot be zero.", nameof(handle));
            }

            _Handle = handle;
            _Close = close ?? BrowserIpcNative.CloseSessionHandle;
        }

        public IntPtr DangerousHandle => _Handle;
        public bool IsClosed => _Handle == IntPtr.Zero;

        public void Dispose()
        {
            if (_Handle == IntPtr.Zero)
            {
                return;
            }

            var handle = _Handle;
            _Handle = IntPtr.Zero;
            _Close(handle);
        }
    }
}
