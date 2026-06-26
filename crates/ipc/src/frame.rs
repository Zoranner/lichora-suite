use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use memmap2::{MmapMut, MmapOptions};
use thiserror::Error;

use crate::{
    ChannelHeader, ChannelKind, ChannelOpenMode, DecodeError, FrameHeader, CHANNEL_HEADER_SIZE,
    FRAME_HEADER_SIZE, FRAME_PIXEL_FORMAT_BGRA32,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum FramePixelFormat {
    Bgra32 = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum FrameType {
    Full = 1,
    Resize = 2,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameSlot {
    pub sequence: u64,
    pub width: u32,
    pub height: u32,
    pub pixel_format: FramePixelFormat,
    pub frame_type: FrameType,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FramePublishResult {
    Published { slot_index: usize },
    Dropped,
}

impl FramePublishResult {
    pub fn published(self) -> bool {
        matches!(self, Self::Published { .. })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameChannelSpec {
    pub name: String,
    pub slot_count: u32,
    pub slot_size: u32,
}

impl FrameChannelSpec {
    pub fn new(name: impl Into<String>, slot_count: u32, slot_size: u32) -> Self {
        Self {
            name: name.into(),
            slot_count: slot_count.max(1),
            slot_size,
        }
    }

    pub fn capacity_bytes(&self) -> usize {
        FRAME_HEADER_SIZE + self.slot_count as usize * self.slot_size as usize
    }

    pub fn mapped_len(&self) -> usize {
        CHANNEL_HEADER_SIZE + self.capacity_bytes()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameCopyResult {
    pub width: i32,
    pub height: i32,
    pub sequence: u64,
    pub written: usize,
}

#[derive(Debug, Error)]
pub enum FrameCopyError {
    #[error("{0}")]
    Decode(#[from] DecodeError),
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("frame buffer too small: required={required}")]
    BufferTooSmall { required: usize },
    #[error("frame channel layout mismatch: {reason}")]
    LayoutMismatch { reason: String },
}

impl PartialEq for FrameCopyError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Decode(left), Self::Decode(right)) => left == right,
            (Self::BufferTooSmall { required: left }, Self::BufferTooSmall { required: right }) => {
                left == right
            }
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

impl Eq for FrameCopyError {}

pub type FrameCopyResultType<T> = Result<T, FrameCopyError>;

pub struct FrameChannel {
    mmap: MmapMut,
    path: PathBuf,
    slot_count: usize,
    slot_size: usize,
}

impl FrameChannel {
    pub fn open_in_dir(
        directory: impl AsRef<Path>,
        spec: &FrameChannelSpec,
        mode: ChannelOpenMode,
    ) -> FrameCopyResultType<Self> {
        let path = directory.as_ref().join(sanitize_file_name(&spec.name));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| FrameCopyError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let file = open_frame_file(&path, mode)?;
        if mode == ChannelOpenMode::Create {
            file.set_len(spec.mapped_len() as u64)
                .map_err(|source| FrameCopyError::Io {
                    path: path.clone(),
                    source,
                })?;
        }

        let mmap = unsafe { MmapOptions::new().len(spec.mapped_len()).map_mut(&file) }.map_err(
            |source| FrameCopyError::Io {
                path: path.clone(),
                source,
            },
        )?;

        let mut channel = Self {
            mmap,
            path,
            slot_count: spec.slot_count as usize,
            slot_size: spec.slot_size as usize,
        };

        if mode == ChannelOpenMode::Create {
            channel.initialize(spec)?;
        } else {
            channel.validate_layout(spec)?;
        }

        Ok(channel)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn header(&self) -> ChannelHeader {
        ChannelHeader::decode(&self.mmap[..CHANNEL_HEADER_SIZE]).expect("valid frame header")
    }

    pub fn frame_header(&self) -> FrameHeader {
        FrameHeader::decode(self.frame_header_slice()).expect("valid frame metadata")
    }

    pub fn publish_full(
        &mut self,
        sequence: u64,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> FrameCopyResultType<FramePublishResult> {
        let mut frame_header = self.frame_header();
        frame_header.submitted_paint_count = frame_header.submitted_paint_count.saturating_add(1);

        if self.should_drop_same_size_waiting_for_ack(&frame_header, width, height) {
            let mut header = self.begin_commit();
            frame_header.dropped_paint_count = frame_header.dropped_paint_count.saturating_add(1);
            self.write_frame_header(&frame_header);
            header.consumer_ack = frame_header.acknowledged_frame;
            self.finish_commit(header)?;
            return Ok(FramePublishResult::Dropped);
        }

        let mut header = self.begin_commit();

        let frame_type = if frame_header.frame_sequence != 0
            && (frame_header.width != width as i32 || frame_header.height != height as i32)
        {
            FrameType::Resize
        } else {
            FrameType::Full
        };
        let slot_index = (frame_header.current_slot as usize + 1) % self.slot_count;
        let payload_bytes = pixels.len().min(self.slot_size);
        self.slot_mut(slot_index)[..payload_bytes].copy_from_slice(&pixels[..payload_bytes]);
        if payload_bytes < self.slot_size {
            self.slot_mut(slot_index)[payload_bytes..].fill(0);
        }

        frame_header.width = width as i32;
        frame_header.height = height as i32;
        frame_header.pixel_format = FRAME_PIXEL_FORMAT_BGRA32;
        frame_header.slot_count = self.slot_count as u32;
        frame_header.slot_size = self.slot_size as u32;
        frame_header.current_slot = slot_index as u32;
        frame_header.frame_sequence = sequence;
        frame_header.frame_type = frame_type as u32;
        frame_header.rect_count = 0;
        frame_header.payload_bytes = payload_bytes as u32;
        frame_header.dirty_header_size = 0;
        frame_header.published_frame_count = frame_header.published_frame_count.saturating_add(1);
        self.write_frame_header(&frame_header);

        header.producer_sequence = sequence;
        header.consumer_ack = frame_header.acknowledged_frame;
        self.finish_commit(header)?;

        Ok(FramePublishResult::Published { slot_index })
    }

    pub fn try_copy_latest(&mut self, target: &mut [u8]) -> FrameCopyResultType<FrameCopyResult> {
        let first_header = self.header();
        if !first_header.header_commit.is_multiple_of(2) {
            return Ok(FrameCopyResult {
                width: 0,
                height: 0,
                sequence: 0,
                written: 0,
            });
        }

        let frame_header = self.frame_header();
        if frame_header.frame_sequence == 0 || frame_header.payload_bytes == 0 {
            return Ok(FrameCopyResult {
                width: 0,
                height: 0,
                sequence: 0,
                written: 0,
            });
        }

        let required = frame_header.payload_bytes as usize;
        if required > target.len() {
            return Err(FrameCopyError::BufferTooSmall { required });
        }

        let slot_index = frame_header.current_slot as usize;
        if slot_index >= self.slot_count || required > self.slot_size {
            return Err(FrameCopyError::LayoutMismatch {
                reason: "current frame slot is outside configured layout".to_string(),
            });
        }

        target[..required].copy_from_slice(&self.slot(slot_index)[..required]);
        let second_header = self.header();
        if first_header.header_commit != second_header.header_commit
            || !second_header.header_commit.is_multiple_of(2)
        {
            return Ok(FrameCopyResult {
                width: 0,
                height: 0,
                sequence: 0,
                written: 0,
            });
        }

        Ok(FrameCopyResult {
            width: frame_header.width,
            height: frame_header.height,
            sequence: frame_header.frame_sequence,
            written: required,
        })
    }

    pub fn ack(&mut self, sequence: u64) -> FrameCopyResultType<()> {
        let mut header = self.begin_commit();
        let mut frame_header = self.frame_header();
        frame_header.acknowledged_frame = frame_header.acknowledged_frame.max(sequence);
        self.write_frame_header(&frame_header);

        header.consumer_ack = frame_header.acknowledged_frame;
        self.finish_commit(header)
    }

    fn initialize(&mut self, spec: &FrameChannelSpec) -> FrameCopyResultType<()> {
        let mut header = ChannelHeader::new(ChannelKind::Frame, spec.capacity_bytes() as u32);
        header.payload_offset = (CHANNEL_HEADER_SIZE + FRAME_HEADER_SIZE) as u32;
        self.write_header(&header);
        self.write_frame_header(&FrameHeader {
            width: 0,
            height: 0,
            pixel_format: FRAME_PIXEL_FORMAT_BGRA32,
            slot_count: spec.slot_count,
            slot_size: spec.slot_size,
            current_slot: 0,
            frame_sequence: 0,
            acknowledged_frame: 0,
            frame_type: FrameType::Full as u32,
            rect_count: 0,
            payload_bytes: 0,
            dirty_header_size: 0,
            dropped_paint_count: 0,
            published_frame_count: 0,
            submitted_paint_count: 0,
            reserved: 0,
        });
        self.mmap[CHANNEL_HEADER_SIZE + FRAME_HEADER_SIZE..].fill(0);
        self.flush()
    }

    fn validate_layout(&self, spec: &FrameChannelSpec) -> FrameCopyResultType<()> {
        let header = self.header();
        if header.channel_kind != ChannelKind::Frame {
            return Err(FrameCopyError::LayoutMismatch {
                reason: format!(
                    "channel kind {:?} does not match Frame",
                    header.channel_kind
                ),
            });
        }
        if header.capacity_bytes != spec.capacity_bytes() as u32 {
            return Err(FrameCopyError::LayoutMismatch {
                reason: format!(
                    "capacity {} does not match {}",
                    header.capacity_bytes,
                    spec.capacity_bytes()
                ),
            });
        }
        if header.payload_offset != (CHANNEL_HEADER_SIZE + FRAME_HEADER_SIZE) as u32 {
            return Err(FrameCopyError::LayoutMismatch {
                reason: format!(
                    "payload offset {} does not match frame layout",
                    header.payload_offset
                ),
            });
        }

        let frame_header = self.frame_header();
        if frame_header.slot_count != spec.slot_count || frame_header.slot_size != spec.slot_size {
            return Err(FrameCopyError::LayoutMismatch {
                reason: "frame metadata does not match spec".to_string(),
            });
        }

        Ok(())
    }

    fn should_drop_same_size_waiting_for_ack(
        &self,
        frame_header: &FrameHeader,
        width: u32,
        height: u32,
    ) -> bool {
        frame_header.frame_sequence > frame_header.acknowledged_frame
            && frame_header.width == width as i32
            && frame_header.height == height as i32
    }

    fn write_header(&mut self, header: &ChannelHeader) {
        self.mmap[..CHANNEL_HEADER_SIZE].copy_from_slice(&header.encode());
    }

    fn begin_commit(&mut self) -> ChannelHeader {
        let mut header = self.header();
        header.header_commit = next_odd_commit(header.header_commit);
        self.write_header(&header);
        header
    }

    fn finish_commit(&mut self, mut header: ChannelHeader) -> FrameCopyResultType<()> {
        header.header_commit = header.header_commit.saturating_add(1);
        self.write_header(&header);
        self.flush()
    }

    fn frame_header_slice(&self) -> &[u8] {
        &self.mmap[CHANNEL_HEADER_SIZE..CHANNEL_HEADER_SIZE + FRAME_HEADER_SIZE]
    }

    fn write_frame_header(&mut self, header: &FrameHeader) {
        self.mmap[CHANNEL_HEADER_SIZE..CHANNEL_HEADER_SIZE + FRAME_HEADER_SIZE]
            .copy_from_slice(&header.encode());
    }

    fn slot(&self, slot_index: usize) -> &[u8] {
        let start = self.slot_start(slot_index);
        &self.mmap[start..start + self.slot_size]
    }

    fn slot_mut(&mut self, slot_index: usize) -> &mut [u8] {
        let start = self.slot_start(slot_index);
        &mut self.mmap[start..start + self.slot_size]
    }

    fn slot_start(&self, slot_index: usize) -> usize {
        CHANNEL_HEADER_SIZE + FRAME_HEADER_SIZE + slot_index * self.slot_size
    }

    fn flush(&mut self) -> FrameCopyResultType<()> {
        self.mmap
            .flush_async()
            .map_err(|source| FrameCopyError::Io {
                path: self.path.clone(),
                source,
            })
    }
}

pub struct FrameRingState {
    slots: Vec<Option<FrameSlot>>,
    slot_size: usize,
    next_slot: usize,
    latest_slot: Option<usize>,
    acknowledged_sequence: u64,
    submitted_count: u64,
    published_count: u64,
    dropped_count: u64,
}

impl FrameRingState {
    pub fn new(slot_count: usize, slot_size: usize) -> Self {
        let slot_count = slot_count.max(1);

        Self {
            slots: vec![None; slot_count],
            slot_size,
            next_slot: 0,
            latest_slot: None,
            acknowledged_sequence: 0,
            submitted_count: 0,
            published_count: 0,
            dropped_count: 0,
        }
    }

    pub fn publish_full(
        &mut self,
        sequence: u64,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> FramePublishResult {
        self.submitted_count = self.submitted_count.saturating_add(1);

        if self.should_drop_same_size_waiting_for_ack(width, height) {
            self.dropped_count = self.dropped_count.saturating_add(1);
            return FramePublishResult::Dropped;
        }

        let frame_type = match self.latest_dimensions() {
            Some((latest_width, latest_height))
                if latest_width != width || latest_height != height =>
            {
                FrameType::Resize
            }
            _ => FrameType::Full,
        };
        let slot_index = self.next_slot;
        self.slots[slot_index] = Some(FrameSlot {
            sequence,
            width,
            height,
            pixel_format: FramePixelFormat::Bgra32,
            frame_type,
            pixels: pixels[..pixels.len().min(self.slot_size)].to_vec(),
        });
        self.latest_slot = Some(slot_index);
        self.next_slot = (self.next_slot + 1) % self.slots.len();
        self.published_count = self.published_count.saturating_add(1);

        FramePublishResult::Published { slot_index }
    }

    pub fn ack(&mut self, sequence: u64) {
        self.acknowledged_sequence = self.acknowledged_sequence.max(sequence);
    }

    pub fn latest_frame(&self) -> Option<&FrameSlot> {
        self.latest_slot
            .and_then(|slot_index| self.slots[slot_index].as_ref())
    }

    pub fn acknowledged_sequence(&self) -> u64 {
        self.acknowledged_sequence
    }

    pub fn submitted_count(&self) -> u64 {
        self.submitted_count
    }

    pub fn published_count(&self) -> u64 {
        self.published_count
    }

    pub fn dropped_count(&self) -> u64 {
        self.dropped_count
    }

    fn should_drop_same_size_waiting_for_ack(&self, width: u32, height: u32) -> bool {
        let Some(latest) = self.latest_frame() else {
            return false;
        };

        latest.width == width
            && latest.height == height
            && latest.sequence > self.acknowledged_sequence
    }

    fn latest_dimensions(&self) -> Option<(u32, u32)> {
        self.latest_frame().map(|frame| (frame.width, frame.height))
    }
}

fn open_frame_file(path: &Path, mode: ChannelOpenMode) -> FrameCopyResultType<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    if mode == ChannelOpenMode::Create {
        options.create(true).truncate(true);
    }

    options.open(path).map_err(|source| FrameCopyError::Io {
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

fn next_odd_commit(current: u64) -> u64 {
    let next = current.saturating_add(1);
    if next.is_multiple_of(2) {
        next.saturating_add(1)
    } else {
        next
    }
}
