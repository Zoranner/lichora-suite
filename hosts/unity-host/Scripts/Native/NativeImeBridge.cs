using System;
using System.Runtime.InteropServices;
using System.Text;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    internal enum NativeImeEventType
    {
        None = 0,
        Preedit = 1,
        PreeditEnd = 2,
        Commit = 3,
        DeleteSurroundingText = 4,
        ForwardKey = 5,
    }

    internal enum NativeImeBackendKind
    {
        Unknown = 0,
        Fcitx5 = 1,
        Fcitx4 = 2,
        IBus = 3,
    }

    [Flags]
    internal enum NativeImeCapabilities : uint
    {
        None = 0,
        Preedit = 1 << 0,
        Commit = 1 << 1,
        ForwardKey = 1 << 2,
        DeleteSurroundingText = 1 << 3,
        SurroundingText = 1 << 4,
        ContentType = 1 << 5,
    }

    internal enum NativeImeContentType
    {
        Normal = 0,
        Password = 1,
        Number = 2,
        Phone = 3,
        Url = 4,
        Email = 5,
    }

    [StructLayout(LayoutKind.Sequential)]
    internal struct NativeImeEventData
    {
        public int EventType;

        [MarshalAs(UnmanagedType.ByValArray, SizeConst = NativeImeBridge.TEXT_BUFFER_SIZE)]
        public byte[] Text;

        public int CursorBegin;
        public int CursorEnd;
        public int Param1;
        public int Param2;
    }

    internal sealed class NativeImeBridge : IDisposable
    {
        private const string PLUGIN = "native_ime";
        internal const int TEXT_BUFFER_SIZE = 2048;
        internal const int TEXT_PAYLOAD_LIMIT = TEXT_BUFFER_SIZE - 1;
        private IntPtr _Handle;
        private bool _OptionalAbiAvailable = true;

        public bool IsAvailable => _Handle != IntPtr.Zero;
        public NativeImeBackendKind BackendKind { get; private set; } =
            NativeImeBackendKind.Unknown;
        public NativeImeCapabilities Capabilities { get; private set; } =
            NativeImeCapabilities.None;

        public static NativeImeBridge TryCreate()
        {
            if (
                !Application.platform.Equals(RuntimePlatform.LinuxEditor)
                && !Application.platform.Equals(RuntimePlatform.LinuxPlayer)
            )
            {
                return null;
            }

            try
            {
                var handle = ime_create();
                if (handle == IntPtr.Zero)
                {
                    Debug.Log(
                        "[NativeImeBridge] native-ime backend unavailable, using fallback IME path."
                    );
                    return null;
                }

                var bridge = new NativeImeBridge(handle);
                bridge.RefreshOptionalAbi();
                return bridge;
            }
            catch (DllNotFoundException)
            {
                Debug.Log("[NativeImeBridge] libnative_ime.so not found, using fallback IME path.");
                return null;
            }
            catch (EntryPointNotFoundException ex)
            {
                Debug.LogWarning($"[NativeImeBridge] Invalid native-ime plugin: {ex.Message}");
                return null;
            }
        }

        private NativeImeBridge(IntPtr handle)
        {
            _Handle = handle;
        }

        public void FocusIn()
        {
            if (!IsAvailable)
            {
                return;
            }

            ime_focus_in(_Handle);
        }

        public void FocusOut()
        {
            if (!IsAvailable)
            {
                return;
            }

            ime_focus_out(_Handle);
        }

        public void SetCursorRect(int x, int y, int width, int height)
        {
            if (!IsAvailable)
            {
                return;
            }

            ime_set_cursor_rect(_Handle, x, y, width, height);
        }

        public void Reset()
        {
            if (!IsAvailable)
            {
                return;
            }

            ime_reset(_Handle);
        }

        public bool HasCapability(NativeImeCapabilities capability)
        {
            return (Capabilities & capability) == capability;
        }

        public void SetContentType(NativeImeContentType contentType)
        {
            if (!IsAvailable || !HasCapability(NativeImeCapabilities.ContentType))
            {
                return;
            }

            try
            {
                ime_set_content_type(_Handle, (int)contentType);
            }
            catch (EntryPointNotFoundException ex)
            {
                DisableOptionalAbi(ex);
            }
        }

        public void SetSurroundingText(string text, int cursor, int anchor)
        {
            if (!IsAvailable || !HasCapability(NativeImeCapabilities.SurroundingText))
            {
                return;
            }

            try
            {
                var textBytes = Encoding.UTF8.GetBytes(text ?? "");
                var nullTerminatedText = new byte[textBytes.Length + 1];
                Array.Copy(textBytes, nullTerminatedText, textBytes.Length);
                ime_set_surrounding_text(_Handle, nullTerminatedText, cursor, anchor);
            }
            catch (EntryPointNotFoundException ex)
            {
                DisableOptionalAbi(ex);
            }
        }

        public bool ProcessKeyEvent(uint keyval, uint keycode, uint state, bool isRelease)
        {
            return IsAvailable
                && ime_process_key_event(_Handle, keyval, keycode, state, isRelease ? 1 : 0) == 1;
        }

        public bool TryPollEvent(out NativeImeEventData data)
        {
            data = new NativeImeEventData { Text = new byte[TEXT_BUFFER_SIZE] };
            return IsAvailable && ime_poll_event(_Handle, ref data) != 0;
        }

        internal static NativeImeTextBufferInfo AnalyzeTextBuffer(byte[] bytes)
        {
            if (bytes == null || bytes.Length == 0)
            {
                return new NativeImeTextBufferInfo(0, false, false);
            }

            var terminatorIndex = Array.IndexOf(bytes, (byte)0);
            var hasNullTerminator = terminatorIndex >= 0;
            var byteLength = hasNullTerminator ? terminatorIndex : bytes.Length;
            var isAtNativeLimit =
                byteLength >= TEXT_PAYLOAD_LIMIT
                || (!hasNullTerminator && byteLength >= TEXT_BUFFER_SIZE);

            return new NativeImeTextBufferInfo(byteLength, hasNullTerminator, isAtNativeLimit);
        }

        public void Dispose()
        {
            if (!IsAvailable)
            {
                return;
            }

            ime_destroy(_Handle);
            _Handle = IntPtr.Zero;
        }

        private void RefreshOptionalAbi()
        {
            if (!IsAvailable || !_OptionalAbiAvailable)
            {
                return;
            }

            try
            {
                BackendKind = (NativeImeBackendKind)ime_backend_kind(_Handle);
                Capabilities = (NativeImeCapabilities)ime_capabilities(_Handle);
            }
            catch (EntryPointNotFoundException ex)
            {
                DisableOptionalAbi(ex);
            }
        }

        private void DisableOptionalAbi(EntryPointNotFoundException ex)
        {
            _OptionalAbiAvailable = false;
            BackendKind = NativeImeBackendKind.Unknown;
            Capabilities = NativeImeCapabilities.None;
            Debug.LogWarning(
                $"[NativeImeBridge] native-ime optional ABI unavailable: {ex.Message}"
            );
        }

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr ime_create();

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern void ime_destroy(IntPtr handle);

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern void ime_focus_in(IntPtr handle);

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern void ime_focus_out(IntPtr handle);

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern void ime_set_cursor_rect(
            IntPtr handle,
            int x,
            int y,
            int width,
            int height
        );

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern void ime_reset(IntPtr handle);

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ime_backend_kind(IntPtr handle);

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern uint ime_capabilities(IntPtr handle);

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern void ime_set_content_type(IntPtr handle, int contentType);

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern void ime_set_surrounding_text(
            IntPtr handle,
            byte[] text,
            int cursor,
            int anchor
        );

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ime_process_key_event(
            IntPtr handle,
            uint keyval,
            uint keycode,
            uint state,
            int isRelease
        );

        [DllImport(PLUGIN, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ime_poll_event(IntPtr handle, ref NativeImeEventData data);
    }

    internal readonly struct NativeImeTextBufferInfo
    {
        public NativeImeTextBufferInfo(int byteLength, bool hasNullTerminator, bool isAtNativeLimit)
        {
            ByteLength = byteLength;
            HasNullTerminator = hasNullTerminator;
            IsAtNativeLimit = isAtNativeLimit;
        }

        public int ByteLength { get; }
        public bool HasNullTerminator { get; }
        public bool IsAtNativeLimit { get; }
        public bool IsTruncationSuspected => !HasNullTerminator || IsAtNativeLimit;
    }
}
