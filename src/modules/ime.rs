//! IME Module - Handles Input Method Editor events from Unity
//!
//! Protocol: 2048 bytes max
//! [flag(1), operationType(1), textLength(2), cursorPos(2), text(variable, UTF-8)]

use anyhow::Result;
use log::{debug, warn};

use super::base::MemoryModuleBase;
use super::keyboard::KeyboardModule;
use super::protocol::{ImeEvent, ImeOperationType};
use crate::ipc::SharedMemoryWrapper;

const IME_DELETE_SURROUNDING_TEXT: u8 = 4;

/// IME input module
pub struct ImeModule {
    memory_name: String,
    shmem: SharedMemoryWrapper,
    running: bool,
    current_composition_length: i16,
}

impl ImeModule {
    pub fn new(memory_name: &str) -> Result<Self> {
        let mut shmem = SharedMemoryWrapper::new(memory_name, ImeEvent::SIZE);
        shmem.initialize()?;
        Ok(Self {
            memory_name: memory_name.to_string(),
            shmem,
            running: true,
            current_composition_length: 0,
        })
    }

    fn read_event(&mut self) -> Option<ImeEvent> {
        let data = self.shmem.read_bytes().ok()?;
        if data.len() < 6 || data[0] != 1 {
            return None;
        }
        let event = ImeEvent::from_bytes(&data)?;
        let _ = self.shmem.write_byte_at(0, 0);
        Some(event)
    }

    /// Poll shared memory and forward any pending IME event to the CEF browser host.
    #[cfg(feature = "cef")]
    pub fn poll(&mut self, host: &cef::BrowserHost, keyboard: &KeyboardModule) -> bool {
        let Some(event) = self.read_event() else {
            return false;
        };

        if event.operation_type == ImeOperationType::SetComposition as u8 {
            let text = String::from_utf8_lossy(&event.text);
            debug!("IME set composition: {:?}", text);
            keyboard.send_backspaces(host, self.current_composition_length);
            for character in text.chars() {
                keyboard.send_char_event(host, character);
            }
            self.current_composition_length = text.chars().count().min(i16::MAX as usize) as i16;
            return true;
        } else if event.operation_type == ImeOperationType::CommitText as u8 {
            let text = String::from_utf8_lossy(&event.text);
            debug!("IME commit: {:?}", text);
            keyboard.send_backspaces(host, self.current_composition_length);
            self.current_composition_length = 0;
            for character in text.chars() {
                match character {
                    '\n' | '\r' => {
                        keyboard.send_key_down(host, 0x0D, 0x0D);
                        keyboard.send_char_event(host, '\r');
                        keyboard.send_key_up(host, 0x0D, 0x0D);
                    }
                    '\t' => {
                        keyboard.send_key_down(host, 0x09, 0x09);
                        keyboard.send_char_event(host, '\t');
                        keyboard.send_key_up(host, 0x09, 0x09);
                    }
                    _ => keyboard.send_char_event(host, character),
                }
            }
            return true;
        } else if event.operation_type == ImeOperationType::CancelComposition as u8 {
            debug!("IME cancel composition");
            keyboard.send_backspaces(host, self.current_composition_length);
            self.current_composition_length = 0;
            return true;
        } else if event.operation_type == IME_DELETE_SURROUNDING_TEXT {
            let before = event.text_length;
            let after = event.cursor_position;
            if before < 0 || after < 0 {
                warn!(
                    "Invalid IME DeleteSurroundingText range: before={}, after={}",
                    before, after
                );
                return false;
            }
            keyboard.send_backspaces(host, self.current_composition_length);
            self.current_composition_length = 0;
            keyboard.send_backspaces(host, before);
            keyboard.send_deletes(host, after);
            debug!(
                "IME delete surrounding text: before={}, after={}",
                before, after
            );
            return true;
        } else {
            warn!("Unknown IME operation type: {}", event.operation_type);
        }
        false
    }
}

impl MemoryModuleBase for ImeModule {
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
