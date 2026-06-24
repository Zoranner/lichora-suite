pub const CHANNEL_MAGIC: u32 = 0x3249_4245;
pub const PROTOCOL_VERSION_MAJOR: u16 = 2;
pub const PROTOCOL_VERSION_MINOR: u16 = 0;
pub const CHANNEL_HEADER_SIZE: usize = 64;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpscQueueMetadata {
    pub item_capacity: u32,
    pub item_size: u32,
    pub write_sequence: u64,
    pub read_sequence: u64,
    pub dropped_count: u64,
    pub merged_count: u64,
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
