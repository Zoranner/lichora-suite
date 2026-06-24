use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use memmap2::{MmapMut, MmapOptions};
use thiserror::Error;

pub mod control;
pub mod frame;
pub mod queue;
pub mod status;

pub use control::{ControlCommand, ControlDecodeError};
pub use frame::{FramePixelFormat, FramePublishResult, FrameRingState, FrameSlot, FrameType};
pub use queue::{MappedQueueError, MappedQueueItem, MappedQueueSpec, MappedSpscQueue};
pub use status::{
    BrowserState, OutputEvent, OutputEventKind, OutputQueueState, ProcessState, StatusCounters,
    StatusSnapshot,
};

pub const CHANNEL_MAGIC: u32 = 0x3249_4245;
pub const PROTOCOL_VERSION_MAJOR: u16 = 2;
pub const PROTOCOL_VERSION_MINOR: u16 = 0;
pub const CHANNEL_HEADER_SIZE: usize = 64;
pub const LATEST_PAYLOAD_LENGTH_SIZE: usize = 4;
pub const FRAME_HEADER_SIZE: usize = 88;
pub const FRAME_PIXEL_FORMAT_BGRA32: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ChannelKind {
    Session = 1,
    Control = 2,
    Status = 3,
    Input = 4,
    Frame = 5,
    Output = 6,
}

impl ChannelKind {
    pub fn suffix(self) -> &'static str {
        match self {
            ChannelKind::Session => "session",
            ChannelKind::Control => "control",
            ChannelKind::Status => "status",
            ChannelKind::Input => "input",
            ChannelKind::Frame => "frame",
            ChannelKind::Output => "output",
        }
    }
}

