//! Shared Memory Protocol Definitions
//!
//! This module defines the exact byte-level protocol for communication
//! between Unity and the HeadlessBrowser via shared memory.

/// Keyboard event types (matches C# KeyEventType enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum KeyEventType {
    KeyDown = 1,
    KeyUp = 2,
    Char = 3,
}

/// Key modifier flags (matches C# KeyModifiers)
pub mod key_modifiers {
    pub const NONE: u8 = 0;
    pub const CTRL: u8 = 0x01;
    pub const SHIFT: u8 = 0x02;
    pub const ALT: u8 = 0x04;
}

/// Mouse event types (matches C# MouseEventType enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MouseEventType {
    LeftDown = 1,
    LeftUp = 2,
    RightDown = 3,
    RightUp = 4,
    MiddleDown = 5,
    MiddleUp = 6,
    Scroll = 7,
}

/// IME operation types (matches C# ImeOperationType enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ImeOperationType {
    SetComposition = 1,
    CommitText = 2,
    CancelComposition = 3,
}

/// Keyboard event structure (16 bytes)
/// Protocol: [flag(1), eventType(1), windowsKeyCode(4), nativeKeyCode(4), modifiers(1), character(4), padding(1)]
#[derive(Debug, Clone)]
pub struct KeyboardEvent {
    pub flag: u8,
    pub event_type: u8,
    pub windows_key_code: i32,
    pub native_key_code: i32,
    pub modifiers: u8,
    pub character: i32,
    pub padding: u8,
}

impl KeyboardEvent {
    pub const SIZE: usize = 16;

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            flag: data[0],
            event_type: data[1],
            windows_key_code: i32::from_le_bytes([data[2], data[3], data[4], data[5]]),
            native_key_code: i32::from_le_bytes([data[6], data[7], data[8], data[9]]),
            modifiers: data[10],
            character: i32::from_le_bytes([data[11], data[12], data[13], data[14]]),
            padding: data[15],
        })
    }

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0] = self.flag;
        bytes[1] = self.event_type;
        bytes[2..6].copy_from_slice(&self.windows_key_code.to_le_bytes());
        bytes[6..10].copy_from_slice(&self.native_key_code.to_le_bytes());
        bytes[10] = self.modifiers;
        bytes[11..15].copy_from_slice(&self.character.to_le_bytes());
        bytes[15] = self.padding;
        bytes
    }
}

/// Mouse event structure (10 bytes max)
/// Protocol: [flag(1), eventType(1), x(2), y(2), deltaX(2), deltaY(2)]
#[derive(Debug, Clone)]
pub struct MouseEvent {
    pub flag: u8,
    pub event_type: u8,
    pub x: i16,
    pub y: i16,
    pub delta_x: i16,
    pub delta_y: i16,
}

impl MouseEvent {
    pub const SIZE: usize = 10;

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 6 {
            return None;
        }

        let delta_x = if data.len() >= 8 {
            i16::from_le_bytes([data[6], data[7]])
        } else {
            0
        };

        let delta_y = if data.len() >= 10 {
            i16::from_le_bytes([data[8], data[9]])
        } else {
            0
        };

        Some(Self {
            flag: data[0],
            event_type: data[1],
            x: i16::from_le_bytes([data[2], data[3]]),
            y: i16::from_le_bytes([data[4], data[5]]),
            delta_x,
            delta_y,
        })
    }

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0] = self.flag;
        bytes[1] = self.event_type;
        bytes[2..4].copy_from_slice(&self.x.to_le_bytes());
        bytes[4..6].copy_from_slice(&self.y.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.delta_x.to_le_bytes());
        bytes[8..10].copy_from_slice(&self.delta_y.to_le_bytes());
        bytes
    }
}

/// Mouse state structure (6 bytes)
/// Protocol: [flag(1), x(2), y(2), buttonState(1)]
#[derive(Debug, Clone)]
pub struct MouseState {
    pub flag: u8,
    pub x: i16,
    pub y: i16,
    pub button_state: u8,
}

impl MouseState {
    pub const SIZE: usize = 6;

    pub const BUTTON_LEFT: u8 = 0x01;
    pub const BUTTON_RIGHT: u8 = 0x02;
    pub const BUTTON_MIDDLE: u8 = 0x04;

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            flag: data[0],
            x: i16::from_le_bytes([data[1], data[2]]),
            y: i16::from_le_bytes([data[3], data[4]]),
            button_state: data[5],
        })
    }

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0] = self.flag;
        bytes[1..3].copy_from_slice(&self.x.to_le_bytes());
        bytes[3..5].copy_from_slice(&self.y.to_le_bytes());
        bytes[5] = self.button_state;
        bytes
    }
}

