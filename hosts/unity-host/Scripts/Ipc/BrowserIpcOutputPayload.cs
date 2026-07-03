using System;
using System.Text;

namespace KimoTech.EmbeddedBrowser
{
    internal enum BrowserIpcOutputPayloadKind : ushort
    {
        Caret = 1,
        SurroundingText = 2,
        ScriptResult = 3,
        PageEvent = 4,
    }

    internal struct BrowserIpcOutputPayload
    {
        public BrowserIpcOutputPayloadKind Kind { get; set; }
        public int CaretX { get; set; }
        public int CaretY { get; set; }
        public int CaretWidth { get; set; }
        public int CaretHeight { get; set; }
        public bool CaretVisible { get; set; }
        public string Text { get; set; }
        public int SelectionStart { get; set; }
        public int SelectionEnd { get; set; }
        public ulong RequestId { get; set; }
        public bool Succeeded { get; set; }
        public uint PageEventType { get; set; }
        public string Url { get; set; }
        public string Detail { get; set; }

        public static bool TryDecode(
            byte[] buffer,
            int length,
            out BrowserIpcOutputPayload payload,
            out string error
        )
        {
            payload = default;
            error = "";

            if (buffer == null)
            {
                error = "buffer is null";
                return false;
            }

            if (length < 8 || length > buffer.Length)
            {
                error = $"invalid buffer length: {length}";
                return false;
            }

            if (
                buffer[0] != (byte)'E'
                || buffer[1] != (byte)'B'
                || buffer[2] != (byte)'O'
                || buffer[3] != (byte)'P'
            )
            {
                error = "invalid output payload magic";
                return false;
            }

            if (buffer[4] != 1 || buffer[5] != 0)
            {
                error = $"unsupported output payload version: {buffer[4]}.{buffer[5]}";
                return false;
            }

            var kind = (BrowserIpcOutputPayloadKind)ReadUInt16(buffer, 6);
            var cursor = 8;

            try
            {
                switch (kind)
                {
                    case BrowserIpcOutputPayloadKind.Caret:
                        payload = DecodeCaret(buffer, length, ref cursor);
                        break;
                    case BrowserIpcOutputPayloadKind.SurroundingText:
                        payload = DecodeSurroundingText(buffer, length, ref cursor);
                        break;
                    case BrowserIpcOutputPayloadKind.ScriptResult:
                        payload = DecodeScriptResult(buffer, length, ref cursor);
                        break;
                    case BrowserIpcOutputPayloadKind.PageEvent:
                        payload = DecodePageEvent(buffer, length, ref cursor);
                        break;
                    default:
                        throw new ArgumentException($"unknown output payload kind: {(ushort)kind}");
                }
            }
            catch (Exception exception)
            {
                error = exception.Message;
                return false;
            }

            if (cursor != length)
            {
                error = $"trailing output payload bytes: expected end {cursor}, actual {length}";
                return false;
            }

            return true;
        }

        private static BrowserIpcOutputPayload DecodeCaret(
            byte[] buffer,
            int length,
            ref int cursor
        )
        {
            var payload = new BrowserIpcOutputPayload
            {
                Kind = BrowserIpcOutputPayloadKind.Caret,
                CaretX = ReadInt32(buffer, length, ref cursor),
                CaretY = ReadInt32(buffer, length, ref cursor),
                CaretWidth = ReadInt32(buffer, length, ref cursor),
                CaretHeight = ReadInt32(buffer, length, ref cursor),
                CaretVisible = ReadByte(buffer, length, ref cursor) != 0,
            };
            Skip(buffer, length, ref cursor, 3);
            return payload;
        }

        private static BrowserIpcOutputPayload DecodeSurroundingText(
            byte[] buffer,
            int length,
            ref int cursor
        )
        {
            return new BrowserIpcOutputPayload
            {
                Kind = BrowserIpcOutputPayloadKind.SurroundingText,
                Text = ReadString(buffer, length, ref cursor),
                SelectionStart = ReadInt32(buffer, length, ref cursor),
                SelectionEnd = ReadInt32(buffer, length, ref cursor),
            };
        }

        private static BrowserIpcOutputPayload DecodeScriptResult(
            byte[] buffer,
            int length,
            ref int cursor
        )
        {
            var requestId = ReadUInt64(buffer, length, ref cursor);
            var succeeded = ReadByte(buffer, length, ref cursor) != 0;
            Skip(buffer, length, ref cursor, 3);
            return new BrowserIpcOutputPayload
            {
                Kind = BrowserIpcOutputPayloadKind.ScriptResult,
                RequestId = requestId,
                Succeeded = succeeded,
                Text = ReadString(buffer, length, ref cursor),
            };
        }

        private static BrowserIpcOutputPayload DecodePageEvent(
            byte[] buffer,
            int length,
            ref int cursor
        )
        {
            return new BrowserIpcOutputPayload
            {
                Kind = BrowserIpcOutputPayloadKind.PageEvent,
                PageEventType = ReadUInt32(buffer, length, ref cursor),
                Url = ReadString(buffer, length, ref cursor),
                Detail = ReadString(buffer, length, ref cursor),
            };
        }

        private static byte ReadByte(byte[] buffer, int length, ref int cursor)
        {
            RequireLength(length, cursor + 1);
            return buffer[cursor++];
        }

        private static int ReadInt32(byte[] buffer, int length, ref int cursor)
        {
            RequireLength(length, cursor + 4);
            var value = BitConverter.ToInt32(buffer, cursor);
            cursor += 4;
            return value;
        }

        private static uint ReadUInt32(byte[] buffer, int length, ref int cursor)
        {
            RequireLength(length, cursor + 4);
            var value = BitConverter.ToUInt32(buffer, cursor);
            cursor += 4;
            return value;
        }

        private static ulong ReadUInt64(byte[] buffer, int length, ref int cursor)
        {
            RequireLength(length, cursor + 8);
            var value = BitConverter.ToUInt64(buffer, cursor);
            cursor += 8;
            return value;
        }

        private static ushort ReadUInt16(byte[] buffer, int offset)
        {
            return BitConverter.ToUInt16(buffer, offset);
        }

        private static string ReadString(byte[] buffer, int length, ref int cursor)
        {
            var byteCount = checked((int)ReadUInt32(buffer, length, ref cursor));
            RequireLength(length, cursor + byteCount);
            var value = Encoding.UTF8.GetString(buffer, cursor, byteCount);
            cursor += byteCount;
            return value;
        }

        private static void Skip(byte[] buffer, int length, ref int cursor, int count)
        {
            RequireLength(length, cursor + count);
            cursor += count;
        }

        private static void RequireLength(int actualLength, int expectedLength)
        {
            if (actualLength < expectedLength)
            {
                throw new ArgumentException(
                    $"output payload buffer too small: expected {expectedLength}, actual {actualLength}"
                );
            }
        }
    }
}
