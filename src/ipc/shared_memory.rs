//! Shared Memory Wrapper - Cross-platform shared memory for Unity communication
//!
//! This module provides a wrapper around the file-backed memory maps used by
//! the Unity `MemoryStack` implementation.

use std::fs::{File, OpenOptions};
use std::path::PathBuf;

use anyhow::{Context, Result};
use log::info;
use memmap2::{MmapMut, MmapOptions};

/// Shared memory wrapper for browser frames
pub struct SharedMemoryWrapper {
    mmap: Option<MmapMut>,
    name: String,
    size: usize,
    file_path: PathBuf,
    file: Option<File>,
}

impl SharedMemoryWrapper {
    /// Create a new shared memory wrapper
    pub fn new(name: &str, size: usize) -> Self {
        Self {
            mmap: None,
            name: name.to_string(),
            size,
            file_path: build_path(name),
            file: None,
        }
    }

    /// Initialize shared memory (create or open)
    pub fn initialize(&mut self) -> Result<()> {
        let total_size = self.size + std::mem::size_of::<u32>();
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create shared memory directory: {parent:?}"))?;
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.file_path)
            .with_context(|| format!("open shared memory file: {:?}", self.file_path))?;
        file.set_len(total_size as u64)
            .with_context(|| format!("resize shared memory file: {:?}", self.file_path))?;

        let mmap = unsafe { MmapOptions::new().len(total_size).map_mut(&file) }
            .with_context(|| format!("map shared memory file: {:?}", self.file_path))?;

        info!("Opened MemoryStack file: {}", self.name);
        self.file = Some(file);
        self.mmap = Some(mmap);
        Ok(())
    }

    /// Write bytes to shared memory
    pub fn write_bytes(&mut self, data: &[u8]) -> Result<()> {
        let mmap = self
            .mmap
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Shared memory not initialized"))?;

        let max_payload = mmap.len().saturating_sub(std::mem::size_of::<u32>());
        let len = data.len().min(max_payload);
        let length_header = (len as u32).to_be_bytes();
        mmap[..4].copy_from_slice(&length_header);
        mmap[4..4 + len].copy_from_slice(&data[..len]);
        mmap.flush_async()?;

        Ok(())
    }

    pub fn write_length(&mut self, length: usize) -> Result<()> {
        let mmap = self
            .mmap
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Shared memory not initialized"))?;
        let max_payload = mmap.len().saturating_sub(std::mem::size_of::<u32>());
        let length = length.min(max_payload);
        mmap[..4].copy_from_slice(&(length as u32).to_be_bytes());
        mmap.flush_async()?;
        Ok(())
    }

    pub fn write_payload_at(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        let mmap = self
            .mmap
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Shared memory not initialized"))?;
        let start = 4 + offset;
        let end = start
            .checked_add(data.len())
            .ok_or_else(|| anyhow::anyhow!("Payload write offset overflow"))?;
        if end > mmap.len() {
            return Err(anyhow::anyhow!("Payload write exceeds shared memory size"));
        }
        mmap[start..end].copy_from_slice(data);
        mmap.flush_async()?;
        Ok(())
    }

    pub fn read_i32_at(&self, offset: usize) -> Result<i32> {
        let mmap = self
            .mmap
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Shared memory not initialized"))?;
        let start = 4 + offset;
        if start + 4 > mmap.len() {
            return Err(anyhow::anyhow!("Payload read exceeds shared memory size"));
        }
        Ok(i32::from_le_bytes([
            mmap[start],
            mmap[start + 1],
            mmap[start + 2],
            mmap[start + 3],
        ]))
    }

    /// Read bytes from shared memory
    pub fn read_bytes(&self) -> Result<Vec<u8>> {
        let mmap = self
            .mmap
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Shared memory not initialized"))?;

        if mmap.len() < 4 {
            return Ok(Vec::new());
        }

        let length = u32::from_be_bytes([mmap[0], mmap[1], mmap[2], mmap[3]]) as usize;
        let max_payload = mmap.len() - 4;
        if length == 0 || length > max_payload {
            return Ok(Vec::new());
        }

        Ok(mmap[4..4 + length].to_vec())
    }

    /// Write a frame to shared memory
    ///
    /// Format: [width(4), height(4), pixels...]
    pub fn write_frame(&mut self, width: i32, height: i32, pixels: &[u8]) -> Result<()> {
        let mut data = Vec::with_capacity(8 + pixels.len());
        data.extend_from_slice(&width.to_le_bytes());
        data.extend_from_slice(&height.to_le_bytes());
        data.extend_from_slice(pixels);

        self.write_bytes(&data)
    }

    /// Read a frame from shared memory
    pub fn read_frame(&self) -> Result<(i32, i32, Vec<u8>)> {
        let data = self.read_bytes()?;

        if data.len() < 8 {
            return Err(anyhow::anyhow!("Frame data too short"));
        }

        let width = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let height = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let pixels = data[8..].to_vec();

        Ok((width, height, pixels))
    }

    /// Write a single byte at a given offset (e.g. to clear flag byte)
    pub fn write_byte_at(&mut self, offset: usize, value: u8) -> Result<()> {
        let mmap = self
            .mmap
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Shared memory not initialized"))?;

        let payload_offset = 4 + offset;
        if payload_offset < mmap.len() {
            mmap[payload_offset] = value;
            mmap.flush_async()?;
        }
        Ok(())
    }

    /// Read a single byte at a given offset (e.g. to check flag byte)
    pub fn read_byte_at(&self, offset: usize) -> Result<u8> {
        let mmap = self
            .mmap
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Shared memory not initialized"))?;

        let payload_offset = 4 + offset;
        if payload_offset < mmap.len() {
            Ok(mmap[payload_offset])
        } else {
            Err(anyhow::anyhow!("Offset out of range"))
        }
    }

    /// Check if shared memory is initialized
    pub fn is_initialized(&self) -> bool {
        self.mmap.is_some()
    }

    /// Get the size of shared memory
    pub fn size(&self) -> usize {
        self.size
    }

    pub fn file_path(&self) -> &std::path::Path {
        &self.file_path
    }
}

