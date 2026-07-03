use ipc::{
    build_browser_channel_name, ChannelKind, ChannelOpenMode, FrameChannel, FrameChannelSpec,
    FrameCopyError, FramePublishResult, FrameRingState, FrameType, CHANNEL_HEADER_SIZE,
    FRAME_HEADER_SIZE,
};

#[test]
fn publishes_full_frame_into_latest_slot() {
    let mut ring = FrameRingState::new(2, 16);

    let result = ring.publish_full(1, 2, 2, &[1, 2, 3, 4]);

    assert_eq!(result, FramePublishResult::Published { slot_index: 0 });
    assert_eq!(ring.submitted_count(), 1);
    assert_eq!(ring.published_count(), 1);
    assert_eq!(ring.dropped_count(), 0);

    let frame = ring.latest_frame().expect("latest frame");
    assert_eq!(frame.sequence, 1);
    assert_eq!(frame.width, 2);
    assert_eq!(frame.height, 2);
    assert_eq!(frame.frame_type, FrameType::Full);
    assert_eq!(frame.pixels, vec![1, 2, 3, 4]);
}

#[test]
fn latest_frame_replaces_same_size_frame_even_when_previous_frame_is_unacknowledged() {
    let mut ring = FrameRingState::new(2, 16);

    assert!(ring.publish_full(1, 2, 2, &[1]).published());
    let result = ring.publish_full(2, 2, 2, &[2]);

    assert_eq!(result, FramePublishResult::Published { slot_index: 1 });
    assert_eq!(ring.submitted_count(), 2);
    assert_eq!(ring.published_count(), 2);
    assert_eq!(ring.dropped_count(), 1);
    assert_eq!(ring.latest_frame().unwrap().sequence, 2);
    assert_eq!(ring.latest_frame().unwrap().pixels, vec![2]);
}

#[test]
fn publishes_same_size_frame_after_acknowledgement() {
    let mut ring = FrameRingState::new(2, 16);

    assert!(ring.publish_full(1, 2, 2, &[1]).published());
    ring.ack(1);
    let result = ring.publish_full(2, 2, 2, &[2]);

    assert_eq!(result, FramePublishResult::Published { slot_index: 1 });
    assert_eq!(ring.acknowledged_sequence(), 1);
    assert_eq!(ring.published_count(), 2);
    assert_eq!(ring.dropped_count(), 0);
    assert_eq!(ring.latest_frame().unwrap().sequence, 2);
    assert_eq!(ring.latest_frame().unwrap().pixels, vec![2]);
}

#[test]
fn resize_interrupts_backpressure_waiting_for_acknowledgement() {
    let mut ring = FrameRingState::new(2, 16);

    assert!(ring.publish_full(1, 2, 2, &[1]).published());
    let result = ring.publish_full(2, 4, 2, &[2, 3]);

    assert_eq!(result, FramePublishResult::Published { slot_index: 1 });
    assert_eq!(ring.submitted_count(), 2);
    assert_eq!(ring.published_count(), 2);
    assert_eq!(ring.dropped_count(), 0);

    let frame = ring.latest_frame().expect("latest frame");
    assert_eq!(frame.sequence, 2);
    assert_eq!(frame.width, 4);
    assert_eq!(frame.height, 2);
    assert_eq!(frame.frame_type, FrameType::Resize);
    assert_eq!(frame.pixels, vec![2, 3]);
}

#[test]
fn frame_channel_returns_zero_bytes_when_no_frame_is_available() {
    let temp = tempfile::tempdir().unwrap();
    let mut channel = FrameChannel::open_in_dir(
        temp.path(),
        &frame_spec("session-42", "browser-A", 2, 64),
        ChannelOpenMode::Create,
    )
    .unwrap();
    let mut buffer = [0u8; 64];

    let result = channel.try_copy_latest(&mut buffer).unwrap();

    assert_eq!(result.width, 0);
    assert_eq!(result.height, 0);
    assert_eq!(result.sequence, 0);
    assert_eq!(result.written, 0);
}

#[test]
fn frame_channel_reports_required_length_when_target_buffer_is_too_small() {
    let temp = tempfile::tempdir().unwrap();
    let mut channel = FrameChannel::open_in_dir(
        temp.path(),
        &frame_spec("session-42", "browser-A", 2, 64),
        ChannelOpenMode::Create,
    )
    .unwrap();
    channel.publish_full(7, 2, 2, &[1u8; 16]).unwrap();
    let mut buffer = [0u8; 15];

    let result = channel.try_copy_latest(&mut buffer);

    assert_eq!(result, Err(FrameCopyError::BufferTooSmall { required: 16 }));
}

