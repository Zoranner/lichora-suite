using System;
using System.Text;

namespace KimoTech.LichoraHost
{
    public enum BrowserIpcOutputPayloadKind : ushort
    {
        Caret = 1,
        SurroundingText = 2,
        ScriptResult = 3,
        PageEvent = 4,
        OverlayPassMap = 5,
        InputOwnershipMap = 6,
    }

    public readonly struct BrowserOverlayPassRegionPayload
    {
        public BrowserOverlayPassRegionPayload(
            uint id,
            byte shape,
            bool disabled,
            float x,
            float y,
            float width,
            float height
        )
        {
            Id = id;
            Shape = shape;
            Disabled = disabled;
            X = x;
            Y = y;
            Width = width;
            Height = height;
        }

        public uint Id { get; }
        public byte Shape { get; }
        public bool Disabled { get; }
        public float X { get; }
        public float Y { get; }
        public float Width { get; }
        public float Height { get; }
    }

    public readonly struct BrowserOverlayPassMapPayload
    {
        public BrowserOverlayPassMapPayload(
            ulong version,
            int viewportWidth,
            int viewportHeight,
            float deviceScaleFactor,
            bool enabled,
            BrowserOverlayPassRegionPayload[] regions
        )
        {
            Version = version;
            ViewportWidth = viewportWidth;
            ViewportHeight = viewportHeight;
            DeviceScaleFactor = deviceScaleFactor;
            Enabled = enabled;
            Regions = regions ?? Array.Empty<BrowserOverlayPassRegionPayload>();
        }

        public ulong Version { get; }
        public int ViewportWidth { get; }
        public int ViewportHeight { get; }
        public float DeviceScaleFactor { get; }
        public bool Enabled { get; }
        public BrowserOverlayPassRegionPayload[] Regions { get; }
    }

    public readonly struct BrowserInputOwnershipRegionPayload
    {
        public BrowserInputOwnershipRegionPayload(
            uint id,
            byte owner,
            byte shape,
            bool disabled,
            float x,
            float y,
            float width,
            float height,
            float radius
        )
        {
            Id = id;
            Owner = owner;
            Shape = shape;
            Disabled = disabled;
            X = x;
            Y = y;
            Width = width;
            Height = height;
            Radius = radius;
        }

        public uint Id { get; }
        public byte Owner { get; }
        public byte Shape { get; }
        public bool Disabled { get; }
        public float X { get; }
        public float Y { get; }
        public float Width { get; }
        public float Height { get; }
        public float Radius { get; }
    }

    public readonly struct BrowserInputOwnershipMapPayload
    {
        public BrowserInputOwnershipMapPayload(
            ulong version,
            int viewportWidth,
            int viewportHeight,
            float deviceScaleFactor,
            bool enabled,
            byte defaultOwner,
            BrowserInputOwnershipRegionPayload[] regions
        )
        {
            Version = version;
            ViewportWidth = viewportWidth;
            ViewportHeight = viewportHeight;
            DeviceScaleFactor = deviceScaleFactor;
            Enabled = enabled;
            DefaultOwner = defaultOwner;
            Regions = regions ?? Array.Empty<BrowserInputOwnershipRegionPayload>();
        }

        public ulong Version { get; }
        public int ViewportWidth { get; }
        public int ViewportHeight { get; }
        public float DeviceScaleFactor { get; }
        public bool Enabled { get; }
        public byte DefaultOwner { get; }
        public BrowserInputOwnershipRegionPayload[] Regions { get; }
    }

    public struct BrowserIpcOutputPayload
    {
        private const int OverlayPassRegionPayloadSize = 24;
        private const int InputOwnershipRegionPayloadSize = 28;

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
        public BrowserOverlayPassMapPayload OverlayPassMap { get; set; }
        public BrowserInputOwnershipMapPayload InputOwnershipMap { get; set; }

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
                    case BrowserIpcOutputPayloadKind.OverlayPassMap:
                        payload = DecodeOverlayPassMap(buffer, length, ref cursor);
                        break;
                    case BrowserIpcOutputPayloadKind.InputOwnershipMap:
                        payload = DecodeInputOwnershipMap(buffer, length, ref cursor);
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

