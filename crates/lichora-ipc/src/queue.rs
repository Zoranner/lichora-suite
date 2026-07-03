use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use memmap2::{MmapMut, MmapOptions};
use thiserror::Error;

use crate::{
    ChannelHeader, ChannelKind, ChannelOpenMode, DecodeError, SpscQueueMetadata,
    CHANNEL_HEADER_SIZE,
};

const ITEM_HEADER_SIZE: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappedQueueSpec {
    pub name: String,
    pub channel_kind: ChannelKind,
    pub item_capacity: u32,
    pub max_payload_len: u32,
}

impl MappedQueueSpec {
    pub fn new(
        name: impl Into<String>,
        channel_kind: ChannelKind,
        item_capacity: u32,
        max_payload_len: u32,
    ) -> Self {
        Self {
            name: name.into(),
            channel_kind,
            item_capacity,
            max_payload_len,
        }
    }

    pub fn item_size(&self) -> usize {
        ITEM_HEADER_SIZE + self.max_payload_len as usize
    }

    pub fn capacity_bytes(&self) -> usize {
        SpscQueueMetadata::BYTE_SIZE + self.item_capacity as usize * self.item_size()
    }

    pub fn mapped_len(&self) -> usize {
        CHANNEL_HEADER_SIZE + self.capacity_bytes()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappedQueueItem {
    pub kind: u32,
    pub sequence: u64,
    pub payload: Vec<u8>,
}

impl MappedQueueItem {
    pub fn new(kind: u32, sequence: u64, payload: impl AsRef<[u8]>) -> Self {
        Self {
            kind,
            sequence,
            payload: payload.as_ref().to_vec(),
        }
    }

    pub fn payload_len(&self) -> usize {
        self.payload.len()
    }
}

#[derive(Debug, Error)]
pub enum MappedQueueError {
    #[error("{0}")]
    Decode(#[from] DecodeError),
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("mapped queue is full: capacity={capacity}")]
    QueueFull { capacity: usize },
    #[error("payload exceeds mapped queue item capacity: payload={payload}, capacity={capacity}")]
    PayloadTooLarge { payload: usize, capacity: usize },
    #[error("mapped queue layout mismatch: {reason}")]
    LayoutMismatch { reason: String },
}

impl PartialEq for MappedQueueError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Decode(left), Self::Decode(right)) => left == right,
            (Self::QueueFull { capacity: left }, Self::QueueFull { capacity: right }) => {
                left == right
            }
            (
                Self::PayloadTooLarge {
                    payload: left_payload,
                    capacity: left_capacity,
                },
                Self::PayloadTooLarge {
                    payload: right_payload,
                    capacity: right_capacity,
                },
            ) => left_payload == right_payload && left_capacity == right_capacity,
            (
                Self::LayoutMismatch {
                    reason: left_reason,
                },
                Self::LayoutMismatch {
                    reason: right_reason,
                },
            ) => left_reason == right_reason,
            _ => false,
        }
    }
}

impl Eq for MappedQueueError {}

pub type MappedQueueResult<T> = Result<T, MappedQueueError>;

pub struct MappedSpscQueue {
    mmap: MmapMut,
    path: PathBuf,
    item_capacity: usize,
    item_size: usize,
    max_payload_len: usize,
}

