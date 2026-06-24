use ipc::{build_browser_channel_name, build_session_channel_name, ChannelKind, SpscQueueMetadata};

#[test]
fn channel_names_match_embedded_browser_prefix_and_scope() {
    assert_eq!(
        build_session_channel_name("session-42", ChannelKind::Control),
        "EmbeddedBrowser_session-42_control"
    );
    assert_eq!(
        build_browser_channel_name("session-42", "browser-A", ChannelKind::Frame),
        "EmbeddedBrowser_session-42_browser-A_frame"
    );
}

#[test]
fn spsc_queue_metadata_reports_layout_and_queue_depth() {
    let metadata = SpscQueueMetadata {
        item_capacity: 8,
        item_size: 32,
        write_sequence: 11,
        read_sequence: 3,
        dropped_count: 2,
        merged_count: 1,
    };

    assert_eq!(SpscQueueMetadata::BYTE_SIZE, 64);
    assert_eq!(metadata.queued_items(), 8);
    assert_eq!(metadata.available_slots(), 0);

    let encoded = metadata.encode();
    assert_eq!(encoded.len(), SpscQueueMetadata::BYTE_SIZE);
    assert_eq!(&encoded[0..4], &8u32.to_le_bytes());
    assert_eq!(&encoded[8..16], &11u64.to_le_bytes());
    assert_eq!(&encoded[24..32], &2u64.to_le_bytes());
    assert_eq!(&encoded[40..64], &[0u8; 24]);

    assert_eq!(SpscQueueMetadata::decode(&encoded).unwrap(), metadata);
}
