//! IME Module - Handles Input Method Editor events from Unity
//!
//! Protocol: 2048 bytes max
//! [flag(1), operationType(1), textLength(2), cursorPos(2), text(variable, UTF-8)]

use anyhow::Result;
use log::{debug, warn};

use super::base::MemoryModuleBase;
use super::protocol::{ImeEvent, ImeOperationType};
use crate::ipc::SharedMemoryWrapper;

/// IME input module
pub struct ImeModule {
    memory_name: String,
    shmem: SharedMemoryWrapper,
    running: bool,
}

impl ImeModule {
    pub fn new(memory_name: &str) -> Result<Self> {
        let mut shmem = SharedMemoryWrapper::new(memory_name, ImeEvent::SIZE);
        shmem.initialize()?;
        Ok(Self {
            memory_name: memory_name.to_string(),
            shmem,
            running: true,
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
    pub fn poll(&mut self, host: &cef::BrowserHost) {
        use cef::{CefString, ImplBrowserHost, Range};

        let Some(event) = self.read_event() else {
            return;
        };

        // u32::MAX signals "no range" to CEF.
        let no_range = Range {
            from: u32::MAX,
            to: u32::MAX,
        };

        if event.operation_type == ImeOperationType::SetComposition as u8 {
            let text = String::from_utf8_lossy(&event.text);
            let cef_text = CefString::from(text.as_ref());
            let cursor = event.cursor_position.max(0) as u32;
            let selection = Range {
                from: cursor,
                to: cursor,
            };
            debug!("IME set composition: {:?}", text);
            // underlines = None  (no visual underline decoration)
            host.ime_set_composition(Some(&cef_text), None, Some(&no_range), Some(&selection));
        } else if event.operation_type == ImeOperationType::CommitText as u8 {
            let text = String::from_utf8_lossy(&event.text);
            let cef_text = CefString::from(text.as_ref());
            debug!("IME commit: {:?}", text);
            host.ime_commit_text(Some(&cef_text), Some(&no_range), 0);
        } else if event.operation_type == ImeOperationType::CancelComposition as u8 {
            debug!("IME cancel composition");
            host.ime_cancel_composition();
        } else {
            warn!("Unknown IME operation type: {}", event.operation_type);
        }
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
