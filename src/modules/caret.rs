//! Caret Module - Publishes the text-cursor screen position to Unity
//!
//! Protocol: 5 bytes
//! [flag(1), x(2), y(2)]
//! Direction: Browser → Unity
//!
//! The actual position is set by `OsrRenderHandler::on_ime_composition_range_changed`,
//! which fires whenever CEF updates the IME/selection rectangle.

use std::sync::{Arc, Mutex};

use anyhow::Result;
use log::debug;

use super::base::MemoryModuleBase;
use super::protocol::CaretPosition;
use crate::ipc::SharedMemoryWrapper;

/// Caret position tracking module.
///
/// Exposes `get_shmem()` so the render handler can write the position directly
/// from the CEF render thread.
pub struct CaretModule {
    memory_name: String,
    running: bool,
    shmem: Arc<Mutex<SharedMemoryWrapper>>,
}

impl CaretModule {
    pub fn new(memory_name: &str) -> Result<Self> {
        let mut raw = SharedMemoryWrapper::new(memory_name, CaretPosition::SIZE);
        raw.initialize()?;
        Ok(Self {
            memory_name: memory_name.to_string(),
            running: true,
            shmem: Arc::new(Mutex::new(raw)),
        })
    }

    /// Return a clone of the inner Arc so the render handler can share it.
    pub fn get_shmem(&self) -> Arc<Mutex<SharedMemoryWrapper>> {
        self.shmem.clone()
    }

    /// Write caret position directly (used in non-render-thread scenarios).
    pub fn write_position(&self, x: i16, y: i16) -> Result<()> {
        let pos = CaretPosition { flag: 1, x, y };
        debug!("Caret position: ({}, {})", x, y);
        self.shmem
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))?
            .write_bytes(&pos.to_bytes())
    }
}

impl MemoryModuleBase for CaretModule {
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