        private static BrowserIpcOutputPayload DecodeOverlayPassMap(
            byte[] buffer,
            int length,
            ref int cursor
        )
        {
            var version = ReadUInt64(buffer, length, ref cursor);
            var viewportWidth = ReadInt32(buffer, length, ref cursor);
            var viewportHeight = ReadInt32(buffer, length, ref cursor);
            var deviceScaleFactor = ReadSingle(buffer, length, ref cursor);
            var enabled = ReadByte(buffer, length, ref cursor) != 0;
            Skip(buffer, length, ref cursor, 7);

            var regionCount = ReadUInt32(buffer, length, ref cursor);
            var remainingRegionCapacity = (length - cursor) / OverlayPassRegionPayloadSize;
            if (regionCount > (uint)remainingRegionCapacity)
            {
                throw new ArgumentException(
                    $"overlay pass map region count exceeds payload length: count={regionCount}, capacity={remainingRegionCapacity}"
                );
            }

            var regions = new BrowserOverlayPassRegionPayload[checked((int)regionCount)];
            for (var index = 0; index < regions.Length; index++)
            {
                regions[index] = DecodeOverlayPassRegion(buffer, length, ref cursor);
            }

            return new BrowserIpcOutputPayload
            {
                Kind = BrowserIpcOutputPayloadKind.OverlayPassMap,
                OverlayPassMap = new BrowserOverlayPassMapPayload(
                    version,
                    viewportWidth,
                    viewportHeight,
                    deviceScaleFactor,
                    enabled,
                    regions
                ),
            };
        }

        private static BrowserOverlayPassRegionPayload DecodeOverlayPassRegion(
            byte[] buffer,
            int length,
            ref int cursor
        )
        {
            var id = ReadUInt32(buffer, length, ref cursor);
            var shape = ReadByte(buffer, length, ref cursor);
            var disabled = ReadByte(buffer, length, ref cursor) != 0;
            Skip(buffer, length, ref cursor, 2);
            return new BrowserOverlayPassRegionPayload(
                id,
                shape,
                disabled,
                ReadSingle(buffer, length, ref cursor),
                ReadSingle(buffer, length, ref cursor),
                ReadSingle(buffer, length, ref cursor),
                ReadSingle(buffer, length, ref cursor)
            );
        }

        private static BrowserIpcOutputPayload DecodeInputOwnershipMap(
            byte[] buffer,
            int length,
            ref int cursor
        )
        {
            var version = ReadUInt64(buffer, length, ref cursor);
            var viewportWidth = ReadInt32(buffer, length, ref cursor);
            var viewportHeight = ReadInt32(buffer, length, ref cursor);
            var deviceScaleFactor = ReadSingle(buffer, length, ref cursor);
            var enabled = ReadByte(buffer, length, ref cursor) != 0;
            var defaultOwner = ReadByte(buffer, length, ref cursor);
            Skip(buffer, length, ref cursor, 6);

            var regionCount = ReadUInt32(buffer, length, ref cursor);
            var remainingRegionCapacity = (length - cursor) / InputOwnershipRegionPayloadSize;
            if (regionCount > (uint)remainingRegionCapacity)
            {
                throw new ArgumentException(
                    $"input ownership map region count exceeds payload length: count={regionCount}, capacity={remainingRegionCapacity}"
                );
            }

            var regions = new BrowserInputOwnershipRegionPayload[checked((int)regionCount)];
            for (var index = 0; index < regions.Length; index++)
            {
                regions[index] = DecodeInputOwnershipRegion(buffer, length, ref cursor);
            }

            return new BrowserIpcOutputPayload
            {
                Kind = BrowserIpcOutputPayloadKind.InputOwnershipMap,
                InputOwnershipMap = new BrowserInputOwnershipMapPayload(
                    version,
                    viewportWidth,
                    viewportHeight,
                    deviceScaleFactor,
                    enabled,
                    defaultOwner,
                    regions
                ),
            };
        }

        private static BrowserInputOwnershipRegionPayload DecodeInputOwnershipRegion(
            byte[] buffer,
            int length,
            ref int cursor
        )
        {
            var id = ReadUInt32(buffer, length, ref cursor);
            var owner = ReadByte(buffer, length, ref cursor);
            var shape = ReadByte(buffer, length, ref cursor);
            var disabled = ReadByte(buffer, length, ref cursor) != 0;
            Skip(buffer, length, ref cursor, 1);
            return new BrowserInputOwnershipRegionPayload(
                id,
                owner,
                shape,
                disabled,
                ReadSingle(buffer, length, ref cursor),
                ReadSingle(buffer, length, ref cursor),
                ReadSingle(buffer, length, ref cursor),
                ReadSingle(buffer, length, ref cursor),
                ReadSingle(buffer, length, ref cursor)
            );
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

        private static float ReadSingle(byte[] buffer, int length, ref int cursor)
        {
            RequireLength(length, cursor + 4);
            var value = BitConverter.ToSingle(buffer, cursor);
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
