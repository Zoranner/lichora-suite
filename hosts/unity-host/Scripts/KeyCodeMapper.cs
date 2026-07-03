using System.Collections.Generic;
using System.Runtime.InteropServices;
using UnityEngine;

namespace KimoTech.EmbeddedBrowser
{
    public static class KeyCodeMapper
    {
        private static readonly Dictionary<KeyCode, int> _KeyCodeToVirtualKey = new Dictionary<
            KeyCode,
            int
        >
        {
            // 字母键 A-Z
            { KeyCode.A, 0x41 },
            { KeyCode.B, 0x42 },
            { KeyCode.C, 0x43 },
            { KeyCode.D, 0x44 },
            { KeyCode.E, 0x45 },
            { KeyCode.F, 0x46 },
            { KeyCode.G, 0x47 },
            { KeyCode.H, 0x48 },
            { KeyCode.I, 0x49 },
            { KeyCode.J, 0x4A },
            { KeyCode.K, 0x4B },
            { KeyCode.L, 0x4C },
            { KeyCode.M, 0x4D },
            { KeyCode.N, 0x4E },
            { KeyCode.O, 0x4F },
            { KeyCode.P, 0x50 },
            { KeyCode.Q, 0x51 },
            { KeyCode.R, 0x52 },
            { KeyCode.S, 0x53 },
            { KeyCode.T, 0x54 },
            { KeyCode.U, 0x55 },
            { KeyCode.V, 0x56 },
            { KeyCode.W, 0x57 },
            { KeyCode.X, 0x58 },
            { KeyCode.Y, 0x59 },
            { KeyCode.Z, 0x5A },
            // 数字键 0-9
            { KeyCode.Alpha0, 0x30 },
            { KeyCode.Alpha1, 0x31 },
            { KeyCode.Alpha2, 0x32 },
            { KeyCode.Alpha3, 0x33 },
            { KeyCode.Alpha4, 0x34 },
            { KeyCode.Alpha5, 0x35 },
            { KeyCode.Alpha6, 0x36 },
            { KeyCode.Alpha7, 0x37 },
            { KeyCode.Alpha8, 0x38 },
            { KeyCode.Alpha9, 0x39 },
            // 小键盘数字 0-9
            { KeyCode.Keypad0, 0x60 },
            { KeyCode.Keypad1, 0x61 },
            { KeyCode.Keypad2, 0x62 },
            { KeyCode.Keypad3, 0x63 },
            { KeyCode.Keypad4, 0x64 },
            { KeyCode.Keypad5, 0x65 },
            { KeyCode.Keypad6, 0x66 },
            { KeyCode.Keypad7, 0x67 },
            { KeyCode.Keypad8, 0x68 },
            { KeyCode.Keypad9, 0x69 },
            // 小键盘运算符
            { KeyCode.KeypadMultiply, 0x6A },
            { KeyCode.KeypadPlus, 0x6B },
            { KeyCode.KeypadMinus, 0x6D },
            { KeyCode.KeypadPeriod, 0x6E },
            { KeyCode.KeypadDivide, 0x6F },
            { KeyCode.KeypadEnter, 0x0D },
            // 功能键 F1-F12
            { KeyCode.F1, 0x70 },
            { KeyCode.F2, 0x71 },
            { KeyCode.F3, 0x72 },
            { KeyCode.F4, 0x73 },
            { KeyCode.F5, 0x74 },
            { KeyCode.F6, 0x75 },
            { KeyCode.F7, 0x76 },
            { KeyCode.F8, 0x77 },
            { KeyCode.F9, 0x78 },
            { KeyCode.F10, 0x79 },
            { KeyCode.F11, 0x7A },
            { KeyCode.F12, 0x7B },
            // 控制键
            { KeyCode.Backspace, 0x08 },
            { KeyCode.Tab, 0x09 },
            { KeyCode.Return, 0x0D },
            { KeyCode.Escape, 0x1B },
            { KeyCode.Space, 0x20 },
            { KeyCode.Delete, 0x2E },
            { KeyCode.Insert, 0x2D },
            { KeyCode.Home, 0x24 },
            { KeyCode.End, 0x23 },
            { KeyCode.PageUp, 0x21 },
            { KeyCode.PageDown, 0x22 },
            // 方向键
            { KeyCode.UpArrow, 0x26 },
            { KeyCode.DownArrow, 0x28 },
            { KeyCode.LeftArrow, 0x25 },
            { KeyCode.RightArrow, 0x27 },
            // 修饰键
            { KeyCode.LeftShift, 0x10 },
            { KeyCode.RightShift, 0x10 },
            { KeyCode.LeftControl, 0x11 },
            { KeyCode.RightControl, 0x11 },
            { KeyCode.LeftAlt, 0x12 },
            { KeyCode.RightAlt, 0x12 },
            { KeyCode.LeftWindows, 0x5B },
            { KeyCode.RightWindows, 0x5C },
            { KeyCode.CapsLock, 0x14 },
            { KeyCode.Numlock, 0x90 },
            { KeyCode.ScrollLock, 0x91 },
            // 标点符号
            { KeyCode.Semicolon, 0xBA },
            { KeyCode.Equals, 0xBB },
            { KeyCode.Comma, 0xBC },
            { KeyCode.Minus, 0xBD },
            { KeyCode.Period, 0xBE },
            { KeyCode.Slash, 0xBF },
            { KeyCode.BackQuote, 0xC0 },
            { KeyCode.LeftBracket, 0xDB },
            { KeyCode.Backslash, 0xDC },
            { KeyCode.RightBracket, 0xDD },
            { KeyCode.Quote, 0xDE },
        };