#[test]
fn frame_channel_copies_latest_frame_metadata_and_pixels() {
    let temp = tempfile::tempdir().unwrap();
    let mut channel = FrameChannel::open_in_dir(
        temp.path(),
        &frame_spec("session-42", "browser-A", 2, 64),
        ChannelOpenMode::Create,
    )
    .unwrap();
    channel.publish_full(7, 2, 2, &[1, 2, 3, 4]).unwrap();
    let mut buffer = [0u8; 64];

    let result = channel.try_copy_latest(&mut buffer).unwrap();

    assert_eq!(result.width, 2);
    assert_eq!(result.height, 2);
    assert_eq!(result.sequence, 7);
    assert_eq!(result.written, 4);
    assert_eq!(&buffer[..4], &[1, 2, 3, 4]);
}

#[test]
fn frame_channel_publish_does_not_clear_bytes_after_payload() {
    let temp = tempfile::tempdir().unwrap();
    let mut channel = FrameChannel::open_in_dir(
        temp.path(),
        &frame_spec("session-42", "browser-A", 2, 64),
        ChannelOpenMode::Create,
    )
    .unwrap();

    let first = channel.publish_full(7, 2, 2, &[9u8; 64]).unwrap();
    assert_eq!(first, FramePublishResult::Published { slot_index: 1 });
    channel.ack(7).unwrap();
    let second = channel.publish_full(8, 2, 2, &[8u8; 64]).unwrap();
    assert_eq!(second, FramePublishResult::Published { slot_index: 0 });
    channel.ack(8).unwrap();
    let third = channel.publish_full(9, 2, 2, &[1, 2, 3, 4]).unwrap();
    assert_eq!(third, FramePublishResult::Published { slot_index: 1 });
    let path = channel.path().to_path_buf();
    drop(channel);

    let bytes = std::fs::read(path).unwrap();
    let second_slot_start = CHANNEL_HEADER_SIZE + FRAME_HEADER_SIZE + 64;

    assert_eq!(
        &bytes[second_slot_start..second_slot_start + 4],
        &[1, 2, 3, 4]
    );
    assert_eq!(
        &bytes[second_slot_start + 4..second_slot_start + 8],
        &[9, 9, 9, 9]
    );
}

#[test]
fn frame_channel_publish_advances_even_commit_after_writing_frame() {
    let temp = tempfile::tempdir().unwrap();
    let mut channel = FrameChannel::open_in_dir(
        temp.path(),
        &frame_spec("session-42", "browser-A", 2, 64),
        ChannelOpenMode::Create,
    )
    .unwrap();

    assert_eq!(channel.header().header_commit, 0);
    channel.publish_full(7, 2, 2, &[1, 2, 3, 4]).unwrap();

    let commit = channel.header().header_commit;
    assert_eq!(commit, 2);
    assert!(commit.is_multiple_of(2));
}

#[test]
fn frame_channel_copy_skips_frame_when_commit_is_in_progress() {
    let temp = tempfile::tempdir().unwrap();
    let mut channel = FrameChannel::open_in_dir(
        temp.path(),
        &frame_spec("session-42", "browser-A", 2, 64),
        ChannelOpenMode::Create,
    )
    .unwrap();
    channel.publish_full(7, 2, 2, &[1, 2, 3, 4]).unwrap();
    let mut header = channel.header();
    header.header_commit = 3;
    let path = channel.path().to_path_buf();
    drop(channel);
    {
        use std::io::{Seek, Write};

        let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.seek(std::io::SeekFrom::Start(0)).unwrap();
        file.write_all(&header.encode()).unwrap();
    }
    let mut channel = FrameChannel::open_in_dir(
        temp.path(),
        &frame_spec("session-42", "browser-A", 2, 64),
        ChannelOpenMode::OpenExisting,
    )
    .unwrap();
    let mut buffer = [9u8; 64];

    let result = channel.try_copy_latest(&mut buffer).unwrap();

    assert_eq!(result.width, 0);
    assert_eq!(result.height, 0);
    assert_eq!(result.sequence, 0);
    assert_eq!(result.written, 0);
    assert_eq!(&buffer[..4], &[9, 9, 9, 9]);
}

#[test]
fn frame_channel_ack_updates_consumer_ack() {
    let temp = tempfile::tempdir().unwrap();
    let mut channel = FrameChannel::open_in_dir(
        temp.path(),
        &frame_spec("session-42", "browser-A", 2, 64),
        ChannelOpenMode::Create,
    )
    .unwrap();
    channel.publish_full(7, 2, 2, &[1, 2, 3, 4]).unwrap();

    channel.ack(7).unwrap();

    assert_eq!(channel.header().consumer_ack, 7);
    assert_eq!(channel.frame_header().acknowledged_frame, 7);
}

fn frame_spec(
    session_id: &str,
    browser_id: &str,
    slot_count: u32,
    slot_size: u32,
) -> FrameChannelSpec {
    FrameChannelSpec::new(
        build_browser_channel_name(session_id, browser_id, ChannelKind::Frame),
        slot_count,
        slot_size,
    )
}
