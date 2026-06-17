//! Caret Module - Publishes the text-cursor screen position to Unity
//!
//! Protocol: 7 bytes
//! [flag(1), x(2), y(2), height(2)]
//! Direction: Browser → Unity
//!
//! The actual position is set by `OsrRenderHandler::on_ime_composition_range_changed`,
//! which fires whenever CEF updates the IME/selection rectangle.

use std::sync::{Arc, Mutex};

use anyhow::Result;
use log::debug;

use super::base::MemoryModuleBase;
use crate::ipc::SharedMemoryWrapper;

pub const CARET_PAYLOAD_SIZE: usize = 7;

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
        let mut raw = SharedMemoryWrapper::new(memory_name, CARET_PAYLOAD_SIZE);
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
        self.write_position_with_height(x, y, 0)
    }

    pub fn write_position_with_height(&self, x: i16, y: i16, height: i16) -> Result<()> {
        let bytes = encode_caret_payload(x, y, height);
        debug!("Caret position: ({}, {}, h={})", x, y, height);
        self.shmem
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))?
            .write_bytes(&bytes)
    }
}

pub fn parse_caret_console_payload(payload: &str) -> Option<(i16, i16, i16)> {
    let mut parts = payload.split(',');
    let x = parts.next()?.trim().parse::<i32>().ok()?;
    let y = parts.next()?.trim().parse::<i32>().ok()?;
    let height = parts.next()?.trim().parse::<i32>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((clamp_i16(x), clamp_i16(y), clamp_i16(height)))
}

pub fn encode_caret_payload(x: i16, y: i16, height: i16) -> [u8; CARET_PAYLOAD_SIZE] {
    let mut bytes = [0u8; CARET_PAYLOAD_SIZE];
    bytes[0] = 1;
    bytes[1..3].copy_from_slice(&x.to_le_bytes());
    bytes[3..5].copy_from_slice(&y.to_le_bytes());
    bytes[5..7].copy_from_slice(&height.to_le_bytes());
    bytes
}

fn clamp_i16(value: i32) -> i16 {
    value.clamp(i16::MIN as i32, i16::MAX as i32) as i16
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

#[cfg(test)]
mod tests {
    use super::{encode_caret_payload, parse_caret_console_payload};

    #[test]
    fn encodes_caret_payload_with_height() {
        let bytes = encode_caret_payload(12, 34, 16);
        assert_eq!(1, bytes[0]);
        assert_eq!(12, i16::from_le_bytes([bytes[1], bytes[2]]));
        assert_eq!(34, i16::from_le_bytes([bytes[3], bytes[4]]));
        assert_eq!(16, i16::from_le_bytes([bytes[5], bytes[6]]));
    }

    #[test]
    fn parses_console_payload_and_clamps_to_i16() {
        assert_eq!(
            Some((32767, -32768, 20)),
            parse_caret_console_payload("40000,-40000,20")
        );
        assert_eq!(None, parse_caret_console_payload("1,2"));
        assert_eq!(None, parse_caret_console_payload("1,2,3,4"));
    }
}
