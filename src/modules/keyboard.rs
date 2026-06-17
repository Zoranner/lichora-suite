//! Keyboard Module - Handles keyboard input from Unity via shared memory
//!
//! Protocol: 16 bytes
//! [flag(1), eventType(1), windowsKeyCode(4), nativeKeyCode(4), modifiers(1), character(4), padding(1)]

use anyhow::Result;
use log::{debug, warn};

use super::base::MemoryModuleBase;
use super::protocol::{KeyEventType as ProtocolKeyEventType, KeyboardEvent};
use crate::ipc::SharedMemoryWrapper;

// CEF event-flag bit values (match cef_event_flags_t constants)
const EVENTFLAG_SHIFT_DOWN: u32 = 2;
const EVENTFLAG_CONTROL_DOWN: u32 = 4;
const EVENTFLAG_ALT_DOWN: u32 = 8;

/// Keyboard input module
pub struct KeyboardModule {
    memory_name: String,
    shmem: SharedMemoryWrapper,
    running: bool,
}

impl KeyboardModule {
    pub fn new(memory_name: &str) -> Result<Self> {
        let mut shmem = SharedMemoryWrapper::new(memory_name, KeyboardEvent::SIZE);
        shmem.initialize()?;
        Ok(Self {
            memory_name: memory_name.to_string(),
            shmem,
            running: true,
        })
    }

    fn read_event(&mut self) -> Option<KeyboardEvent> {
        let data = self.shmem.read_bytes().ok()?;
        if data.len() < KeyboardEvent::SIZE || data[0] != 1 {
            return None;
        }
        let event = KeyboardEvent::from_bytes(&data)?;
        let _ = self.shmem.write_byte_at(0, 0);
        Some(event)
    }

    fn should_update_after_event(event: &KeyboardEvent) -> bool {
        if event.event_type == ProtocolKeyEventType::Char as u8 {
            return true;
        }
        event.event_type == ProtocolKeyEventType::KeyUp as u8
            && should_update_caret_position(event.windows_key_code, event.modifiers)
    }

    /// Poll shared memory and forward any pending key event to the CEF browser host.
    #[cfg(feature = "cef")]
    pub fn poll(&mut self, host: &cef::BrowserHost) -> bool {
        let Some(event) = self.read_event() else {
            return false;
        };
        let should_update = Self::should_update_after_event(&event);
        self.send_event(host, &event);
        should_update
    }

    #[cfg(feature = "cef")]
    fn send_event(&self, host: &cef::BrowserHost, event: &KeyboardEvent) {
        use cef::{ImplBrowserHost, KeyEvent, KeyEventType};
        let type_ = match event.event_type {
            x if x == ProtocolKeyEventType::KeyDown as u8 => KeyEventType::RAWKEYDOWN,
            x if x == ProtocolKeyEventType::KeyUp as u8 => KeyEventType::KEYUP,
            x if x == ProtocolKeyEventType::Char as u8 => KeyEventType::CHAR,
            other => {
                warn!("Unknown key event type: {}", other);
                return;
            }
        };

        let mut modifiers: u32 = 0;
        if event.modifiers & 0x01 != 0 {
            modifiers |= EVENTFLAG_CONTROL_DOWN;
        }
        if event.modifiers & 0x02 != 0 {
            modifiers |= EVENTFLAG_SHIFT_DOWN;
        }
        if event.modifiers & 0x04 != 0 {
            modifiers |= EVENTFLAG_ALT_DOWN;
        }

        let cef_event = KeyEvent {
            size: std::mem::size_of::<KeyEvent>(),
            type_,
            modifiers,
            windows_key_code: if type_ == KeyEventType::CHAR {
                event.character
            } else {
                event.windows_key_code
            },
            native_key_code: event.native_key_code,
            is_system_key: 0,
            character: event.character as u16,
            unmodified_character: event.character as u16,
            focus_on_editable_field: 0,
        };

        debug!(
            "Key event: type={:?}, code={}",
            type_, event.windows_key_code
        );
        host.send_key_event(Some(&cef_event));
    }

    #[cfg(feature = "cef")]
    pub fn send_backspaces(&self, host: &cef::BrowserHost, count: i16) {
        for _ in 0..count.max(0) {
            self.send_key_down(host, 0x08, 0x08);
            self.send_char_event(host, '\u{0008}');
            self.send_key_up(host, 0x08, 0x08);
        }
    }

    #[cfg(feature = "cef")]
    pub fn send_deletes(&self, host: &cef::BrowserHost, count: i16) {
        for _ in 0..count.max(0) {
            self.send_key_down(host, 0x2E, 0x2E);
            self.send_key_up(host, 0x2E, 0x2E);
        }
    }

    #[cfg(feature = "cef")]
    pub fn send_char_event(&self, host: &cef::BrowserHost, character: char) {
        use cef::{ImplBrowserHost, KeyEvent, KeyEventType};
        let character = character as u16;
        let event = KeyEvent {
            size: std::mem::size_of::<KeyEvent>(),
            type_: KeyEventType::CHAR,
            modifiers: 0,
            windows_key_code: character as i32,
            native_key_code: 0,
            is_system_key: 0,
            character,
            unmodified_character: character,
            focus_on_editable_field: 0,
        };
        host.send_key_event(Some(&event));
    }

    #[cfg(feature = "cef")]
    pub fn send_key_down(
        &self,
        host: &cef::BrowserHost,
        windows_key_code: i32,
        native_key_code: i32,
    ) {
        self.send_key_event_with_type(
            host,
            cef::KeyEventType::RAWKEYDOWN,
            windows_key_code,
            native_key_code,
        );
    }

    #[cfg(feature = "cef")]
    pub fn send_key_up(
        &self,
        host: &cef::BrowserHost,
        windows_key_code: i32,
        native_key_code: i32,
    ) {
        self.send_key_event_with_type(
            host,
            cef::KeyEventType::KEYUP,
            windows_key_code,
            native_key_code,
        );
    }

    #[cfg(feature = "cef")]
    fn send_key_event_with_type(
        &self,
        host: &cef::BrowserHost,
        type_: cef::KeyEventType,
        windows_key_code: i32,
        native_key_code: i32,
    ) {
        use cef::{ImplBrowserHost, KeyEvent};
        let event = KeyEvent {
            size: std::mem::size_of::<KeyEvent>(),
            type_,
            modifiers: 0,
            windows_key_code,
            native_key_code,
            is_system_key: 0,
            character: 0,
            unmodified_character: 0,
            focus_on_editable_field: 0,
        };
        host.send_key_event(Some(&event));
    }
}

pub fn should_update_caret_position(windows_key_code: i32, modifiers: u8) -> bool {
    match windows_key_code {
        0x25 | 0x26 | 0x27 | 0x28 | 0x24 | 0x23 | 0x21 | 0x22 | 0x09 | 0x08 | 0x2E | 0x0D => {
            return true;
        }
        _ => {}
    }
    if modifiers & super::protocol::key_modifiers::CTRL != 0 {
        return matches!(windows_key_code, 0x41 | 0x56 | 0x58 | 0x5A | 0x59);
    }
    false
}

impl MemoryModuleBase for KeyboardModule {
    fn get_memory_name(&self) -> &str {
        &self.memory_name
    }
    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }
    fn shutdown(&mut self) {
        self.running = false;
    }
    fn is_running(&self) -> bool {
        self.running
    }
}