// SAFETY: SharedMemoryWrapper is always accessed through Arc<Mutex<>> where it is shared.
unsafe impl Send for SharedMemoryWrapper {}

impl Drop for SharedMemoryWrapper {
    fn drop(&mut self) {
        if self.mmap.is_some() {
            info!("Closing shared memory: {}", self.name);
        }
    }
}

fn build_path(name: &str) -> PathBuf {
    let safe_name = sanitize_name(name);
    if cfg!(target_os = "linux") {
        PathBuf::from(format!("/dev/shm/MemoryStacks_{safe_name}"))
    } else {
        std::env::temp_dir().join(format!("MemoryStacks_{safe_name}"))
    }
}

fn sanitize_name(name: &str) -> String {
    if name.trim().is_empty() {
        return "default".to_string();
    }

    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::SharedMemoryWrapper;

    #[test]
    fn writes_memorystack_length_header_and_payload() {
        let name = format!("Test.Stack.{}", uuid::Uuid::new_v4());
        let mut stack = SharedMemoryWrapper::new(&name, 16);
        stack.initialize().unwrap();

        stack.write_bytes(&[1, 2, 3]).unwrap();

        let path = std::env::temp_dir().join(format!("MemoryStacks_{name}"));
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(&bytes[..4], &[0, 0, 0, 3]);
        assert_eq!(&bytes[4..7], &[1, 2, 3]);
    }

    #[test]
    fn writes_payload_offsets_after_memorystack_length_header() {
        let name = format!("Test.Stack.{}", uuid::Uuid::new_v4());
        let mut stack = SharedMemoryWrapper::new(&name, 16);
        stack.initialize().unwrap();
        stack.write_bytes(&[1, 2, 3, 4]).unwrap();

        stack.write_byte_at(1, 9).unwrap();

        let payload = stack.read_bytes().unwrap();
        assert_eq!(payload, vec![1, 9, 3, 4]);
    }
}
