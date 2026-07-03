use ipc::{
    build_session_channel_name, ChannelKind, ChannelMappedFile, ChannelOpenMode, ChannelSpec,
};

#[test]
fn mapped_channel_creates_file_with_header_and_payload_capacity() {
    let temp = tempfile::tempdir().unwrap();
    let name = build_session_channel_name("session-42", ChannelKind::Status);
    let spec = ChannelSpec::new(name, ChannelKind::Status, 128);

    let channel = ChannelMappedFile::open_in_dir(temp.path(), &spec, ChannelOpenMode::Create)
        .expect("create mapped channel");

    assert_eq!(channel.capacity_bytes(), 128);
    assert_eq!(channel.header().channel_kind, ChannelKind::Status);
    assert_eq!(channel.header().capacity_bytes, 128);
    assert_eq!(channel.header().payload_offset, 68);
    assert_eq!(std::fs::metadata(channel.path()).unwrap().len(), 64 + 128);
}

#[test]
fn mapped_channel_publishes_and_reads_latest_payload_with_seqlock_commit() {
    let temp = tempfile::tempdir().unwrap();
    let name = build_session_channel_name("session-42", ChannelKind::Status);
    let spec = ChannelSpec::new(name, ChannelKind::Status, 128);

    let mut writer =
        ChannelMappedFile::open_in_dir(temp.path(), &spec, ChannelOpenMode::Create).unwrap();
    let mut reader =
        ChannelMappedFile::open_in_dir(temp.path(), &spec, ChannelOpenMode::OpenExisting).unwrap();

    writer.publish_latest(7, 99, b"ready").unwrap();

    let snapshot = reader.try_read_latest().unwrap().expect("latest payload");
    assert_eq!(snapshot.header.channel_kind, ChannelKind::Status);
    assert_eq!(snapshot.header.capacity_bytes, 128);
    assert_eq!(snapshot.header.producer_sequence, 7);
    assert_eq!(snapshot.header.status_code, 99);
    assert_eq!(snapshot.payload, b"ready");
}

#[test]
fn mapped_channel_rejects_payload_larger_than_capacity() {
    let temp = tempfile::tempdir().unwrap();
    let name = build_session_channel_name("session-42", ChannelKind::Status);
    let spec = ChannelSpec::new(name, ChannelKind::Status, 8);
    let mut channel = ChannelMappedFile::open_in_dir(temp.path(), &spec, ChannelOpenMode::Create)
        .expect("create mapped channel");

    let error = channel.publish_latest(1, 0, b"too-large").unwrap_err();

    assert!(error
        .to_string()
        .contains("payload exceeds channel capacity"));
}
