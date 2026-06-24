//! Capture Module - Writes rendered frames to shared memory for Unity
//!
//! Protocol: Capture v2 three-slot pixel ring plus a 32-byte commit header.
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
const CAPTURE_RECT_HEADER_SIZE: usize = 16;
const MAX_PIXEL_BUFFER_SIZE: usize = MAX_WIDTH * MAX_HEIGHT * BYTES_PER_PIXEL;
const CAPTURE_HEADER_OFFSET: usize = MAX_PIXEL_BUFFER_SIZE * CAPTURE_SLOT_COUNT;
const CAPTURE_FRAME_TYPE_FULL: i32 = 0;
const CAPTURE_FRAME_TYPE_DIRTY: i32 = 1;
const MAX_DIRTY_RECT_COUNT: usize = 64;
const FULL_FRAME_DIRTY_AREA_PERCENT: i64 = 70;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DirtyRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

/// Screen capture module.
///
/// Holds an `Arc<Mutex<SharedMemoryWrapper>>` so the same underlying buffer
/// can be written from the CEF render thread (via `OsrRenderHandler`) while
/// `BrowserEntry` retains the Arc for lifetime management.
pub struct CaptureModule {
    memory_name: String,
    capture_lock: CaptureLock,
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
            capture_lock: CaptureLock::new(memory_name),
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

    pub fn write_paint_frame(
        &mut self,
        width: i32,
        height: i32,
        pixels: &[u8],
        dirty_rects: &[(i32, i32, i32, i32)],
    ) -> Result<bool> {
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

        let next_slot = (self.slot + 1) % CAPTURE_SLOT_COUNT as i32;
        let next_sequence = self.sequence.wrapping_add(1);
        let previous_sequence = self.sequence;
        let slot_offset = next_slot as usize * MAX_PIXEL_BUFFER_SIZE;
        let size_changed = width != self.width || height != self.height;

        let Some(_capture_guard) = self.capture_lock.try_lock() else {
            return Ok(false);
        };
        let mut shmem = self
            .shmem
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))?;
        let ack_sequence = shmem.read_i32_at(CAPTURE_HEADER_OFFSET + 28).unwrap_or(0);
        let previous_frame_acked = previous_sequence == 0 || ack_sequence == previous_sequence;
        if !previous_frame_acked && !size_changed {
            return Ok(false);
        }

        let dirty_payload = if previous_frame_acked && previous_sequence > 0 {
            build_dirty_payload(width, height, pixels, dirty_rects, size_changed)
        } else {
            None
        };
        let (frame_type, rect_count, payload_size) = if let Some(payload) = dirty_payload
            .as_ref()
            .filter(|payload| !payload.rects.is_empty())
        {
            shmem.write_payload_at_unflushed(slot_offset, &payload.bytes)?;
            (
                CAPTURE_FRAME_TYPE_DIRTY,
                payload.rects.len() as i32,
                payload.bytes.len() as i32,
            )
        } else {
            shmem.write_payload_at_unflushed(slot_offset, pixels)?;
            (CAPTURE_FRAME_TYPE_FULL, 0, pixel_data_size as i32)
        };

        let mut header = [0u8; CAPTURE_HEADER_SIZE - 4];
        write_i32(&mut header, 0, width);
        write_i32(&mut header, 4, height);
        write_i32(&mut header, 8, next_slot);
        write_i32(&mut header, 12, next_sequence);
        write_i32(&mut header, 16, frame_type);
        write_i32(&mut header, 20, rect_count);
        write_i32(&mut header, 24, payload_size);
        shmem.write_payload_at_unflushed(CAPTURE_HEADER_OFFSET, &header)?;
        shmem.write_length_unflushed(Self::capture_v2_size())?;
        shmem.flush_async()?;
        self.slot = next_slot;
        self.sequence = next_sequence;
        self.width = width;
        self.height = height;
        Ok(true)
    }

    pub fn write_full_frame(&mut self, width: i32, height: i32, pixels: &[u8]) -> Result<()> {
        self.write_paint_frame(width, height, pixels, &[])?;
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

struct CaptureLock {
    name: String,
}

impl CaptureLock {
    fn new(memory_name: &str) -> Self {
        Self {
            name: format!("MemoryStacks_CaptureLock_{}", sanitize_name(memory_name)),
        }
    }

    fn try_lock(&self) -> Option<CaptureLockGuard> {
        try_lock_capture(&self.name)
    }
}

struct CaptureLockGuard {
    #[cfg(target_os = "windows")]
    handle: winapi::shared::ntdef::HANDLE,
    #[cfg(target_os = "linux")]
    file: std::fs::File,
}

#[cfg(target_os = "windows")]
fn try_lock_capture(name: &str) -> Option<CaptureLockGuard> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use winapi::um::synchapi::{CreateMutexW, WaitForSingleObject};
    use winapi::um::winbase::{WAIT_ABANDONED, WAIT_OBJECT_0};

    let wide_name: Vec<u16> = OsStr::new(name).encode_wide().chain(Some(0)).collect();
    let handle = unsafe { CreateMutexW(ptr::null_mut(), 0, wide_name.as_ptr()) };
    if handle.is_null() {
        return None;
    }
    let wait = unsafe { WaitForSingleObject(handle, 0) };
    if wait != WAIT_OBJECT_0 && wait != WAIT_ABANDONED {
        unsafe {
            winapi::um::handleapi::CloseHandle(handle);
        }
        return None;
    }
    Some(CaptureLockGuard { handle })
}

