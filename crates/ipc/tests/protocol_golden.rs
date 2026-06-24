use ipc::{
    ChannelHeader, ChannelKind, DecodeError, FrameHeader, CHANNEL_HEADER_SIZE, CHANNEL_MAGIC,
    FRAME_HEADER_SIZE, FRAME_PIXEL_FORMAT_BGRA32, PROTOCOL_VERSION_MAJOR,
};

#[test]
fn channel_header_encodes_and_decodes_64_byte_little_endian_layout() {
    let header = ChannelHeader {
        version_minor: 7,
        channel_kind: ChannelKind::Frame,
        flags: 0x10,
        capacity_bytes: 4096,
        producer_sequence: 0x0102_0304_0506_0708,
        consumer_ack: 0x1112_1314_1516_1718,
        producer_ticks: -42,
        status_code: -9,
        payload_offset: 128,
        header_commit: 44,
    };

    let encoded = header.encode();

    assert_eq!(encoded.len(), CHANNEL_HEADER_SIZE);
    assert_eq!(&encoded[0..4], &CHANNEL_MAGIC.to_le_bytes());
    assert_eq!(&encoded[4..6], &PROTOCOL_VERSION_MAJOR.to_le_bytes());
    assert_eq!(&encoded[6..8], &7u16.to_le_bytes());
    assert_eq!(&encoded[8..12], &(CHANNEL_HEADER_SIZE as u32).to_le_bytes());
    assert_eq!(&encoded[12..16], &(ChannelKind::Frame as u32).to_le_bytes());
    assert_eq!(&encoded[24..32], &0x0102_0304_0506_0708u64.to_le_bytes());
    assert_eq!(&encoded[32..40], &0x1112_1314_1516_1718u64.to_le_bytes());
    assert_eq!(&encoded[40..48], &(-42i64).to_le_bytes());
    assert_eq!(&encoded[48..52], &(-9i32).to_le_bytes());
    assert_eq!(&encoded[56..64], &44u64.to_le_bytes());

    assert_eq!(ChannelHeader::decode(&encoded).unwrap(), header);
}

#[test]
fn channel_header_rejects_invalid_magic_version_header_size_and_kind() {
    let encoded = ChannelHeader::new(ChannelKind::Session, 1024).encode();

    let mut invalid_magic = encoded;
    invalid_magic[0..4].copy_from_slice(&0u32.to_le_bytes());
    assert_eq!(
        ChannelHeader::decode(&invalid_magic).unwrap_err(),
        DecodeError::InvalidMagic(0)
    );

    let mut invalid_version = encoded;
    invalid_version[4..6].copy_from_slice(&3u16.to_le_bytes());
    assert_eq!(
        ChannelHeader::decode(&invalid_version).unwrap_err(),
        DecodeError::UnsupportedVersion { major: 3, minor: 0 }
    );

    let mut invalid_header_size = encoded;
    invalid_header_size[8..12].copy_from_slice(&32u32.to_le_bytes());
    assert_eq!(
        ChannelHeader::decode(&invalid_header_size).unwrap_err(),
        DecodeError::InvalidHeaderSize(32)
    );

    let mut invalid_kind = encoded;
    invalid_kind[12..16].copy_from_slice(&99u32.to_le_bytes());
    assert_eq!(
        ChannelHeader::decode(&invalid_kind).unwrap_err(),
        DecodeError::InvalidChannelKind(99)
    );

    assert_eq!(
        ChannelHeader::decode(&encoded[..16]).unwrap_err(),
        DecodeError::BufferTooSmall {
            expected: CHANNEL_HEADER_SIZE,
            actual: 16
        }
    );
}

#[test]
fn frame_header_encodes_and_decodes_base_frame_metadata() {
    let frame = FrameHeader {
        width: 1920,
        height: 1080,
        pixel_format: FRAME_PIXEL_FORMAT_BGRA32,
        slot_count: 3,
        slot_size: 1920 * 1080 * 4,
        current_slot: 2,
        frame_sequence: 91,
        acknowledged_frame: 90,
        frame_type: 1,
        rect_count: 0,
        payload_bytes: 1920 * 1080 * 4,
        dirty_header_size: 0,
        dropped_paint_count: 4,
        published_frame_count: 50,
        submitted_paint_count: 54,
        reserved: 0,
    };

    let encoded = frame.encode();

    assert_eq!(encoded.len(), FRAME_HEADER_SIZE);
    assert_eq!(&encoded[0..4], &1920i32.to_le_bytes());
    assert_eq!(&encoded[4..8], &1080i32.to_le_bytes());
    assert_eq!(&encoded[8..12], &FRAME_PIXEL_FORMAT_BGRA32.to_le_bytes());
    assert_eq!(&encoded[56..64], &4u64.to_le_bytes());
    assert_eq!(&encoded[80..88], &0u64.to_le_bytes());
    assert_eq!(FrameHeader::decode(&encoded).unwrap(), frame);
}
