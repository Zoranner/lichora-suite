//! Capture Module - Writes rendered frames to shared memory for Unity
//!
//! Protocol: [width(4), height(4), pixels(BGRA, width*height*4)]
//! Direction: Browser → Unity
//! Max resolution: 3840x2160

use std::sync::{Arc, Mutex};

use anyhow::{ensure, Result};
use log::debug;

use super::base::MemoryModuleBase;
use super::protocol::CaptureFrame;
use crate::ipc::SharedMemoryWrapper;

const MAX_WIDTH: usize = 2560;
const MAX_HEIGHT: usize = 1440;
const BYTES_PER_PIXEL: usize = 4;
const CAPTURE_SLOT_COUNT: usize = 3;
const CAPTURE_HEADER_SIZE: usize = 32;
const MAX_PIXEL_BUFFER_SIZE: usize = MAX_WIDTH * MAX_HEIGHT * BYTES_PER_PIXEL;
const CAPTURE_HEADER_OFFSET: usize = MAX_PIXEL_BUFFER_SIZE * CAPTURE_SLOT_COUNT;
const CAPTURE_FRAME_TYPE_FULL: i32 = 0;

/// Screen capture module.
///
/// Holds an `Arc<Mutex<SharedMemoryWrapper>>` so the same underlying buffer
/// can be written from the CEF render thread (via `OsrRenderHandler`) while
/// `BrowserEntry` retains the Arc for lifetime management.
pub struct CaptureModule {
    memory_name: String,
    running: bool,
    width: i32,
    height: i32,
    shmem: Arc<Mutex<SharedMemoryWrapper>>,
    slot: i32,
    sequence: i32,
}

impl CaptureModule {
    pub fn new(memory_name: &str, width: i32, height: i32) -> Result<Self> {
        let size = Self::capture_v2_size();
        let mut raw = SharedMemoryWrapper::new(memory_name, size);
        raw.initialize()?;
        Ok(Self {
            memory_name: memory_name.to_string(),
            running: true,
            width,
            height,
            shmem: Arc::new(Mutex::new(raw)),
            slot: 0,
            sequence: 0,
        })
    }

    pub fn new_shared(
        memory_name: &str,
        width: i32,
        height: i32,
    ) -> Result<Arc<Mutex<CaptureModule>>> {
        Ok(Arc::new(Mutex::new(Self::new(memory_name, width, height)?)))
    }

    pub const fn capture_header_offset() -> usize {
        CAPTURE_HEADER_OFFSET
    }

    pub const fn capture_v2_size() -> usize {
        CAPTURE_HEADER_OFFSET + CAPTURE_HEADER_SIZE
    }

    /// Return a clone of the inner Arc so the render handler can share it.
    pub fn get_shmem(&self) -> Arc<Mutex<SharedMemoryWrapper>> {
        self.shmem.clone()
    }

    /// Write a captured frame directly (used in non-CEF / test scenarios).
    pub fn write_frame(&self, frame: &CaptureFrame) -> Result<()> {
        let data = frame.to_bytes();
        debug!(
            "Write frame: {}x{}, {} bytes",
            frame.width,
            frame.height,
            data.len()
        );
        self.shmem
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))?
            .write_bytes(&data)
    }

    pub fn write_full_frame(&mut self, width: i32, height: i32, pixels: &[u8]) -> Result<()> {
        ensure!(width > 0 && height > 0, "invalid capture size");
        let pixel_data_size = width as usize * height as usize * BYTES_PER_PIXEL;
        ensure!(
            pixel_data_size == pixels.len(),
            "pixel buffer length does not match capture size"
        );
        ensure!(
            pixel_data_size <= MAX_PIXEL_BUFFER_SIZE,
            "pixel buffer exceeds capture slot size"
        );

        self.slot = (self.slot + 1) % CAPTURE_SLOT_COUNT as i32;
        self.sequence = self.sequence.wrapping_add(1);
        let slot_offset = self.slot as usize * MAX_PIXEL_BUFFER_SIZE;

        let mut shmem = self
            .shmem
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))?;
        shmem.write_payload_at(slot_offset, pixels)?;
        let ack_sequence = shmem.read_i32_at(CAPTURE_HEADER_OFFSET + 28).unwrap_or(0);
        let mut header = [0u8; CAPTURE_HEADER_SIZE];
        write_i32(&mut header, 0, width);
        write_i32(&mut header, 4, height);
        write_i32(&mut header, 8, self.slot);
        write_i32(&mut header, 12, self.sequence);
        write_i32(&mut header, 16, CAPTURE_FRAME_TYPE_FULL);
        write_i32(&mut header, 20, 0);
        write_i32(&mut header, 24, pixel_data_size as i32);
        write_i32(&mut header, 28, ack_sequence);
        shmem.write_payload_at(CAPTURE_HEADER_OFFSET, &header)?;
        shmem.write_length(Self::capture_v2_size())?;
        Ok(())
    }

    /// Update the expected resolution (does not resize shared memory).
    pub fn set_resolution(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;
    }

    pub fn get_buffer_size(&self) -> usize {
        8 + (self.width * self.height * 4) as usize
    }
}

fn write_i32(buffer: &mut [u8], offset: usize, value: i32) {
    buffer[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

impl MemoryModuleBase for CaptureModule {
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
    use super::{CaptureModule, MAX_PIXEL_BUFFER_SIZE};
    use crate::ipc::SharedMemoryWrapper;

    #[test]
    fn writes_capture_v2_full_frame_header_and_slot_payload() {
        let name = format!("Capture.{}", uuid::Uuid::new_v4());
        let mut module = CaptureModule::new(&name, 2, 2).unwrap();
        let pixels = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

        module.write_full_frame(2, 2, &pixels).unwrap();

        let reader = SharedMemoryWrapper::new(&name, CaptureModule::capture_v2_size());
        let bytes = std::fs::read(reader.file_path()).unwrap();
        let payload = &bytes[4..];
        let header_offset = CaptureModule::capture_header_offset();
        let slot_offset = MAX_PIXEL_BUFFER_SIZE;
        assert_eq!(&payload[slot_offset..slot_offset + 16], pixels.as_slice());
        assert_eq!(read_i32(payload, header_offset), 2);
        assert_eq!(read_i32(payload, header_offset + 4), 2);
        assert_eq!(read_i32(payload, header_offset + 8), 1);
        assert_eq!(read_i32(payload, header_offset + 12), 1);
        assert_eq!(read_i32(payload, header_offset + 16), 0);
        assert_eq!(read_i32(payload, header_offset + 20), 0);
        assert_eq!(read_i32(payload, header_offset + 24), 16);
        assert_eq!(read_i32(payload, header_offset + 28), 0);
    }

    fn read_i32(bytes: &[u8], offset: usize) -> i32 {
        i32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    }
}
