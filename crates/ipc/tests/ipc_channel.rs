use ipc::{
    build_browser_channel_name, build_session_channel_name, ChannelKind, InputChannelState,
    InputEvent, InputEventKind, MouseLatest, SpscQueueMetadata,
};

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

#[test]
fn input_channel_mouse_latest_overwrites_previous_position() {
    let mut input = InputChannelState::new(2);

    input.set_mouse_latest(MouseLatest {
        x: 10,
        y: 20,
        buttons: 1,
        valid: true,
    });
    input.set_mouse_latest(MouseLatest {
        x: 30,
        y: 40,
        buttons: 0,
        valid: true,
    });

    assert_eq!(
        input.mouse_latest(),
        Some(MouseLatest {
            x: 30,
            y: 40,
            buttons: 0,
            valid: true,
        })
    );
    assert_eq!(input.metadata().merged_count, 1);
}

#[test]
fn input_channel_event_queue_preserves_fifo_order() {
    let mut input = InputChannelState::new(2);

    input
        .push_event(InputEvent::new(InputEventKind::MouseButton, 1, b"down"))
        .unwrap();
    input
        .push_event(InputEvent::new(InputEventKind::Keyboard, 2, b"key"))
        .unwrap();

    assert_eq!(
        input.pop_event().unwrap(),
        Some(InputEvent::new(InputEventKind::MouseButton, 1, b"down"))
    );
    assert_eq!(
        input.pop_event().unwrap(),
        Some(InputEvent::new(InputEventKind::Keyboard, 2, b"key"))
    );
    assert_eq!(input.pop_event().unwrap(), None);
}

#[test]
fn input_channel_event_queue_reports_overflow() {
    let mut input = InputChannelState::new(1);
    input
        .push_event(InputEvent::new(InputEventKind::MouseButton, 1, b"down"))
        .unwrap();

    let error = input
        .push_event(InputEvent::new(InputEventKind::Keyboard, 2, b"key"))
        .unwrap_err();

    assert!(error.to_string().contains("input event queue is full"));
    assert_eq!(input.metadata().dropped_count, 1);
}