impl TryFrom<u32> for ChannelKind {
    type Error = DecodeError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(ChannelKind::Session),
            2 => Ok(ChannelKind::Control),
            3 => Ok(ChannelKind::Status),
            4 => Ok(ChannelKind::Input),
            5 => Ok(ChannelKind::Frame),
            6 => Ok(ChannelKind::Output),
            other => Err(DecodeError::InvalidChannelKind(other)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodeError {
    BufferTooSmall { expected: usize, actual: usize },
    InvalidMagic(u32),
    UnsupportedVersion { major: u16, minor: u16 },
    InvalidHeaderSize(u32),
    InvalidChannelKind(u32),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::BufferTooSmall { expected, actual } => {
                write!(
                    formatter,
                    "buffer too small: expected {expected}, actual {actual}"
                )
            }
            DecodeError::InvalidMagic(magic) => write!(formatter, "invalid magic: {magic:#x}"),
            DecodeError::UnsupportedVersion { major, minor } => {
                write!(formatter, "unsupported version: {major}.{minor}")
            }
            DecodeError::InvalidHeaderSize(size) => {
                write!(formatter, "invalid header size: {size}")
            }
            DecodeError::InvalidChannelKind(kind) => {
                write!(formatter, "invalid channel kind: {kind}")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

#[derive(Debug, Error)]
pub enum IpcError {
    #[error("{0}")]
    Decode(#[from] DecodeError),
    #[error("payload exceeds channel capacity: payload={payload}, capacity={capacity}")]
    PayloadExceedsCapacity { payload: usize, capacity: usize },
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("input event queue is full: capacity={capacity}")]
    InputQueueFull { capacity: usize },
}

pub type IpcResult<T> = Result<T, IpcError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChannelHeader {
    pub version_minor: u16,
    pub channel_kind: ChannelKind,
    pub flags: u32,
    pub capacity_bytes: u32,
    pub producer_sequence: u64,
    pub consumer_ack: u64,
    pub producer_ticks: i64,
    pub status_code: i32,
    pub payload_offset: u32,
    pub header_commit: u64,
}

impl ChannelHeader {
    pub fn new(channel_kind: ChannelKind, capacity_bytes: u32) -> Self {
        Self {
            version_minor: PROTOCOL_VERSION_MINOR,
            channel_kind,
            flags: 0,
            capacity_bytes,
            producer_sequence: 0,
            consumer_ack: 0,
            producer_ticks: 0,
            status_code: 0,
            payload_offset: CHANNEL_HEADER_SIZE as u32,
            header_commit: 0,
        }
    }

    pub fn encode(&self) -> [u8; CHANNEL_HEADER_SIZE] {
        let mut buffer = [0u8; CHANNEL_HEADER_SIZE];
        write_u32(&mut buffer, 0, CHANNEL_MAGIC);
        write_u16(&mut buffer, 4, PROTOCOL_VERSION_MAJOR);
        write_u16(&mut buffer, 6, self.version_minor);
        write_u32(&mut buffer, 8, CHANNEL_HEADER_SIZE as u32);
        write_u32(&mut buffer, 12, self.channel_kind as u32);
        write_u32(&mut buffer, 16, self.flags);
        write_u32(&mut buffer, 20, self.capacity_bytes);
        write_u64(&mut buffer, 24, self.producer_sequence);
        write_u64(&mut buffer, 32, self.consumer_ack);
        write_i64(&mut buffer, 40, self.producer_ticks);
        write_i32(&mut buffer, 48, self.status_code);
        write_u32(&mut buffer, 52, self.payload_offset);
        write_u64(&mut buffer, 56, self.header_commit);
        buffer
    }

    pub fn decode(buffer: &[u8]) -> Result<Self, DecodeError> {
        require_len(buffer, CHANNEL_HEADER_SIZE)?;

        let magic = read_u32(buffer, 0);
        if magic != CHANNEL_MAGIC {
            return Err(DecodeError::InvalidMagic(magic));
        }

        let version_major = read_u16(buffer, 4);
        let version_minor = read_u16(buffer, 6);
        if version_major != PROTOCOL_VERSION_MAJOR {
            return Err(DecodeError::UnsupportedVersion {
                major: version_major,
                minor: version_minor,
            });
        }

        let header_size = read_u32(buffer, 8);
        if header_size != CHANNEL_HEADER_SIZE as u32 {
            return Err(DecodeError::InvalidHeaderSize(header_size));
        }

        Ok(Self {
            version_minor,
            channel_kind: ChannelKind::try_from(read_u32(buffer, 12))?,
            flags: read_u32(buffer, 16),
            capacity_bytes: read_u32(buffer, 20),
            producer_sequence: read_u64(buffer, 24),
            consumer_ack: read_u64(buffer, 32),
            producer_ticks: read_i64(buffer, 40),
            status_code: read_i32(buffer, 48),
            payload_offset: read_u32(buffer, 52),
            header_commit: read_u64(buffer, 56),
        })
    }
}

pub fn build_session_channel_name(session_id: &str, channel_kind: ChannelKind) -> String {
    format!("EmbeddedBrowser_{session_id}_{}", channel_kind.suffix())
}

pub fn build_browser_channel_name(
    session_id: &str,
    browser_id: &str,
    channel_kind: ChannelKind,
) -> String {
    format!(
        "EmbeddedBrowser_{session_id}_{browser_id}_{}",
        channel_kind.suffix()
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChannelSpec {
    pub name: String,
    pub channel_kind: ChannelKind,
    pub capacity_bytes: u32,
}

impl ChannelSpec {
    pub fn new(name: impl Into<String>, channel_kind: ChannelKind, capacity_bytes: u32) -> Self {
        Self {
            name: name.into(),
            channel_kind,
            capacity_bytes,
        }
    }

    pub fn mapped_len(&self) -> usize {
        CHANNEL_HEADER_SIZE + self.capacity_bytes as usize
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelOpenMode {
    Create,
    OpenExisting,
}

#[derive(Debug)]
pub struct LatestSnapshot {
    pub header: ChannelHeader,
    pub payload: Vec<u8>,
}

pub struct ChannelMappedFile {
    mmap: MmapMut,
    path: PathBuf,
    capacity_bytes: usize,
}

impl ChannelMappedFile {
    pub fn open_in_dir(
        directory: impl AsRef<Path>,
        spec: &ChannelSpec,
        mode: ChannelOpenMode,
    ) -> IpcResult<Self> {
        let path = directory.as_ref().join(sanitize_file_name(&spec.name));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| IpcError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let file = open_channel_file(&path, mode)?;
        if mode == ChannelOpenMode::Create {
            file.set_len(spec.mapped_len() as u64)
                .map_err(|source| IpcError::Io {
                    path: path.clone(),
                    source,
                })?;
        }

        let mmap = unsafe { MmapOptions::new().len(spec.mapped_len()).map_mut(&file) }.map_err(
            |source| IpcError::Io {
                path: path.clone(),
                source,
            },
        )?;

        let mut channel = Self {
            mmap,
            path,
            capacity_bytes: spec.capacity_bytes as usize,
        };

        if mode == ChannelOpenMode::Create {
            let mut header = ChannelHeader::new(spec.channel_kind, spec.capacity_bytes);
            header.payload_offset = (CHANNEL_HEADER_SIZE + LATEST_PAYLOAD_LENGTH_SIZE) as u32;
            channel.write_header(&header);
            channel.flush()?;
        }

        Ok(channel)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn capacity_bytes(&self) -> usize {
        self.capacity_bytes
    }

    pub fn header(&self) -> ChannelHeader {
        ChannelHeader::decode(&self.mmap[..CHANNEL_HEADER_SIZE]).expect("valid channel header")
    }

    pub fn publish_latest(
        &mut self,
        producer_sequence: u64,
        status_code: i32,
        payload: &[u8],
    ) -> IpcResult<()> {
        let payload_capacity = self
            .capacity_bytes
            .saturating_sub(LATEST_PAYLOAD_LENGTH_SIZE);
        if payload.len() > payload_capacity {
            return Err(IpcError::PayloadExceedsCapacity {
                payload: payload.len(),
                capacity: payload_capacity,
            });
        }

        let mut header = self.header();
        header.header_commit = next_odd_commit(header.header_commit);
        self.write_header(&header);

        let length_start = CHANNEL_HEADER_SIZE;
        self.mmap[length_start..length_start + LATEST_PAYLOAD_LENGTH_SIZE]
            .copy_from_slice(&(payload.len() as u32).to_le_bytes());
        let payload_start = header.payload_offset as usize;
        let payload_end = payload_start + payload.len();
        self.mmap[payload_start..payload_end].copy_from_slice(payload);
        if payload_end < self.mmap.len() {
            self.mmap[payload_end..].fill(0);
        }

        header.producer_sequence = producer_sequence;
        header.status_code = status_code;
        header.producer_ticks = current_ticks_millis();
        header.header_commit += 1;
        self.write_header(&header);
        self.flush()
    }

    pub fn try_read_latest(&mut self) -> IpcResult<Option<LatestSnapshot>> {
        let first_header = ChannelHeader::decode(&self.mmap[..CHANNEL_HEADER_SIZE])?;
        if first_header.header_commit % 2 != 0 {
            return Ok(None);
        }

        let length_start = CHANNEL_HEADER_SIZE;
        let payload_len = read_u32(&self.mmap, length_start) as usize;
        let payload_len = payload_len.min(
            self.capacity_bytes
                .saturating_sub(LATEST_PAYLOAD_LENGTH_SIZE),
        );
        let payload_start = first_header.payload_offset as usize;
        let payload_end = payload_start + payload_len;
        let payload = self.mmap[payload_start..payload_end].to_vec();

        let second_header = ChannelHeader::decode(&self.mmap[..CHANNEL_HEADER_SIZE])?;
        if first_header.header_commit != second_header.header_commit
            || second_header.header_commit % 2 != 0
        {
            return Ok(None);
        }

        Ok(Some(LatestSnapshot {
            header: second_header,
            payload,
        }))
    }

    fn write_header(&mut self, header: &ChannelHeader) {
        self.mmap[..CHANNEL_HEADER_SIZE].copy_from_slice(&header.encode());
    }

    fn flush(&mut self) -> IpcResult<()> {
        self.mmap.flush_async().map_err(|source| IpcError::Io {
            path: self.path.clone(),
            source,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpscQueueMetadata {
    pub item_capacity: u32,
    pub item_size: u32,
    pub write_sequence: u64,
    pub read_sequence: u64,
    pub dropped_count: u64,
    pub merged_count: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MouseLatest {
    pub x: i32,
    pub y: i32,
    pub buttons: u32,
    pub valid: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum InputEventKind {
    MouseButton = 1,
    MouseWheel = 2,
    Keyboard = 3,
    Ime = 4,
    Script = 5,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputEvent {
    pub kind: InputEventKind,
    pub sequence: u64,
    pub payload: Vec<u8>,
}

impl InputEvent {
    pub fn new(kind: InputEventKind, sequence: u64, payload: impl AsRef<[u8]>) -> Self {
        Self {
            kind,
            sequence,
            payload: payload.as_ref().to_vec(),
        }
    }
}

pub struct InputChannelState {
    mouse_latest: Option<MouseLatest>,
    events: VecDeque<InputEvent>,
    metadata: SpscQueueMetadata,
}

impl InputChannelState {
    pub fn new(event_capacity: u32) -> Self {
        Self {
            mouse_latest: None,
            events: VecDeque::with_capacity(event_capacity as usize),
            metadata: SpscQueueMetadata {
                item_capacity: event_capacity,
                item_size: 0,
                write_sequence: 0,
                read_sequence: 0,
                dropped_count: 0,
                merged_count: 0,
            },
        }
    }

    pub fn set_mouse_latest(&mut self, mouse: MouseLatest) {
        if self.mouse_latest.is_some() {
            self.metadata.merged_count = self.metadata.merged_count.saturating_add(1);
        }
        self.mouse_latest = Some(mouse);
    }

    pub fn mouse_latest(&self) -> Option<MouseLatest> {
        self.mouse_latest
    }

    pub fn push_event(&mut self, event: InputEvent) -> IpcResult<()> {
        if self.events.len() >= self.metadata.item_capacity as usize {
            self.metadata.dropped_count = self.metadata.dropped_count.saturating_add(1);
            return Err(IpcError::InputQueueFull {
                capacity: self.metadata.item_capacity as usize,
            });
        }

        self.metadata.write_sequence = self.metadata.write_sequence.saturating_add(1);
        self.events.push_back(event);
        Ok(())
    }

    pub fn pop_event(&mut self) -> IpcResult<Option<InputEvent>> {
        let event = self.events.pop_front();
        if event.is_some() {
            self.metadata.read_sequence = self.metadata.read_sequence.saturating_add(1);
        }
        Ok(event)
    }

    pub fn metadata(&self) -> SpscQueueMetadata {
        self.metadata
    }
}

impl SpscQueueMetadata {
    pub const BYTE_SIZE: usize = 64;

    pub fn queued_items(&self) -> u64 {
        self.write_sequence.saturating_sub(self.read_sequence)
    }

    pub fn available_slots(&self) -> u64 {
        u64::from(self.item_capacity).saturating_sub(self.queued_items())
    }

    pub fn encode(&self) -> [u8; Self::BYTE_SIZE] {
        let mut buffer = [0u8; Self::BYTE_SIZE];
        write_u32(&mut buffer, 0, self.item_capacity);
        write_u32(&mut buffer, 4, self.item_size);
        write_u64(&mut buffer, 8, self.write_sequence);
        write_u64(&mut buffer, 16, self.read_sequence);
        write_u64(&mut buffer, 24, self.dropped_count);
        write_u64(&mut buffer, 32, self.merged_count);
        buffer
    }

    pub fn decode(buffer: &[u8]) -> Result<Self, DecodeError> {
        require_len(buffer, Self::BYTE_SIZE)?;

        Ok(Self {
            item_capacity: read_u32(buffer, 0),
            item_size: read_u32(buffer, 4),
            write_sequence: read_u64(buffer, 8),
            read_sequence: read_u64(buffer, 16),
            dropped_count: read_u64(buffer, 24),
            merged_count: read_u64(buffer, 32),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameHeader {
    pub width: i32,
    pub height: i32,
    pub pixel_format: u32,
    pub slot_count: u32,
    pub slot_size: u32,
    pub current_slot: u32,
    pub frame_sequence: u64,
    pub acknowledged_frame: u64,
    pub frame_type: u32,
    pub rect_count: u32,
    pub payload_bytes: u32,
    pub dirty_header_size: u32,
    pub dropped_paint_count: u64,
    pub published_frame_count: u64,
    pub submitted_paint_count: u64,
    pub reserved: u64,
}

impl FrameHeader {
    pub fn encode(&self) -> [u8; FRAME_HEADER_SIZE] {
        let mut buffer = [0u8; FRAME_HEADER_SIZE];
        write_i32(&mut buffer, 0, self.width);
        write_i32(&mut buffer, 4, self.height);
        write_u32(&mut buffer, 8, self.pixel_format);
        write_u32(&mut buffer, 12, self.slot_count);
        write_u32(&mut buffer, 16, self.slot_size);
        write_u32(&mut buffer, 20, self.current_slot);
        write_u64(&mut buffer, 24, self.frame_sequence);
        write_u64(&mut buffer, 32, self.acknowledged_frame);
        write_u32(&mut buffer, 40, self.frame_type);
        write_u32(&mut buffer, 44, self.rect_count);
        write_u32(&mut buffer, 48, self.payload_bytes);
        write_u32(&mut buffer, 52, self.dirty_header_size);
        write_u64(&mut buffer, 56, self.dropped_paint_count);
        write_u64(&mut buffer, 64, self.published_frame_count);
        write_u64(&mut buffer, 72, self.submitted_paint_count);
        write_u64(&mut buffer, 80, self.reserved);
        buffer
    }

    pub fn decode(buffer: &[u8]) -> Result<Self, DecodeError> {
        require_len(buffer, FRAME_HEADER_SIZE)?;

        Ok(Self {
            width: read_i32(buffer, 0),
            height: read_i32(buffer, 4),
            pixel_format: read_u32(buffer, 8),
            slot_count: read_u32(buffer, 12),
            slot_size: read_u32(buffer, 16),
            current_slot: read_u32(buffer, 20),
            frame_sequence: read_u64(buffer, 24),
            acknowledged_frame: read_u64(buffer, 32),
            frame_type: read_u32(buffer, 40),
            rect_count: read_u32(buffer, 44),
            payload_bytes: read_u32(buffer, 48),
            dirty_header_size: read_u32(buffer, 52),
            dropped_paint_count: read_u64(buffer, 56),
            published_frame_count: read_u64(buffer, 64),
            submitted_paint_count: read_u64(buffer, 72),
            reserved: read_u64(buffer, 80),
        })
    }
}

fn require_len(buffer: &[u8], expected: usize) -> Result<(), DecodeError> {
    if buffer.len() < expected {
        Err(DecodeError::BufferTooSmall {
            expected,
            actual: buffer.len(),
        })
    } else {
        Ok(())
    }
}

fn write_u16(buffer: &mut [u8], offset: usize, value: u16) {
    buffer[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(buffer: &mut [u8], offset: usize, value: u32) {
    buffer[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_i32(buffer: &mut [u8], offset: usize, value: i32) {
    buffer[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(buffer: &mut [u8], offset: usize, value: u64) {
    buffer[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_i64(buffer: &mut [u8], offset: usize, value: i64) {
    buffer[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn read_u16(buffer: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(buffer[offset..offset + 2].try_into().expect("slice length"))
}

fn read_u32(buffer: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(buffer[offset..offset + 4].try_into().expect("slice length"))
}

fn read_i32(buffer: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(buffer[offset..offset + 4].try_into().expect("slice length"))
}

fn read_u64(buffer: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(buffer[offset..offset + 8].try_into().expect("slice length"))
}

fn read_i64(buffer: &[u8], offset: usize) -> i64 {
    i64::from_le_bytes(buffer[offset..offset + 8].try_into().expect("slice length"))
}

fn open_channel_file(path: &Path, mode: ChannelOpenMode) -> IpcResult<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    if mode == ChannelOpenMode::Create {
        options.create(true).truncate(true);
    }

    options.open(path).map_err(|source| IpcError::Io {
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
    if next % 2 == 0 {
        next.saturating_add(1)
    } else {
        next
    }
}

fn current_ticks_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}