#[cfg(target_os = "linux")]
fn try_lock_capture(name: &str) -> Option<CaptureLockGuard> {
    use std::os::fd::AsRawFd;

    let path = std::env::temp_dir().join(format!("{name}.lock"));
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(path)
        .ok()?;
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    (result == 0).then_some(CaptureLockGuard { file })
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn try_lock_capture(_name: &str) -> Option<CaptureLockGuard> {
    Some(CaptureLockGuard {})
}

impl Drop for CaptureLockGuard {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        unsafe {
            winapi::um::synchapi::ReleaseMutex(self.handle);
            winapi::um::handleapi::CloseHandle(self.handle);
        }

        #[cfg(target_os = "linux")]
        unsafe {
            use std::os::fd::AsRawFd;
            libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

fn sanitize_name(name: &str) -> String {
    if name.trim().is_empty() {
        return "default".to_string();
    }
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn write_i32(buffer: &mut [u8], offset: usize, value: i32) {
    buffer[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

struct DirtyPayload {
    rects: Vec<DirtyRect>,
    bytes: Vec<u8>,
}

fn build_dirty_payload(
    width: i32,
    height: i32,
    pixels: &[u8],
    dirty_rects: &[(i32, i32, i32, i32)],
    size_changed: bool,
) -> Option<DirtyPayload> {
    if size_changed
        || width <= 0
        || height <= 0
        || dirty_rects.is_empty()
        || dirty_rects.len() > MAX_DIRTY_RECT_COUNT
    {
        return None;
    }

    let mut rects = Vec::with_capacity(dirty_rects.len());
    let mut payload_size = 0usize;
    let mut dirty_area = 0i64;
    let full_area = width as i64 * height as i64;
    for &(x, y, rect_width, rect_height) in dirty_rects {
        let rect = clip_dirty_rect(width, height, x, y, rect_width, rect_height)?;
        let pixel_bytes = rect.width as usize * rect.height as usize * BYTES_PER_PIXEL;
        payload_size = payload_size.checked_add(CAPTURE_RECT_HEADER_SIZE + pixel_bytes)?;
        dirty_area += rect.width as i64 * rect.height as i64;
        rects.push(rect);
    }

    if payload_size > MAX_PIXEL_BUFFER_SIZE
        || dirty_area * 100 >= full_area * FULL_FRAME_DIRTY_AREA_PERCENT
    {
        return None;
    }

    let mut bytes = Vec::with_capacity(payload_size);
    for rect in &rects {
        bytes.extend_from_slice(&rect.x.to_le_bytes());
        bytes.extend_from_slice(&rect.y.to_le_bytes());
        bytes.extend_from_slice(&rect.width.to_le_bytes());
        bytes.extend_from_slice(&rect.height.to_le_bytes());

        let row_bytes = rect.width as usize * BYTES_PER_PIXEL;
        for row in 0..rect.height as usize {
            let source_offset =
                ((rect.y as usize + row) * width as usize + rect.x as usize) * BYTES_PER_PIXEL;
            let source_end = source_offset + row_bytes;
            bytes.extend_from_slice(&pixels[source_offset..source_end]);
        }
    }

    Some(DirtyPayload { rects, bytes })
}

fn clip_dirty_rect(
    width: i32,
    height: i32,
    x: i32,
    y: i32,
    rect_width: i32,
    rect_height: i32,
) -> Option<DirtyRect> {
    let x1 = x.clamp(0, width);
    let y1 = y.clamp(0, height);
    let x2 = x.saturating_add(rect_width).clamp(0, width);
    let y2 = y.saturating_add(rect_height).clamp(0, height);

    if x2 <= x1 || y2 <= y1 {
        return None;
    }

    Some(DirtyRect {
        x: x1,
        y: y1,
        width: x2 - x1,
        height: y2 - y1,
    })
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
    use super::{CaptureModule, CAPTURE_HEADER_OFFSET, MAX_PIXEL_BUFFER_SIZE};
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

    #[test]
    fn writes_dirty_frame_when_previous_sequence_is_acked() {
        let name = format!("Capture.{}", uuid::Uuid::new_v4());
        let mut module = CaptureModule::new(&name, 4, 4).unwrap();
        let first = pixels(4, 4);
        module.write_full_frame(4, 4, &first).unwrap();
        ack_sequence(&module, 1);

        let second = pixels(4, 4);
        module
            .write_paint_frame(4, 4, &second, &[(1, 1, 2, 2)])
            .unwrap();

        let payload = read_payload(&name);
        let header_offset = CaptureModule::capture_header_offset();
        let slot_offset = MAX_PIXEL_BUFFER_SIZE * 2;
        assert_eq!(read_i32(&payload, header_offset), 4);
        assert_eq!(read_i32(&payload, header_offset + 4), 4);
        assert_eq!(read_i32(&payload, header_offset + 8), 2);
        assert_eq!(read_i32(&payload, header_offset + 12), 2);
        assert_eq!(read_i32(&payload, header_offset + 16), 1);
        assert_eq!(read_i32(&payload, header_offset + 20), 1);
        assert_eq!(read_i32(&payload, header_offset + 24), 32);
        assert_eq!(read_i32(&payload, header_offset + 28), 1);
        assert_eq!(read_i32(&payload, slot_offset), 1);
        assert_eq!(read_i32(&payload, slot_offset + 4), 1);
        assert_eq!(read_i32(&payload, slot_offset + 8), 2);
        assert_eq!(read_i32(&payload, slot_offset + 12), 2);
        assert_eq!(
            &payload[slot_offset + 16..slot_offset + 24],
            &second[20..28]
        );
        assert_eq!(
            &payload[slot_offset + 24..slot_offset + 32],
            &second[36..44]
        );
    }

    #[test]
    fn drops_same_size_frame_when_previous_sequence_is_not_acked() {
        let name = format!("Capture.{}", uuid::Uuid::new_v4());
        let mut module = CaptureModule::new(&name, 4, 4).unwrap();
        let first = pixels(4, 4);
        module.write_full_frame(4, 4, &first).unwrap();

        let second = pixels(4, 4);
        let published = module
            .write_paint_frame(4, 4, &second, &[(1, 1, 2, 2)])
            .unwrap();

        let payload = read_payload(&name);
        let header_offset = CaptureModule::capture_header_offset();
        assert!(!published);
        assert_eq!(read_i32(&payload, header_offset + 12), 1);
        assert_eq!(read_i32(&payload, header_offset + 16), 0);
        assert_eq!(read_i32(&payload, header_offset + 20), 0);
        assert_eq!(read_i32(&payload, header_offset + 24), 64);
        assert_eq!(
            &payload[MAX_PIXEL_BUFFER_SIZE..MAX_PIXEL_BUFFER_SIZE + 64],
            first.as_slice()
        );
    }

    #[test]
    fn falls_back_to_full_frame_for_resize_and_large_dirty_area() {
        let name = format!("Capture.{}", uuid::Uuid::new_v4());
        let mut module = CaptureModule::new(&name, 4, 4).unwrap();
        module.write_full_frame(4, 4, &pixels(4, 4)).unwrap();
        ack_sequence(&module, 1);

        let resized = pixels(5, 4);
        module
            .write_paint_frame(5, 4, &resized, &[(1, 1, 2, 2)])
            .unwrap();
        let payload = read_payload(&name);
        let header_offset = CaptureModule::capture_header_offset();
        assert_eq!(read_i32(&payload, header_offset + 16), 0);
        assert_eq!(read_i32(&payload, header_offset + 24), 80);

        ack_sequence(&module, 2);
        let large = pixels(5, 4);
        module
            .write_paint_frame(5, 4, &large, &[(0, 0, 5, 3)])
            .unwrap();
        let payload = read_payload(&name);
        assert_eq!(read_i32(&payload, header_offset + 16), 0);
        assert_eq!(read_i32(&payload, header_offset + 24), 80);
    }

    #[test]
    fn publishes_resized_full_frame_even_when_previous_sequence_is_not_acked() {
        let name = format!("Capture.{}", uuid::Uuid::new_v4());
        let mut module = CaptureModule::new(&name, 4, 4).unwrap();
        module.write_full_frame(4, 4, &pixels(4, 4)).unwrap();

        let resized = pixels(5, 4);
        let published = module.write_paint_frame(5, 4, &resized, &[]).unwrap();

        let payload = read_payload(&name);
        let header_offset = CaptureModule::capture_header_offset();
        let slot_offset = MAX_PIXEL_BUFFER_SIZE * 2;
        assert!(published);
        assert_eq!(read_i32(&payload, header_offset), 5);
        assert_eq!(read_i32(&payload, header_offset + 4), 4);
        assert_eq!(read_i32(&payload, header_offset + 8), 2);
        assert_eq!(read_i32(&payload, header_offset + 12), 2);
        assert_eq!(read_i32(&payload, header_offset + 16), 0);
        assert_eq!(read_i32(&payload, header_offset + 24), 80);
        assert_eq!(&payload[slot_offset..slot_offset + 80], resized.as_slice());
    }

    #[test]
    fn clips_dirty_rect_before_writing_payload() {
        let name = format!("Capture.{}", uuid::Uuid::new_v4());
        let mut module = CaptureModule::new(&name, 4, 4).unwrap();
        module.write_full_frame(4, 4, &pixels(4, 4)).unwrap();
        ack_sequence(&module, 1);

        let second = pixels(4, 4);
        module
            .write_paint_frame(4, 4, &second, &[(-1, 2, 3, 4)])
            .unwrap();

        let payload = read_payload(&name);
        let slot_offset = MAX_PIXEL_BUFFER_SIZE * 2;
        assert_eq!(read_i32(&payload, slot_offset), 0);
        assert_eq!(read_i32(&payload, slot_offset + 4), 2);
        assert_eq!(read_i32(&payload, slot_offset + 8), 2);
        assert_eq!(read_i32(&payload, slot_offset + 12), 2);
        assert_eq!(
            &payload[slot_offset + 16..slot_offset + 24],
            &second[32..40]
        );
        assert_eq!(
            &payload[slot_offset + 24..slot_offset + 32],
            &second[48..56]
        );
    }

    fn pixels(width: usize, height: usize) -> Vec<u8> {
        (0..width * height * 4).map(|value| value as u8).collect()
    }

    fn read_payload(name: &str) -> Vec<u8> {
        let reader = SharedMemoryWrapper::new(name, CaptureModule::capture_v2_size());
        let bytes = std::fs::read(reader.file_path()).unwrap();
        bytes[4..].to_vec()
    }

    fn ack_sequence(module: &CaptureModule, sequence: i32) {
        let mut shmem = module.shmem.lock().unwrap();
        shmem
            .write_payload_at(CAPTURE_HEADER_OFFSET + 28, &sequence.to_le_bytes())
            .unwrap();
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