/// IME event structure (2048 bytes max)
/// Protocol: [flag(1), operationType(1), textLength(2), cursorPos(2), text(variable)]
#[derive(Debug, Clone)]
pub struct ImeEvent {
    pub flag: u8,
    pub operation_type: u8,
    pub text_length: i16,
    pub cursor_position: i16,
    pub text: Vec<u8>,
}

impl ImeEvent {
    pub const SIZE: usize = 2048;
    pub const MAX_TEXT_LENGTH: usize = 2040;

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 6 {
            return None;
        }

        let text_length = i16::from_le_bytes([data[2], data[3]]) as usize;
        let cursor_position = i16::from_le_bytes([data[4], data[5]]);
        let text = if text_length > 0 && data.len() >= 6 + text_length {
            data[6..6 + text_length].to_vec()
        } else {
            Vec::new()
        };

        Some(Self {
            flag: data[0],
            operation_type: data[1],
            text_length: text_length as i16,
            cursor_position,
            text,
        })
    }
}

/// Caret position structure (5 bytes)
/// Protocol: [flag(1), x(2), y(2)]
#[derive(Debug, Clone)]
pub struct CaretPosition {
    pub flag: u8,
    pub x: i16,
    pub y: i16,
}

impl CaretPosition {
    pub const SIZE: usize = 5;

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0] = self.flag;
        bytes[1..3].copy_from_slice(&self.x.to_le_bytes());
        bytes[3..5].copy_from_slice(&self.y.to_le_bytes());
        bytes
    }
}

/// Script execution request (10005 bytes max)
/// Protocol: [flag(1), scriptLength(2), reserved(2), script(variable)]
#[derive(Debug, Clone)]
pub struct ScriptRequest {
    pub flag: u8,
    pub script_length: i16,
    pub script: Vec<u8>,
}

impl ScriptRequest {
    pub const SIZE: usize = 10005;
    pub const MAX_SCRIPT_LENGTH: usize = 10000;

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 5 {
            return None;
        }

        let script_length = i16::from_le_bytes([data[1], data[2]]) as usize;
        let script = if script_length > 0 && data.len() >= 5 + script_length {
            data[5..5 + script_length].to_vec()
        } else {
            Vec::new()
        };

        Some(Self {
            flag: data[0],
            script_length: script_length as i16,
            script,
        })
    }
}

/// Handler management command written by Unity `BrowserStatic`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerCommand {
    Shutdown,
    AddBrowser {
        guid: String,
        width: i32,
        height: i32,
        address: String,
    },
    RemoveBrowser {
        guid: String,
    },
    ResizeBrowser {
        guid: String,
        width: i32,
        height: i32,
    },
}

impl HandlerCommand {
    const GUID_OFFSET: usize = 2;
    const GUID_LEN: usize = 36;
    const WIDTH_OFFSET: usize = Self::GUID_OFFSET + Self::GUID_LEN;
    const HEIGHT_OFFSET: usize = Self::WIDTH_OFFSET + 2;
    const ADDRESS_LENGTH_OFFSET: usize = Self::HEIGHT_OFFSET + 2;
    const ADDRESS_OFFSET: usize = Self::ADDRESS_LENGTH_OFFSET + 2;

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 2 || data[0] != 1 {
            return None;
        }

        match data[1] {
            0 => Some(Self::Shutdown),
            1 => {
                let guid = parse_guid(data)?;
                let width = read_i16(data, Self::WIDTH_OFFSET)? as i32;
                let height = read_i16(data, Self::HEIGHT_OFFSET)? as i32;
                let address_length = read_i16(data, Self::ADDRESS_LENGTH_OFFSET)? as usize;
                let end = Self::ADDRESS_OFFSET.checked_add(address_length)?;
                if end > data.len() {
                    return None;
                }
                let address = String::from_utf8_lossy(&data[Self::ADDRESS_OFFSET..end])
                    .trim_end_matches('\0')
                    .to_string();
                Some(Self::AddBrowser {
                    guid,
                    width,
                    height,
                    address,
                })
            }
            2 => Some(Self::RemoveBrowser {
                guid: parse_guid(data)?,
            }),
            3 => Some(Self::ResizeBrowser {
                guid: parse_guid(data)?,
                width: read_i16(data, Self::WIDTH_OFFSET)? as i32,
                height: read_i16(data, Self::HEIGHT_OFFSET)? as i32,
            }),
            _ => None,
        }
    }
}

