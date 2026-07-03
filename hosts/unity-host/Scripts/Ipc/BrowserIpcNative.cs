using System;
using System.Runtime.InteropServices;
using System.Text;
using Unity.Collections;
using Unity.Collections.LowLevel.Unsafe;

namespace KimoTech.LichoraHost
{
    internal static class BrowserIpcNative
    {
        private const string PLUGIN = "lichora_ipc_native";

        public static NativeIpcResult OpenSession(string sessionId, out IntPtr handle)
        {
            handle = IntPtr.Zero;
            if (string.IsNullOrWhiteSpace(sessionId))
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "session id is required"
                );
            }

            var sessionIdBytes = Encoding.UTF8.GetBytes(sessionId);
            var code = ebi_session_open(sessionIdBytes, (UIntPtr)sessionIdBytes.Length, out handle);
            return NativeIpcResult.FromCode(code);
        }

        public static NativeIpcResult CloseSession(IntPtr handle)
        {
            var code = ebi_session_close(handle);
            return NativeIpcResult.FromCode(code);
        }

        public static void CloseSessionHandle(IntPtr handle)
        {
            CloseSession(handle).ThrowIfFailed();
        }

        public static void CloseBrowserInputHandle(IntPtr handle)
        {
            BrowserInputClose(handle).ThrowIfFailed();
        }

        public static void CloseBrowserFrameHandle(IntPtr handle)
        {
            BrowserFrameClose(handle).ThrowIfFailed();
        }

        public static void CloseBrowserOutputHandle(IntPtr handle)
        {
            BrowserOutputClose(handle).ThrowIfFailed();
        }

        public static NativeIpcResult SendControl(IntPtr handle, ulong sequence, byte[] payload)
        {
            if (handle == IntPtr.Zero || payload == null)
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "control handle and payload are required"
                );
            }

            var code = ebi_control_send(handle, sequence, payload, (UIntPtr)payload.Length);
            return NativeIpcResult.FromCode(code);
        }

        public static NativeIpcResult AddBrowserControl(
            IntPtr handle,
            ulong sequence,
            string browserId,
            int width,
            int height,
            string address
        )
        {
            if (handle == IntPtr.Zero || string.IsNullOrWhiteSpace(browserId))
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "control handle and browser id are required"
                );
            }

            if (width <= 0 || height <= 0)
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "browser dimensions must be positive"
                );
            }

            if (string.IsNullOrWhiteSpace(address))
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "browser address is required"
                );
            }

            var browserIdBytes = Encoding.UTF8.GetBytes(browserId);
            var addressBytes = Encoding.UTF8.GetBytes(address);
            var code = ebi_control_add_browser(
                handle,
                sequence,
                browserIdBytes,
                (UIntPtr)browserIdBytes.Length,
                width,
                height,
                addressBytes,
                (UIntPtr)addressBytes.Length
            );
            return NativeIpcResult.FromCode(code);
        }

        public static NativeIpcResult RemoveBrowserControl(
            IntPtr handle,
            ulong sequence,
            string browserId
        )
        {
            if (handle == IntPtr.Zero || string.IsNullOrWhiteSpace(browserId))
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "control handle and browser id are required"
                );
            }

            var browserIdBytes = Encoding.UTF8.GetBytes(browserId);
            var code = ebi_control_remove_browser(
                handle,
                sequence,
                browserIdBytes,
                (UIntPtr)browserIdBytes.Length
            );
            return NativeIpcResult.FromCode(code);
        }

        public static NativeIpcResult ResizeBrowserControl(
            IntPtr handle,
            ulong sequence,
            string browserId,
            int width,
            int height
        )
        {
            if (handle == IntPtr.Zero || string.IsNullOrWhiteSpace(browserId))
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "control handle and browser id are required"
                );
            }

            var browserIdBytes = Encoding.UTF8.GetBytes(browserId);
            var code = ebi_control_resize_browser(
                handle,
                sequence,
                browserIdBytes,
                (UIntPtr)browserIdBytes.Length,
                width,
                height
            );
            return NativeIpcResult.FromCode(code);
        }

        public static NativeIpcResult ShutdownControl(IntPtr handle, ulong sequence)
        {
            if (handle == IntPtr.Zero)
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "control handle is required"
                );
            }

            var code = ebi_control_shutdown(handle, sequence);
            return NativeIpcResult.FromCode(code);
        }

        public static NativeIpcResult OpenBrowserInput(
            IntPtr handle,
            string browserId,
            out IntPtr browserInputHandle
        )
        {
            browserInputHandle = IntPtr.Zero;
            if (handle == IntPtr.Zero || string.IsNullOrWhiteSpace(browserId))
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "session handle and browser id are required"
                );
            }

            var browserIdBytes = Encoding.UTF8.GetBytes(browserId);
            var code = ebi_browser_input_open(
                handle,
                browserIdBytes,
                (UIntPtr)browserIdBytes.Length,
                out browserInputHandle
            );
            return NativeIpcResult.FromCode(code);
        }

        public static NativeIpcResult BrowserInputClose(IntPtr handle)
        {
            var code = ebi_browser_input_close(handle);
            return NativeIpcResult.FromCode(code);
        }

        public static NativeIpcResult SetBrowserInputMouseLatest(
            IntPtr handle,
            int x,
            int y,
            uint buttons,
            int deltaX,
            int deltaY,
            bool valid
        )
        {
            if (handle == IntPtr.Zero)
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "browser input handle is required"
                );
            }

            var code = ebi_browser_input_set_mouse_latest(
                handle,
                x,
                y,
                buttons,
                deltaX,
                deltaY,
                valid ? (byte)1 : (byte)0
            );
            return NativeIpcResult.FromCode(code);
        }

        public static NativeIpcResult PushBrowserInputEvent(
            IntPtr handle,
            NativeIpcInputEventKind kind,
            ulong sequence,
            byte[] payload
        )
        {
            if (handle == IntPtr.Zero || payload == null)
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "browser input handle and payload are required"
                );
            }

            var code = ebi_browser_input_push_event(
                handle,
                (uint)kind,
                sequence,
                payload,
                (UIntPtr)payload.Length
            );
            return NativeIpcResult.FromCode(code);
        }

        public static NativeIpcResult ReadStatus(IntPtr handle, byte[] buffer, out UIntPtr written)
        {
            written = UIntPtr.Zero;
            if (handle == IntPtr.Zero || buffer == null || buffer.Length == 0)
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "status handle and buffer are required"
                );
            }

            var code = ebi_status_read(handle, buffer, (UIntPtr)buffer.Length, out written);
            return NativeIpcResult.FromCode(code);
        }

        public static NativeIpcResult OpenBrowserOutput(
            IntPtr handle,
            string browserId,
            out IntPtr browserOutputHandle
        )
        {
            browserOutputHandle = IntPtr.Zero;
            if (handle == IntPtr.Zero || string.IsNullOrWhiteSpace(browserId))
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "session handle and browser id are required"
                );
            }

            var browserIdBytes = Encoding.UTF8.GetBytes(browserId);
            var code = ebi_browser_output_open(
                handle,
                browserIdBytes,
                (UIntPtr)browserIdBytes.Length,
                out browserOutputHandle
            );
            return NativeIpcResult.FromCode(code);
        }

        public static NativeIpcResult BrowserOutputClose(IntPtr handle)
        {
            var code = ebi_browser_output_close(handle);
            return NativeIpcResult.FromCode(code);
        }

        public static NativeIpcResult TryReadBrowserOutputLatest(
            IntPtr handle,
            byte[] buffer,
            out UIntPtr written
        )
        {
            written = UIntPtr.Zero;
            if (handle == IntPtr.Zero || buffer == null || buffer.Length == 0)
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "browser output handle and buffer are required"
                );
            }

            var code = ebi_browser_output_try_read_latest(
                handle,
                buffer,
                (UIntPtr)buffer.Length,
                out written
            );
            return NativeIpcResult.FromCode(code);
        }

        public static NativeIpcResult OpenBrowserFrame(
            IntPtr handle,
            string browserId,
            out IntPtr browserFrameHandle
        )
        {
            browserFrameHandle = IntPtr.Zero;
            if (handle == IntPtr.Zero || string.IsNullOrWhiteSpace(browserId))
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "session handle and browser id are required"
                );
            }

            var browserIdBytes = Encoding.UTF8.GetBytes(browserId);
            var code = ebi_browser_frame_open(
                handle,
                browserIdBytes,
                (UIntPtr)browserIdBytes.Length,
                out browserFrameHandle
            );
            return NativeIpcResult.FromCode(code);
        }

        public static NativeIpcResult BrowserFrameClose(IntPtr handle)
        {
            var code = ebi_browser_frame_close(handle);
            return NativeIpcResult.FromCode(code);
        }

        public static unsafe NativeIpcResult TryCopyLatestBrowserFrame(
            IntPtr handle,
            NativeArray<byte> buffer,
            out int width,
            out int height,
            out ulong sequence,
            out UIntPtr written
        )
        {
            width = 0;
            height = 0;
            sequence = 0;
            written = UIntPtr.Zero;

            if (handle == IntPtr.Zero)
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "browser frame handle is required"
                );
            }

            if (!buffer.IsCreated || buffer.Length == 0)
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "frame buffer is required"
                );
            }

            var bufferPtr = (byte*)NativeArrayUnsafeUtility.GetUnsafePtr(buffer);
            var code = ebi_browser_frame_try_copy_latest(
                handle,
                bufferPtr,
                (UIntPtr)buffer.Length,
                out width,
                out height,
                out sequence,
                out written
            );
            return NativeIpcResult.FromCode(code);
        }

        public static NativeIpcResult AckBrowserFrame(IntPtr handle, ulong sequence)
        {
            if (handle == IntPtr.Zero)
            {
                return new NativeIpcResult(
                    NativeIpcErrorCode.InvalidArgument,
                    "browser frame handle is required"
                );
            }

            var code = ebi_browser_frame_ack(handle, sequence);
            return NativeIpcResult.FromCode(code);
        }

        public static string GetErrorMessage(int code)
        {
            var messagePtr = ebi_error_message(code);
            return messagePtr == IntPtr.Zero
                ? string.Empty
                : Marshal.PtrToStringAnsi(messagePtr) ?? string.Empty;
        }

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_session_open(
            byte[] sessionId,
            UIntPtr sessionIdLength,
            out IntPtr handle
        );

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_session_close(IntPtr handle);

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_control_send(
            IntPtr handle,
            ulong sequence,
            byte[] payload,
            UIntPtr payloadLength
        );

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_control_add_browser(
            IntPtr handle,
            ulong sequence,
            [In] byte[] browserIdUtf8,
            UIntPtr browserIdLength,
            int width,
            int height,
            [In] byte[] addressUtf8,
            UIntPtr addressLength
        );

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_control_remove_browser(
            IntPtr handle,
            ulong sequence,
            [In] byte[] browserIdUtf8,
            UIntPtr browserIdLength
        );

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_control_resize_browser(
            IntPtr handle,
            ulong sequence,
            [In] byte[] browserIdUtf8,
            UIntPtr browserIdLength,
            int width,
            int height
        );

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_control_shutdown(IntPtr handle, ulong sequence);

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_browser_input_open(
            IntPtr handle,
            [In] byte[] browserIdUtf8,
            UIntPtr browserIdLength,
            out IntPtr browserInputHandle
        );

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_browser_input_close(IntPtr handle);

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_browser_input_set_mouse_latest(
            IntPtr handle,
            int x,
            int y,
            uint buttons,
            int deltaX,
            int deltaY,
            byte valid
        );

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_browser_input_push_event(
            IntPtr handle,
            uint kind,
            ulong sequence,
            byte[] payload,
            UIntPtr payloadLength
        );

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_status_read(
            IntPtr handle,
            [Out] byte[] buffer,
            UIntPtr bufferLength,
            out UIntPtr written
        );

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_browser_output_open(
            IntPtr handle,
            [In] byte[] browserId,
            UIntPtr browserIdLength,
            out IntPtr browserOutputHandle
        );

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_browser_output_close(IntPtr handle);

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_browser_output_try_read_latest(
            IntPtr handle,
            [Out] byte[] buffer,
            UIntPtr bufferLength,
            out UIntPtr written
        );

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_browser_frame_open(
            IntPtr handle,
            [In] byte[] browserId,
            UIntPtr browserIdLength,
            out IntPtr browserFrameHandle
        );

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_browser_frame_close(IntPtr handle);

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern unsafe int ebi_browser_frame_try_copy_latest(
            IntPtr handle,
            byte* buffer,
            UIntPtr bufferLength,
            out int width,
            out int height,
            out ulong sequence,
            out UIntPtr written
        );

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ebi_browser_frame_ack(IntPtr handle, ulong sequence);

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr ebi_error_message(int code);
    }
}