        public static int ToVirtualKey(KeyCode keyCode)
        {
            return _KeyCodeToVirtualKey.TryGetValue(keyCode, out var virtualKey) ? virtualKey : 0;
        }

        public static int ToNativeKey(KeyCode keyCode)
        {
            return ToVirtualKey(keyCode);
        }

        public static int ToNativeKey(KeyCode keyCode, char character)
        {
            return RuntimeInformation.IsOSPlatform(OSPlatform.Linux)
                ? (int)ToX11KeySym(keyCode, character)
                : ToNativeKey(keyCode);
        }

        public static uint ToX11KeySym(KeyCode keyCode, char character)
        {
            if (character >= 0x20 && character < 0x7f)
            {
                return character;
            }

            switch (keyCode)
            {
                case KeyCode.Backspace:
                    return 0xff08;
                case KeyCode.Tab:
                    return 0xff09;
                case KeyCode.Return:
                    return 0xff0d;
                case KeyCode.Escape:
                    return 0xff1b;
                case KeyCode.Delete:
                    return 0xffff;
                case KeyCode.Insert:
                    return 0xff63;
                case KeyCode.Home:
                    return 0xff50;
                case KeyCode.End:
                    return 0xff57;
                case KeyCode.PageUp:
                    return 0xff55;
                case KeyCode.PageDown:
                    return 0xff56;
                case KeyCode.UpArrow:
                    return 0xff52;
                case KeyCode.DownArrow:
                    return 0xff54;
                case KeyCode.LeftArrow:
                    return 0xff51;
                case KeyCode.RightArrow:
                    return 0xff53;
                case KeyCode.Space:
                    return 0x0020;
                case KeyCode.Keypad0:
                    return 0xffb0;
                case KeyCode.Keypad1:
                    return 0xffb1;
                case KeyCode.Keypad2:
                    return 0xffb2;
                case KeyCode.Keypad3:
                    return 0xffb3;
                case KeyCode.Keypad4:
                    return 0xffb4;
                case KeyCode.Keypad5:
                    return 0xffb5;
                case KeyCode.Keypad6:
                    return 0xffb6;
                case KeyCode.Keypad7:
                    return 0xffb7;
                case KeyCode.Keypad8:
                    return 0xffb8;
                case KeyCode.Keypad9:
                    return 0xffb9;
                case KeyCode.KeypadMultiply:
                    return 0xffaa;
                case KeyCode.KeypadPlus:
                    return 0xffab;
                case KeyCode.KeypadMinus:
                    return 0xffad;
                case KeyCode.KeypadPeriod:
                    return 0xffae;
                case KeyCode.KeypadDivide:
                    return 0xffaf;
                case KeyCode.KeypadEnter:
                    return 0xff8d;
                case KeyCode.F1:
                    return 0xffbe;
                case KeyCode.F2:
                    return 0xffbf;
                case KeyCode.F3:
                    return 0xffc0;
                case KeyCode.F4:
                    return 0xffc1;
                case KeyCode.F5:
                    return 0xffc2;
                case KeyCode.F6:
                    return 0xffc3;
                case KeyCode.F7:
                    return 0xffc4;
                case KeyCode.F8:
                    return 0xffc5;
                case KeyCode.F9:
                    return 0xffc6;
                case KeyCode.F10:
                    return 0xffc7;
                case KeyCode.F11:
                    return 0xffc8;
                case KeyCode.F12:
                    return 0xffc9;
                case KeyCode.LeftShift:
                    return 0xffe1;
                case KeyCode.RightShift:
                    return 0xffe2;
                case KeyCode.LeftControl:
                    return 0xffe3;
                case KeyCode.RightControl:
                    return 0xffe4;
                case KeyCode.LeftAlt:
                    return 0xffe9;
                case KeyCode.RightAlt:
                    return 0xffea;
                case KeyCode.LeftWindows:
                    return 0xffeb;
                case KeyCode.RightWindows:
                    return 0xffec;
                case KeyCode.CapsLock:
                    return 0xffe5;
                case KeyCode.Numlock:
                    return 0xff7f;
                case KeyCode.ScrollLock:
                    return 0xff14;
                case KeyCode.Semicolon:
                    return 0x003b;
                case KeyCode.Equals:
                    return 0x003d;
                case KeyCode.Comma:
                    return 0x002c;
                case KeyCode.Minus:
                    return 0x002d;
                case KeyCode.Period:
                    return 0x002e;
                case KeyCode.Slash:
                    return 0x002f;
                case KeyCode.BackQuote:
                    return 0x0060;
                case KeyCode.LeftBracket:
                    return 0x005b;
                case KeyCode.Backslash:
                    return 0x005c;
                case KeyCode.RightBracket:
                    return 0x005d;
                case KeyCode.Quote:
                    return 0x0027;
                case KeyCode.Alpha0:
                    return 0x0030;
                case KeyCode.Alpha1:
                    return 0x0031;
                case KeyCode.Alpha2:
                    return 0x0032;
                case KeyCode.Alpha3:
                    return 0x0033;
                case KeyCode.Alpha4:
                    return 0x0034;
                case KeyCode.Alpha5:
                    return 0x0035;
                case KeyCode.Alpha6:
                    return 0x0036;
                case KeyCode.Alpha7:
                    return 0x0037;
                case KeyCode.Alpha8:
                    return 0x0038;
                case KeyCode.Alpha9:
                    return 0x0039;
                case KeyCode.A:
                    return 0x0061;
                case KeyCode.B:
                    return 0x0062;
                case KeyCode.C:
                    return 0x0063;
                case KeyCode.D:
                    return 0x0064;
                case KeyCode.E:
                    return 0x0065;
                case KeyCode.F:
                    return 0x0066;
                case KeyCode.G:
                    return 0x0067;
                case KeyCode.H:
                    return 0x0068;
                case KeyCode.I:
                    return 0x0069;
                case KeyCode.J:
                    return 0x006a;
                case KeyCode.K:
                    return 0x006b;
                case KeyCode.L:
                    return 0x006c;
                case KeyCode.M:
                    return 0x006d;
                case KeyCode.N:
                    return 0x006e;
                case KeyCode.O:
                    return 0x006f;
                case KeyCode.P:
                    return 0x0070;
                case KeyCode.Q:
                    return 0x0071;
                case KeyCode.R:
                    return 0x0072;
                case KeyCode.S:
                    return 0x0073;
                case KeyCode.T:
                    return 0x0074;
                case KeyCode.U:
                    return 0x0075;
                case KeyCode.V:
                    return 0x0076;
                case KeyCode.W:
                    return 0x0077;
                case KeyCode.X:
                    return 0x0078;
                case KeyCode.Y:
                    return 0x0079;
                case KeyCode.Z:
                    return 0x007a;
                default:
                    return 0;
            }
        }

        public static bool IsModifierKey(KeyCode keyCode)
        {
            switch (keyCode)
            {
                case KeyCode.LeftShift:
                case KeyCode.RightShift:
                case KeyCode.LeftControl:
                case KeyCode.RightControl:
                case KeyCode.LeftAlt:
                case KeyCode.RightAlt:
                case KeyCode.LeftWindows:
                case KeyCode.RightWindows:
                    return true;
                default:
                    return false;
            }
        }
    }

    public enum KeyEventType : byte
    {
        KeyDown = 1,
        KeyUp = 2,
        Char = 3,
    }

    [System.Flags]
    public enum KeyModifiers : byte
    {
        None = 0,
        Ctrl = 1,
        Shift = 2,
        Alt = 4,
    }

    public struct KeyboardState
    {
        public bool HasEvent;
        public KeyEventType Type;
        public int WindowsKeyCode;
        public int NativeKeyCode;
        public KeyModifiers Modifiers;
        public char Character;
    }
}