impl MappedSpscQueue {
    pub fn open_in_dir(
        directory: impl AsRef<Path>,
        spec: &MappedQueueSpec,
        mode: ChannelOpenMode,
    ) -> MappedQueueResult<Self> {
        let path = directory.as_ref().join(sanitize_file_name(&spec.name));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| MappedQueueError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let file = open_queue_file(&path, mode)?;
        if mode == ChannelOpenMode::Create {
            file.set_len(spec.mapped_len() as u64)
                .map_err(|source| MappedQueueError::Io {
                    path: path.clone(),
                    source,
                })?;
        }

        let mmap = unsafe { MmapOptions::new().len(spec.mapped_len()).map_mut(&file) }.map_err(
            |source| MappedQueueError::Io {
                path: path.clone(),
                source,
            },
        )?;

        let mut queue = Self {
            mmap,
            path,
            item_capacity: spec.item_capacity as usize,
            item_size: spec.item_size(),
            max_payload_len: spec.max_payload_len as usize,
        };

        if mode == ChannelOpenMode::Create {
            queue.initialize(spec)?;
        } else {
            queue.validate_layout(spec)?;
        }

        Ok(queue)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn header(&self) -> MappedQueueResult<ChannelHeader> {
        Ok(ChannelHeader::decode(&self.mmap[..CHANNEL_HEADER_SIZE])?)
    }

    pub fn metadata(&self) -> SpscQueueMetadata {
        SpscQueueMetadata::decode(self.metadata_slice()).expect("valid queue metadata")
    }

    pub fn try_push(&mut self, item: MappedQueueItem) -> MappedQueueResult<()> {
        if item.payload.len() > self.max_payload_len {
            return Err(MappedQueueError::PayloadTooLarge {
                payload: item.payload.len(),
                capacity: self.max_payload_len,
            });
        }

        let mut metadata = self.metadata();
        if metadata.available_slots() == 0 {
            metadata.dropped_count = metadata.dropped_count.saturating_add(1);
            self.write_metadata(metadata);
            self.update_header_sequences(metadata)?;
            self.flush()?;
            return Err(MappedQueueError::QueueFull {
                capacity: self.item_capacity,
            });
        }

        let slot_index = metadata.write_sequence as usize % self.item_capacity;
        self.write_item(slot_index, &item);

        metadata.write_sequence = metadata.write_sequence.saturating_add(1);
        self.write_metadata(metadata);
        self.update_header_sequences(metadata)?;
        self.flush()
    }

    pub fn try_pop(&mut self) -> MappedQueueResult<Option<MappedQueueItem>> {
        let mut metadata = self.metadata();
        if metadata.queued_items() == 0 {
            return Ok(None);
        }

        let slot_index = metadata.read_sequence as usize % self.item_capacity;
        let item = self.read_item(slot_index)?;
        self.clear_slot(slot_index);

        metadata.read_sequence = metadata.read_sequence.saturating_add(1);
        self.write_metadata(metadata);
        self.update_header_sequences(metadata)?;
        self.flush()?;

        Ok(Some(item))
    }

    fn initialize(&mut self, spec: &MappedQueueSpec) -> MappedQueueResult<()> {
        let mut header = ChannelHeader::new(spec.channel_kind, spec.capacity_bytes() as u32);
        header.payload_offset = (CHANNEL_HEADER_SIZE + SpscQueueMetadata::BYTE_SIZE) as u32;
        self.write_header(&header);
        self.write_metadata(SpscQueueMetadata {
            item_capacity: spec.item_capacity,
            item_size: spec.item_size() as u32,
            write_sequence: 0,
            read_sequence: 0,
            dropped_count: 0,
            merged_count: 0,
        });
        self.mmap[header.payload_offset as usize..].fill(0);
        self.flush()
    }

    fn validate_layout(&self, spec: &MappedQueueSpec) -> MappedQueueResult<()> {
        let header = self.header()?;
        let expected_payload_offset = (CHANNEL_HEADER_SIZE + SpscQueueMetadata::BYTE_SIZE) as u32;
        if header.channel_kind != spec.channel_kind {
            return Err(MappedQueueError::LayoutMismatch {
                reason: format!(
                    "channel kind {:?} does not match {:?}",
                    header.channel_kind, spec.channel_kind
                ),
            });
        }
        if header.capacity_bytes != spec.capacity_bytes() as u32 {
            return Err(MappedQueueError::LayoutMismatch {
                reason: format!(
                    "capacity {} does not match {}",
                    header.capacity_bytes,
                    spec.capacity_bytes()
                ),
            });
        }
        if header.payload_offset != expected_payload_offset {
            return Err(MappedQueueError::LayoutMismatch {
                reason: format!(
                    "payload offset {} does not match {}",
                    header.payload_offset, expected_payload_offset
                ),
            });
        }

        let metadata = self.metadata();
        if metadata.item_capacity != spec.item_capacity
            || metadata.item_size != spec.item_size() as u32
        {
            return Err(MappedQueueError::LayoutMismatch {
                reason: "queue metadata does not match spec".to_string(),
            });
        }

        Ok(())
    }

    fn write_item(&mut self, slot_index: usize, item: &MappedQueueItem) {
        let max_payload_len = self.max_payload_len;
        let slot = self.slot_mut(slot_index);
        write_u32(slot, 0, item.kind);
        write_u32(slot, 4, item.payload.len() as u32);
        write_u64(slot, 8, item.sequence);
        slot[ITEM_HEADER_SIZE..ITEM_HEADER_SIZE + item.payload.len()]
            .copy_from_slice(&item.payload);
        slot[ITEM_HEADER_SIZE + item.payload.len()..ITEM_HEADER_SIZE + max_payload_len].fill(0);
    }

    fn read_item(&self, slot_index: usize) -> MappedQueueResult<MappedQueueItem> {
        let slot = self.slot(slot_index);
        let kind = read_u32(slot, 0);
        let payload_len = read_u32(slot, 4) as usize;
        if payload_len > self.max_payload_len {
            return Err(MappedQueueError::LayoutMismatch {
                reason: format!(
                    "item payload length {payload_len} exceeds {}",
                    self.max_payload_len
                ),
            });
        }

        Ok(MappedQueueItem {
            kind,
            sequence: read_u64(slot, 8),
            payload: slot[ITEM_HEADER_SIZE..ITEM_HEADER_SIZE + payload_len].to_vec(),
        })
    }

    fn clear_slot(&mut self, slot_index: usize) {
        self.slot_mut(slot_index).fill(0);
    }

    fn update_header_sequences(&mut self, metadata: SpscQueueMetadata) -> MappedQueueResult<()> {
        let mut header = self.header()?;
        header.producer_sequence = metadata.write_sequence;
        header.consumer_ack = metadata.read_sequence;
        self.write_header(&header);
        Ok(())
    }

    fn write_header(&mut self, header: &ChannelHeader) {
        self.mmap[..CHANNEL_HEADER_SIZE].copy_from_slice(&header.encode());
    }

    fn metadata_slice(&self) -> &[u8] {
        &self.mmap[CHANNEL_HEADER_SIZE..CHANNEL_HEADER_SIZE + SpscQueueMetadata::BYTE_SIZE]
    }

    fn write_metadata(&mut self, metadata: SpscQueueMetadata) {
        let start = CHANNEL_HEADER_SIZE;
        let end = start + SpscQueueMetadata::BYTE_SIZE;
        self.mmap[start..end].copy_from_slice(&metadata.encode());
    }

    fn slot(&self, slot_index: usize) -> &[u8] {
        let start = self.slot_start(slot_index);
        &self.mmap[start..start + self.item_size]
    }

    fn slot_mut(&mut self, slot_index: usize) -> &mut [u8] {
        let start = self.slot_start(slot_index);
        &mut self.mmap[start..start + self.item_size]
    }

    fn slot_start(&self, slot_index: usize) -> usize {
        CHANNEL_HEADER_SIZE + SpscQueueMetadata::BYTE_SIZE + slot_index * self.item_size
    }

    fn flush(&mut self) -> MappedQueueResult<()> {
        Ok(())
    }
}

fn open_queue_file(path: &Path, mode: ChannelOpenMode) -> MappedQueueResult<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    if mode == ChannelOpenMode::Create {
        options.create(true).truncate(true);
    }

    options.open(path).map_err(|source| MappedQueueError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn sanitize_file_name(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn write_u32(buffer: &mut [u8], offset: usize, value: u32) {
    buffer[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(buffer: &mut [u8], offset: usize, value: u64) {
    buffer[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn read_u32(buffer: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(buffer[offset..offset + 4].try_into().expect("slice length"))
}

fn read_u64(buffer: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(buffer[offset..offset + 8].try_into().expect("slice length"))
}
