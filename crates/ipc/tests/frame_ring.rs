use ipc::{FramePublishResult, FrameRingState, FrameType};

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
fn drops_same_size_frame_when_previous_frame_is_unacknowledged() {
    let mut ring = FrameRingState::new(2, 16);

    assert!(ring.publish_full(1, 2, 2, &[1]).published());
    let result = ring.publish_full(2, 2, 2, &[2]);

    assert_eq!(result, FramePublishResult::Dropped);
    assert_eq!(ring.submitted_count(), 2);
    assert_eq!(ring.published_count(), 1);
    assert_eq!(ring.dropped_count(), 1);
    assert_eq!(ring.latest_frame().unwrap().sequence, 1);
    assert_eq!(ring.latest_frame().unwrap().pixels, vec![1]);
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
