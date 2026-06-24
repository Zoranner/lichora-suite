use ipc::{
    build_session_channel_name, ChannelKind, ChannelOpenMode, MappedQueueError, MappedQueueItem,
    MappedQueueSpec, MappedSpscQueue,
};

fn queue_spec(item_capacity: u32, max_payload_len: u32) -> MappedQueueSpec {
    MappedQueueSpec::new(
        build_session_channel_name("session-42", ChannelKind::Control),
        ChannelKind::Control,
        item_capacity,
        max_payload_len,
    )
}

#[test]
fn mapped_queue_preserves_fifo_order() {
    let temp = tempfile::tempdir().unwrap();
    let spec = queue_spec(4, 16);
    let mut queue =
        MappedSpscQueue::open_in_dir(temp.path(), &spec, ChannelOpenMode::Create).unwrap();

    queue
        .try_push(MappedQueueItem::new(10, 1, b"add"))
        .expect("push first item");
    queue
        .try_push(MappedQueueItem::new(20, 2, b"resize"))
        .expect("push second item");

    assert_eq!(
        queue.try_pop().unwrap(),
        Some(MappedQueueItem::new(10, 1, b"add"))
    );
    assert_eq!(
        queue.try_pop().unwrap(),
        Some(MappedQueueItem::new(20, 2, b"resize"))
    );
    assert_eq!(queue.try_pop().unwrap(), None);
}

#[test]
fn mapped_queue_reports_full_queue_and_increments_dropped_count() {
    let temp = tempfile::tempdir().unwrap();
    let spec = queue_spec(1, 8);
    let mut queue =
        MappedSpscQueue::open_in_dir(temp.path(), &spec, ChannelOpenMode::Create).unwrap();

    queue
        .try_push(MappedQueueItem::new(1, 1, b"first"))
        .expect("push first item");

    let error = queue
        .try_push(MappedQueueItem::new(2, 2, b"second"))
        .unwrap_err();

    assert_eq!(error, MappedQueueError::QueueFull { capacity: 1 });
    assert_eq!(queue.metadata().dropped_count, 1);
    assert_eq!(
        queue.try_pop().unwrap(),
        Some(MappedQueueItem::new(1, 1, b"first"))
    );
}

#[test]
fn mapped_queue_can_be_reopened_before_consuming_items() {
    let temp = tempfile::tempdir().unwrap();
    let spec = queue_spec(2, 16);
    {
        let mut producer =
            MappedSpscQueue::open_in_dir(temp.path(), &spec, ChannelOpenMode::Create).unwrap();
        producer
            .try_push(MappedQueueItem::new(100, 7, b"persist"))
            .expect("push persistent item");
    }

    let mut consumer =
        MappedSpscQueue::open_in_dir(temp.path(), &spec, ChannelOpenMode::OpenExisting).unwrap();

    assert_eq!(
        consumer.try_pop().unwrap(),
        Some(MappedQueueItem::new(100, 7, b"persist"))
    );
    assert_eq!(consumer.try_pop().unwrap(), None);
}

#[test]
fn mapped_queue_rejects_payload_larger_than_fixed_slot_capacity() {
    let temp = tempfile::tempdir().unwrap();
    let spec = queue_spec(2, 4);
    let mut queue =
        MappedSpscQueue::open_in_dir(temp.path(), &spec, ChannelOpenMode::Create).unwrap();

    let error = queue
        .try_push(MappedQueueItem::new(1, 1, b"12345"))
        .unwrap_err();

    assert_eq!(
        error,
        MappedQueueError::PayloadTooLarge {
            payload: 5,
            capacity: 4
        }
    );
    assert_eq!(queue.metadata().dropped_count, 0);
}