fn parse_guid(data: &[u8]) -> Option<String> {
    let end = HandlerCommand::GUID_OFFSET.checked_add(HandlerCommand::GUID_LEN)?;
    if end > data.len() {
        return None;
    }

    Some(
        String::from_utf8_lossy(&data[HandlerCommand::GUID_OFFSET..end])
            .trim_end_matches('\0')
            .to_string(),
    )
}

fn read_i16(data: &[u8], offset: usize) -> Option<i16> {
    if offset + 2 > data.len() {
        return None;
    }

    Some(i16::from_le_bytes([data[offset], data[offset + 1]]))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeartbeatPayload {
    pub sequence: i64,
    pub utc_ticks: i64,
}

impl HeartbeatPayload {
    pub const SIZE: usize = 16;

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            sequence: i64::from_le_bytes([
                data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
            ]),
            utc_ticks: i64::from_le_bytes([
                data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
            ]),
        })
    }
}

/// Capture frame header
/// Protocol: [width(4), height(4), pixels(BGRA)]
#[derive(Debug, Clone)]
pub struct CaptureFrame {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<u8>,
}

impl CaptureFrame {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + self.pixels.len());
        bytes.extend_from_slice(&self.width.to_le_bytes());
        bytes.extend_from_slice(&self.height.to_le_bytes());
        bytes.extend_from_slice(&self.pixels);
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::{HandlerCommand, HeartbeatPayload};

    fn guid() -> String {
        "12345678-1234-1234-1234-123456789abc".to_string()
    }

    #[test]
    fn parses_unity_add_browser_command() {
        let mut bytes = vec![0; 3000];
        bytes[0] = 1;
        bytes[1] = 1;
        bytes[2..38].copy_from_slice(guid().as_bytes());
        bytes[38..40].copy_from_slice(&800i16.to_le_bytes());
        bytes[40..42].copy_from_slice(&600i16.to_le_bytes());
        let address = b"https://example.test";
        bytes[42..44].copy_from_slice(&(address.len() as i16).to_le_bytes());
        bytes[44..44 + address.len()].copy_from_slice(address);

        let command = HandlerCommand::from_bytes(&bytes).unwrap();

        assert_eq!(
            command,
            HandlerCommand::AddBrowser {
                guid: guid(),
                width: 800,
                height: 600,
                address: "https://example.test".to_string(),
            }
        );
    }

    #[test]
    fn parses_unity_resize_browser_command() {
        let mut bytes = vec![0; 44];
        bytes[0] = 1;
        bytes[1] = 3;
        bytes[2..38].copy_from_slice(guid().as_bytes());
        bytes[38..40].copy_from_slice(&1024i16.to_le_bytes());
        bytes[40..42].copy_from_slice(&768i16.to_le_bytes());

        let command = HandlerCommand::from_bytes(&bytes).unwrap();

        assert_eq!(
            command,
            HandlerCommand::ResizeBrowser {
                guid: guid(),
                width: 1024,
                height: 768,
            }
        );
    }

    #[test]
    fn parses_unity_remove_and_shutdown_commands() {
        let mut remove = vec![0; 3000];
        remove[0] = 1;
        remove[1] = 2;
        remove[2..38].copy_from_slice(guid().as_bytes());

        assert_eq!(
            HandlerCommand::from_bytes(&remove).unwrap(),
            HandlerCommand::RemoveBrowser { guid: guid() }
        );
        assert_eq!(
            HandlerCommand::from_bytes(&[1, 0]).unwrap(),
            HandlerCommand::Shutdown
        );
    }

    #[test]
    fn parses_unity_heartbeat_payload() {
        let mut bytes = vec![0; HeartbeatPayload::SIZE];
        bytes[0..8].copy_from_slice(&42i64.to_le_bytes());
        bytes[8..16].copy_from_slice(&638858000000000000i64.to_le_bytes());

        let payload = HeartbeatPayload::from_bytes(&bytes).unwrap();

        assert_eq!(
            payload,
            HeartbeatPayload {
                sequence: 42,
                utc_ticks: 638858000000000000,
            }
        );
    }
}
