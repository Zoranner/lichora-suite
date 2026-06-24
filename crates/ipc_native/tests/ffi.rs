use ipc::{
    build_browser_channel_name, build_session_channel_name, ChannelKind, ChannelMappedFile,
    ChannelOpenMode, ChannelSpec, ControlCommand, MappedQueueSpec, MappedSpscQueue,
};
use ipc_native::{
    ebi_control_add_browser, ebi_control_send, ebi_error_message, ebi_input_push_event,
    ebi_input_set_mouse_latest, ebi_output_try_read, ebi_session_close, ebi_session_open,
    ebi_status_read, EbiSessionHandle, EBI_ERROR_BUFFER_TOO_SMALL, EBI_ERROR_INVALID_ARGUMENT,
    EBI_ERROR_NOT_IMPLEMENTED, EBI_ERROR_QUEUE_FULL, EBI_OK,
};
use std::ffi::CStr;
use std::ptr;
use std::sync::{Mutex, MutexGuard};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn error_message_returns_static_c_strings_for_known_codes() {
    let ok = unsafe { CStr::from_ptr(ebi_error_message(EBI_OK)) };
    let not_implemented = unsafe { CStr::from_ptr(ebi_error_message(EBI_ERROR_NOT_IMPLEMENTED)) };

    assert_eq!(ok.to_str().unwrap(), "ok");
    assert_eq!(not_implemented.to_str().unwrap(), "not implemented");
    assert!(ebi_error_message(999).is_null());
}

