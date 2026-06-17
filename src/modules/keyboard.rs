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

    /// Poll shared memory and forward any pending key event to the CEF browser host.
    #[cfg(feature = "cef")]
    pub fn poll(&mut self, host: &cef::BrowserHost) {
        use cef::{ImplBrowserHost, KeyEvent, KeyEventType};

        let Some(event) = self.read_event() else {
            return;
        };

        let type_ = match event.event_type {
            x if x == ProtocolKeyEventType::KeyDown as u8 => KeyEventType::KEYDOWN,
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
            windows_key_code: event.windows_key_code,
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
