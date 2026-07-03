using System;

namespace KimoTech.EmbeddedBrowser
{
    internal readonly struct NativeIpcResult
    {
        public NativeIpcResult(NativeIpcErrorCode code, string message)
        {
            Code = code;
            Message = message ?? string.Empty;
        }

        public NativeIpcErrorCode Code { get; }
        public string Message { get; }
        public bool IsOk => Code == NativeIpcErrorCode.Ok;

        public static NativeIpcResult Ok()
        {
            return new NativeIpcResult(NativeIpcErrorCode.Ok, string.Empty);
        }

        public static NativeIpcResult FromCode(int code)
        {
            var errorCode = Enum.IsDefined(typeof(NativeIpcErrorCode), code)
                ? (NativeIpcErrorCode)code
                : NativeIpcErrorCode.Unknown;
            var message = BrowserIpcNative.GetErrorMessage(code);
            return new NativeIpcResult(errorCode, message);
        }

        public void ThrowIfFailed()
        {
            if (!IsOk)
            {
                throw new InvalidOperationException(
                    Message.Length > 0 ? Message : $"IPC native call failed: {Code}"
                );
            }
        }
    }
}