#[test]
fn session_open_validates_output_pointer_and_returns_opaque_handle() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());

    assert_eq!(
        ebi_session_open(ptr::null(), 0, ptr::null_mut()),
        EBI_ERROR_INVALID_ARGUMENT
    );

    let session_id = b"session-42";
    let mut handle: EbiSessionHandle = ptr::null_mut();
    let result = ebi_session_open(session_id.as_ptr(), session_id.len(), &mut handle);

    assert_eq!(result, EBI_OK);
    assert!(!handle.is_null());
    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn session_open_creates_session_control_and_status_channel_files() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let session_id = b"session-42";
    let mut handle: EbiSessionHandle = ptr::null_mut();

    let result = ebi_session_open(session_id.as_ptr(), session_id.len(), &mut handle);

    assert_eq!(result, EBI_OK);
    assert!(temp
        .path()
        .join("EmbeddedBrowser_session-42_session")
        .exists());
    assert!(temp
        .path()
        .join("EmbeddedBrowser_session-42_control")
        .exists());
    assert!(temp
        .path()
        .join("EmbeddedBrowser_session-42_status")
        .exists());
    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn control_send_enqueues_payload_to_control_queue() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let session_id = b"session-42";
    let mut handle: EbiSessionHandle = ptr::null_mut();
    assert_eq!(
        ebi_session_open(session_id.as_ptr(), session_id.len(), &mut handle),
        EBI_OK
    );

    let payload = b"add-browser";
    assert_eq!(
        ebi_control_send(handle, 11, payload.as_ptr(), payload.len()),
        EBI_OK
    );

    let mut queue = MappedSpscQueue::open_in_dir(
        temp.path(),
        &control_queue_spec(),
        ChannelOpenMode::OpenExisting,
    )
    .unwrap();
    let item = queue.try_pop().unwrap().expect("control queue item");
    assert_eq!(item.kind, 0);
    assert_eq!(item.sequence, 11);
    assert_eq!(item.payload, payload);
    assert_eq!(queue.try_pop().unwrap(), None);
    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn control_add_browser_encodes_command_on_rust_side_and_enqueues_it() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    let browser_id = b"browser-A";
    let address = b"https://example.test";

    assert_eq!(
        ebi_control_add_browser(
            handle,
            12,
            browser_id.as_ptr(),
            browser_id.len(),
            1280,
            720,
            address.as_ptr(),
            address.len()
        ),
        EBI_OK
    );

    let mut queue = MappedSpscQueue::open_in_dir(
        temp.path(),
        &control_queue_spec(),
        ChannelOpenMode::OpenExisting,
    )
    .unwrap();
    let item = queue.try_pop().unwrap().expect("typed control queue item");
    assert_eq!(item.kind, 1);
    assert_eq!(item.sequence, 12);
    assert_eq!(
        ControlCommand::decode(&item.payload).unwrap(),
        ControlCommand::AddBrowser {
            browser_id: "browser-A".to_string(),
            width: 1280,
            height: 720,
            address: "https://example.test".to_string(),
        }
    );
    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn control_add_browser_validates_string_arguments() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    let browser_id = b"browser-A";
    let address = b"https://example.test";

    assert_eq!(
        ebi_control_add_browser(
            ptr::null_mut(),
            1,
            browser_id.as_ptr(),
            browser_id.len(),
            1280,
            720,
            address.as_ptr(),
            address.len()
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_control_add_browser(
            handle,
            1,
            ptr::null(),
            browser_id.len(),
            1280,
            720,
            address.as_ptr(),
            address.len()
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_control_add_browser(
            handle,
            1,
            browser_id.as_ptr(),
            0,
            1280,
            720,
            address.as_ptr(),
            address.len()
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_control_add_browser(
            handle,
            1,
            b"../bad".as_ptr(),
            b"../bad".len(),
            1280,
            720,
            address.as_ptr(),
            address.len()
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_control_add_browser(
            handle,
            1,
            browser_id.as_ptr(),
            browser_id.len(),
            1280,
            720,
            ptr::null(),
            address.len()
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_control_add_browser(
            handle,
            1,
            [0xff, 0xfe].as_ptr(),
            2,
            1280,
            720,
            address.as_ptr(),
            address.len()
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );

    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn control_send_reports_queue_full_without_overwriting_existing_commands() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    let payload = b"command";

    for sequence in 0..256 {
        assert_eq!(
            ebi_control_send(handle, sequence, payload.as_ptr(), payload.len()),
            EBI_OK
        );
    }

    assert_eq!(
        ebi_control_send(handle, 257, payload.as_ptr(), payload.len()),
        EBI_ERROR_QUEUE_FULL
    );

    let mut queue = MappedSpscQueue::open_in_dir(
        temp.path(),
        &control_queue_spec(),
        ChannelOpenMode::OpenExisting,
    )
    .unwrap();
    assert_eq!(queue.metadata().dropped_count, 1);
    assert_eq!(queue.try_pop().unwrap().unwrap().sequence, 0);
    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn input_set_mouse_latest_accepts_valid_handle() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();

    assert_eq!(ebi_input_set_mouse_latest(handle, 10, 20, 1, 1), EBI_OK);

    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn input_push_event_accepts_payload() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    let payload = b"key";

    assert_eq!(
        ebi_input_push_event(handle, 3, 12, payload.as_ptr(), payload.len()),
        EBI_OK
    );

    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn status_read_returns_zero_bytes_when_no_status_payload_is_available() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    let mut buffer = [0u8; 16];
    let mut written = usize::MAX;

    assert_eq!(
        ebi_status_read(handle, buffer.as_mut_ptr(), buffer.len(), &mut written),
        EBI_OK
    );

    assert_eq!(written, 0);
    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn status_read_copies_latest_status_payload() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    publish_session_payload(
        temp.path(),
        "session-42",
        ChannelKind::Status,
        4,
        b"{\"ok\":true}",
    );
    let mut buffer = [0u8; 32];
    let mut written = 0;

    assert_eq!(
        ebi_status_read(handle, buffer.as_mut_ptr(), buffer.len(), &mut written),
        EBI_OK
    );

    assert_eq!(written, b"{\"ok\":true}".len());
    assert_eq!(&buffer[..written], b"{\"ok\":true}");
    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn status_read_reports_required_length_when_buffer_is_too_small() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    publish_session_payload(temp.path(), "session-42", ChannelKind::Status, 4, b"ready");
    let mut buffer = [0u8; 4];
    let mut written = 0;

    assert_eq!(
        ebi_status_read(handle, buffer.as_mut_ptr(), buffer.len(), &mut written),
        EBI_ERROR_BUFFER_TOO_SMALL
    );

    assert_eq!(written, b"ready".len());
    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn status_read_validates_pointers_and_empty_buffer() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    let mut buffer = [0u8; 4];
    let mut written = 0;

    assert_eq!(
        ebi_status_read(
            ptr::null_mut(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut written
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_status_read(handle, ptr::null_mut(), buffer.len(), &mut written),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_status_read(handle, buffer.as_mut_ptr(), buffer.len(), ptr::null_mut()),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_status_read(handle, buffer.as_mut_ptr(), 0, &mut written),
        EBI_ERROR_INVALID_ARGUMENT
    );

    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn output_try_read_copies_latest_browser_output_payload() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    publish_browser_payload(
        temp.path(),
        "session-42",
        "browser-A",
        ChannelKind::Output,
        7,
        b"console:ready",
    );
    let browser_id = b"browser-A";
    let mut buffer = [0u8; 32];
    let mut written = 0;

    assert_eq!(
        ebi_output_try_read(
            handle,
            browser_id.as_ptr(),
            browser_id.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut written
        ),
        EBI_OK
    );

    assert_eq!(written, b"console:ready".len());
    assert_eq!(&buffer[..written], b"console:ready");
    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn output_try_read_returns_zero_bytes_when_channel_or_payload_is_absent() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    let browser_id = b"browser-A";
    let mut buffer = [0u8; 16];
    let mut written = usize::MAX;

    assert_eq!(
        ebi_output_try_read(
            handle,
            browser_id.as_ptr(),
            browser_id.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut written
        ),
        EBI_OK
    );

    assert_eq!(written, 0);
    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn output_try_read_reports_required_length_when_buffer_is_too_small() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    publish_browser_payload(
        temp.path(),
        "session-42",
        "browser-A",
        ChannelKind::Output,
        7,
        b"payload",
    );
    let browser_id = b"browser-A";
    let mut buffer = [0u8; 4];
    let mut written = 0;

    assert_eq!(
        ebi_output_try_read(
            handle,
            browser_id.as_ptr(),
            browser_id.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut written
        ),
        EBI_ERROR_BUFFER_TOO_SMALL
    );

    assert_eq!(written, b"payload".len());
    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn output_try_read_validates_pointers_empty_buffer_and_browser_id() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    let browser_id = b"browser-A";
    let mut buffer = [0u8; 4];
    let mut written = 0;

    assert_eq!(
        ebi_output_try_read(
            ptr::null_mut(),
            browser_id.as_ptr(),
            browser_id.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut written
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_output_try_read(
            handle,
            ptr::null(),
            browser_id.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut written
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_output_try_read(
            handle,
            browser_id.as_ptr(),
            browser_id.len(),
            ptr::null_mut(),
            buffer.len(),
            &mut written
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_output_try_read(
            handle,
            browser_id.as_ptr(),
            browser_id.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            ptr::null_mut()
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_output_try_read(
            handle,
            browser_id.as_ptr(),
            browser_id.len(),
            buffer.as_mut_ptr(),
            0,
            &mut written
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_output_try_read(
            handle,
            browser_id.as_ptr(),
            0,
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut written
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_output_try_read(
            handle,
            [0xff, 0xfe].as_ptr(),
            2,
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut written
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );

    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn session_close_accepts_null_as_noop() {
    assert_eq!(ebi_session_close(ptr::null_mut()), EBI_OK);
}

#[test]
fn session_open_rejects_non_utf8_session_id() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let session_id = [0xff, 0xfe];
    let mut handle: EbiSessionHandle = ptr::null_mut();

    assert_eq!(
        ebi_session_open(session_id.as_ptr(), session_id.len(), &mut handle),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert!(handle.is_null());
    std::env::remove_var("EBI_IPC_DIR");
}

fn lock_env() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn open_test_session() -> EbiSessionHandle {
    let session_id = b"session-42";
    let mut handle: EbiSessionHandle = ptr::null_mut();
    assert_eq!(
        ebi_session_open(session_id.as_ptr(), session_id.len(), &mut handle),
        EBI_OK
    );
    handle
}

fn control_queue_spec() -> MappedQueueSpec {
    MappedQueueSpec::new(
        build_session_channel_name("session-42", ChannelKind::Control),
        ChannelKind::Control,
        256,
        16 * 1024,
    )
}

fn publish_session_payload(
    directory: &std::path::Path,
    session_id: &str,
    kind: ChannelKind,
    sequence: u64,
    payload: &[u8],
) {
    let name = build_session_channel_name(session_id, kind);
    publish_payload(directory, name, kind, sequence, payload);
}

fn publish_browser_payload(
    directory: &std::path::Path,
    session_id: &str,
    browser_id: &str,
    kind: ChannelKind,
    sequence: u64,
    payload: &[u8],
) {
    let name = build_browser_channel_name(session_id, browser_id, kind);
    publish_payload(directory, name, kind, sequence, payload);
}

fn publish_payload(
    directory: &std::path::Path,
    name: String,
    kind: ChannelKind,
    sequence: u64,
    payload: &[u8],
) {
    let spec = ChannelSpec::new(name, kind, 64 * 1024);
    let mut channel =
        ChannelMappedFile::open_in_dir(directory, &spec, ChannelOpenMode::OpenExisting)
            .or_else(|_| ChannelMappedFile::open_in_dir(directory, &spec, ChannelOpenMode::Create))
            .unwrap();
    channel.publish_latest(sequence, 0, payload).unwrap();
}
