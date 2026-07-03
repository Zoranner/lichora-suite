using System;
using System.Collections.Generic;
using System.Text;

namespace KimoTech.EmbeddedBrowser
{
    internal static class BrowserIpcInputPayload
    {
        private const byte PayloadVersionMajor = 1;
        private const byte PayloadVersionMinor = 0;

        public static byte[] EncodeMouseButton(
            int x,
            int y,
            uint button,
            uint buttons,
            bool pressed,
            byte clickCount,
            uint modifiers
        )
        {
            var buffer = CreateBuffer(BrowserIpcInputPayloadKind.MouseButton);
            WriteInt32(buffer, x);
            WriteInt32(buffer, y);
            WriteUInt32(buffer, button);
            WriteUInt32(buffer, buttons);
            WriteUInt32(buffer, modifiers);
            buffer.Add(pressed ? (byte)1 : (byte)0);
            buffer.Add(clickCount);
            buffer.Add(0);
            buffer.Add(0);
            return buffer.ToArray();
        }

        public static byte[] EncodeMouseWheel(int x, int y, int deltaX, int deltaY, uint modifiers)
        {
            var buffer = CreateBuffer(BrowserIpcInputPayloadKind.MouseWheel);
            WriteInt32(buffer, x);
            WriteInt32(buffer, y);
            WriteInt32(buffer, deltaX);
            WriteInt32(buffer, deltaY);
            WriteUInt32(buffer, modifiers);
            return buffer.ToArray();
        }

        public static byte[] EncodeKeyboardKey(
            bool pressed,
            uint keyCode,
            uint nativeKeyCode,
            uint modifiers
        )
        {
            var buffer = CreateBuffer(
                pressed
                    ? BrowserIpcInputPayloadKind.KeyboardKeyDown
                    : BrowserIpcInputPayloadKind.KeyboardKeyUp
            );
            WriteUInt32(buffer, keyCode);
            WriteUInt32(buffer, nativeKeyCode);
            WriteUInt32(buffer, modifiers);
            return buffer.ToArray();
        }

        public static byte[] EncodeKeyboardChar(uint codePoint, uint modifiers)
        {
            var buffer = CreateBuffer(BrowserIpcInputPayloadKind.KeyboardChar);
            WriteUInt32(buffer, codePoint);
            WriteUInt32(buffer, modifiers);
            return buffer.ToArray();
        }

        public static byte[] EncodeImeComposition(string text, int selectionStart, int selectionEnd)
        {
            var buffer = CreateBuffer(BrowserIpcInputPayloadKind.ImeComposition);
            WriteString(buffer, text);
            WriteInt32(buffer, selectionStart);
            WriteInt32(buffer, selectionEnd);
            return buffer.ToArray();
        }

        public static byte[] EncodeImeCommit(string text)
        {
            var buffer = CreateBuffer(BrowserIpcInputPayloadKind.ImeCommit);
            WriteString(buffer, text);
            return buffer.ToArray();
        }

        public static byte[] EncodeImeCancel()
        {
            return CreateBuffer(BrowserIpcInputPayloadKind.ImeCancel).ToArray();
        }

        public static byte[] EncodeImeDeleteSurroundingText(int before, int after)
        {
            var buffer = CreateBuffer(BrowserIpcInputPayloadKind.ImeDeleteSurroundingText);
            WriteInt32(buffer, before);
            WriteInt32(buffer, after);
            return buffer.ToArray();
        }

        public static byte[] EncodeScriptRequest(ulong requestId, string script)
        {
            var buffer = CreateBuffer(BrowserIpcInputPayloadKind.ScriptRequest);
            WriteUInt64(buffer, requestId);
            WriteString(buffer, script);
            return buffer.ToArray();
        }

        private static List<byte> CreateBuffer(BrowserIpcInputPayloadKind kind)
        {
            var buffer = new List<byte>(64)
            {
                (byte)'E',
                (byte)'B',
                (byte)'I',
                (byte)'P',
                PayloadVersionMajor,
                PayloadVersionMinor,
            };
            WriteUInt16(buffer, (ushort)kind);
            return buffer;
        }

        private static void WriteString(List<byte> buffer, string value)
        {
            if (value == null)
            {
                throw new ArgumentNullException(nameof(value));
            }

            var bytes = Encoding.UTF8.GetBytes(value);
            WriteUInt32(buffer, checked((uint)bytes.Length));
            buffer.AddRange(bytes);
        }

        private static void WriteInt32(List<byte> buffer, int value)
        {
            WriteUInt32(buffer, unchecked((uint)value));
        }

        private static void WriteUInt16(List<byte> buffer, ushort value)
        {
            buffer.Add((byte)value);
            buffer.Add((byte)(value >> 8));
        }

        private static void WriteUInt32(List<byte> buffer, uint value)
        {
            buffer.Add((byte)value);
            buffer.Add((byte)(value >> 8));
            buffer.Add((byte)(value >> 16));
            buffer.Add((byte)(value >> 24));
        }

        private static void WriteUInt64(List<byte> buffer, ulong value)
        {
            buffer.Add((byte)value);
            buffer.Add((byte)(value >> 8));
            buffer.Add((byte)(value >> 16));
            buffer.Add((byte)(value >> 24));
            buffer.Add((byte)(value >> 32));
            buffer.Add((byte)(value >> 40));
            buffer.Add((byte)(value >> 48));
            buffer.Add((byte)(value >> 56));
        }
    }
}
