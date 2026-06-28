use ipc::OutputPayload;
use ipc::{
    build_browser_channel_name, build_session_channel_name, ChannelKind, ChannelMappedFile,
    ChannelOpenMode, ChannelSpec, ControlCommand, FrameChannel, FrameChannelSpec, MappedQueueSpec,
    MappedSpscQueue,
};
use ipc_native::{
    EbiBrowserFrameHandle, EbiBrowserInputHandle, EbiBrowserOutputHandle, EbiErrorCode,
    EbiSessionHandle, EBI_ERROR_BUFFER_TOO_SMALL, EBI_ERROR_INVALID_ARGUMENT,
    EBI_ERROR_NOT_IMPLEMENTED, EBI_ERROR_QUEUE_FULL, EBI_OK,
};
use std::ffi::{c_char, CStr};
use std::ptr;
use std::sync::{Mutex, MutexGuard};

static ENV_LOCK: Mutex<()> = Mutex::new(());

macro_rules! ffi_wrap {
    ($(fn $name:ident($($arg:ident: $ty:ty),*) -> $ret:ty;)*) => {
        $(
            #[allow(clippy::too_many_arguments)]
            fn $name($($arg: $ty),*) -> $ret {
                unsafe { ipc_native::$name($($arg),*) }
            }
        )*
    };
}

ffi_wrap! {
    fn ebi_error_message(code: EbiErrorCode) -> *const c_char;
    fn ebi_session_open(session_id_ptr: *const u8, session_id_len: usize, out_handle: *mut EbiSessionHandle) -> EbiErrorCode;
    fn ebi_session_close(handle: EbiSessionHandle) -> EbiErrorCode;
    fn ebi_control_send(handle: EbiSessionHandle, sequence: u64, payload_ptr: *const u8, payload_len: usize) -> EbiErrorCode;
    fn ebi_control_add_browser(handle: EbiSessionHandle, sequence: u64, browser_id_ptr: *const u8, browser_id_len: usize, width: i32, height: i32, address_ptr: *const u8, address_len: usize) -> EbiErrorCode;
    fn ebi_control_remove_browser(handle: EbiSessionHandle, sequence: u64, browser_id_ptr: *const u8, browser_id_len: usize) -> EbiErrorCode;
    fn ebi_control_resize_browser(handle: EbiSessionHandle, sequence: u64, browser_id_ptr: *const u8, browser_id_len: usize, width: i32, height: i32) -> EbiErrorCode;
    fn ebi_control_shutdown(handle: EbiSessionHandle, sequence: u64) -> EbiErrorCode;
    fn ebi_input_set_mouse_latest(handle: EbiSessionHandle, x: i32, y: i32, buttons: u32, valid: u8) -> EbiErrorCode;
    fn ebi_input_push_event(handle: EbiSessionHandle, kind: u32, sequence: u64, payload_ptr: *const u8, payload_len: usize) -> EbiErrorCode;
    fn ebi_browser_input_open(session_handle: EbiSessionHandle, browser_id_ptr: *const u8, browser_id_len: usize, out_handle: *mut EbiBrowserInputHandle) -> EbiErrorCode;
    fn ebi_browser_input_close(handle: EbiBrowserInputHandle) -> EbiErrorCode;
    fn ebi_browser_input_set_mouse_latest(handle: EbiBrowserInputHandle, x: i32, y: i32, buttons: u32, delta_x: i32, delta_y: i32, valid: u8) -> EbiErrorCode;
    fn ebi_browser_input_push_event(handle: EbiBrowserInputHandle, kind: u32, sequence: u64, payload_ptr: *const u8, payload_len: usize) -> EbiErrorCode;
    fn ebi_status_read(handle: EbiSessionHandle, buffer_ptr: *mut u8, buffer_len: usize, written_ptr: *mut usize) -> EbiErrorCode;
    fn ebi_output_try_read(handle: EbiSessionHandle, browser_id_ptr: *const u8, browser_id_len: usize, buffer_ptr: *mut u8, buffer_len: usize, written_ptr: *mut usize) -> EbiErrorCode;
    fn ebi_browser_output_open(session_handle: EbiSessionHandle, browser_id_ptr: *const u8, browser_id_len: usize, out_handle: *mut EbiBrowserOutputHandle) -> EbiErrorCode;
    fn ebi_browser_output_close(handle: EbiBrowserOutputHandle) -> EbiErrorCode;
    fn ebi_browser_output_try_read_latest(handle: EbiBrowserOutputHandle, buffer_ptr: *mut u8, buffer_len: usize, written_ptr: *mut usize) -> EbiErrorCode;
    fn ebi_browser_output_try_pop_event(handle: EbiBrowserOutputHandle, kind_ptr: *mut u32, sequence_ptr: *mut u64, buffer_ptr: *mut u8, buffer_len: usize, written_ptr: *mut usize) -> EbiErrorCode;
    fn ebi_frame_try_copy_latest(handle: EbiSessionHandle, browser_id_ptr: *const u8, browser_id_len: usize, buffer_ptr: *mut u8, buffer_len: usize, width_ptr: *mut i32, height_ptr: *mut i32, sequence_ptr: *mut u64, written_ptr: *mut usize) -> EbiErrorCode;
    fn ebi_frame_ack(handle: EbiSessionHandle, browser_id_ptr: *const u8, browser_id_len: usize, sequence: u64) -> EbiErrorCode;
    fn ebi_browser_frame_open(session_handle: EbiSessionHandle, browser_id_ptr: *const u8, browser_id_len: usize, out_handle: *mut EbiBrowserFrameHandle) -> EbiErrorCode;
    fn ebi_browser_frame_close(handle: EbiBrowserFrameHandle) -> EbiErrorCode;
    fn ebi_browser_frame_try_copy_latest(handle: EbiBrowserFrameHandle, buffer_ptr: *mut u8, buffer_len: usize, width_ptr: *mut i32, height_ptr: *mut i32, sequence_ptr: *mut u64, written_ptr: *mut usize) -> EbiErrorCode;
    fn ebi_browser_frame_ack(handle: EbiBrowserFrameHandle, sequence: u64) -> EbiErrorCode;
}

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
fn session_open_reuses_existing_session_control_and_status_channels_without_truncating_queue() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    precreate_session_channels(temp.path(), "session-42");
    let mut existing_queue = MappedSpscQueue::open_in_dir(
        temp.path(),
        &control_queue_spec(),
        ChannelOpenMode::OpenExisting,
    )
    .unwrap();
    existing_queue
        .try_push(ipc::MappedQueueItem::new(99, 123, b"preexisting"))
        .unwrap();
    drop(existing_queue);
    let session_id = b"session-42";
    let mut handle: EbiSessionHandle = ptr::null_mut();

    let result = ebi_session_open(session_id.as_ptr(), session_id.len(), &mut handle);

    assert_eq!(result, EBI_OK);
    let mut queue = MappedSpscQueue::open_in_dir(
        temp.path(),
        &control_queue_spec(),
        ChannelOpenMode::OpenExisting,
    )
    .unwrap();
    let item = queue
        .try_pop()
        .unwrap()
        .expect("existing control queue item");
    assert_eq!(item.kind, 99);
    assert_eq!(item.sequence, 123);
    assert_eq!(item.payload, b"preexisting");
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
fn typed_control_apis_encode_commands_on_rust_side_and_enqueue_them() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    let remove_browser_id = b"browser-A";
    let resize_browser_id = b"browser-B";

    assert_eq!(
        ebi_control_remove_browser(
            handle,
            13,
            remove_browser_id.as_ptr(),
            remove_browser_id.len()
        ),
        EBI_OK
    );
    assert_eq!(
        ebi_control_resize_browser(
            handle,
            14,
            resize_browser_id.as_ptr(),
            resize_browser_id.len(),
            1920,
            1080
        ),
        EBI_OK
    );
    assert_eq!(ebi_control_shutdown(handle, 15), EBI_OK);

    let mut queue = MappedSpscQueue::open_in_dir(
        temp.path(),
        &control_queue_spec(),
        ChannelOpenMode::OpenExisting,
    )
    .unwrap();
    let remove_item = queue.try_pop().unwrap().expect("remove control item");
    assert_eq!(remove_item.kind, 1);
    assert_eq!(remove_item.sequence, 13);
    assert_eq!(
        ControlCommand::decode(&remove_item.payload).unwrap(),
        ControlCommand::RemoveBrowser {
            browser_id: "browser-A".to_string(),
        }
    );

    let resize_item = queue.try_pop().unwrap().expect("resize control item");
    assert_eq!(resize_item.kind, 1);
    assert_eq!(resize_item.sequence, 14);
    assert_eq!(
        ControlCommand::decode(&resize_item.payload).unwrap(),
        ControlCommand::ResizeBrowser {
            browser_id: "browser-B".to_string(),
            width: 1920,
            height: 1080,
        }
    );

    let shutdown_item = queue.try_pop().unwrap().expect("shutdown control item");
    assert_eq!(shutdown_item.kind, 1);
    assert_eq!(shutdown_item.sequence, 15);
    assert_eq!(
        ControlCommand::decode(&shutdown_item.payload).unwrap(),
        ControlCommand::Shutdown
    );
    assert_eq!(queue.try_pop().unwrap(), None);
    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn typed_control_apis_validate_browser_id_arguments() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    let browser_id = b"browser-A";

    assert_eq!(
        ebi_control_remove_browser(ptr::null_mut(), 1, browser_id.as_ptr(), browser_id.len()),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_control_remove_browser(handle, 1, ptr::null(), browser_id.len()),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_control_remove_browser(handle, 1, browser_id.as_ptr(), 0),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_control_remove_browser(handle, 1, b"../bad".as_ptr(), b"../bad".len()),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_control_remove_browser(handle, 1, [0xff, 0xfe].as_ptr(), 2),
        EBI_ERROR_INVALID_ARGUMENT
    );

    assert_eq!(
        ebi_control_resize_browser(
            ptr::null_mut(),
            1,
            browser_id.as_ptr(),
            browser_id.len(),
            1,
            2
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_control_resize_browser(handle, 1, ptr::null(), browser_id.len(), 1, 2),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_control_resize_browser(handle, 1, browser_id.as_ptr(), 0, 1, 2),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_control_resize_browser(handle, 1, b"../bad".as_ptr(), b"../bad".len(), 1, 2),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_control_resize_browser(handle, 1, [0xff, 0xfe].as_ptr(), 2, 1, 2),
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
fn browser_input_handle_writes_latest_mouse_delta_and_queue_without_browser_id_on_hot_path() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let session = open_test_session();
    let browser_id = b"browser-A";
    let mut input: EbiBrowserInputHandle = ptr::null_mut();

    assert_eq!(
        ebi_browser_input_open(session, browser_id.as_ptr(), browser_id.len(), &mut input),
        EBI_OK
    );
    assert!(!input.is_null());
    assert_eq!(
        ebi_browser_input_set_mouse_latest(input, -10, 20, 3, -7, 9, 1),
        EBI_OK
    );
    assert_eq!(
        ebi_browser_input_push_event(input, 3, 12, b"key".as_ptr(), b"key".len()),
        EBI_OK
    );

    let mut latest = ChannelMappedFile::open_in_dir(
        temp.path(),
        &input_latest_spec("session-42", "browser-A"),
        ChannelOpenMode::OpenExisting,
    )
    .unwrap();
    let snapshot = latest.try_read_latest().unwrap().expect("mouse latest");
    assert_eq!(
        decode_mouse_latest_payload(&snapshot.payload),
        (-10, 20, 3, -7, 9, true)
    );

    let mut queue = MappedSpscQueue::open_in_dir(
        temp.path(),
        &input_queue_spec("session-42", "browser-A"),
        ChannelOpenMode::OpenExisting,
    )
    .unwrap();
    let item = queue.try_pop().unwrap().expect("input queue item");
    assert_eq!(item.kind, 3);
    assert_eq!(item.sequence, 12);
    assert_eq!(item.payload, b"key");

    assert_eq!(ebi_browser_input_close(input), EBI_OK);
    assert_eq!(ebi_session_close(session), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn browser_input_handle_reuses_existing_latest_channel() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let session = open_test_session();
    let browser_id = b"browser-A";
    let mut input: EbiBrowserInputHandle = ptr::null_mut();

    assert_eq!(
        ebi_browser_input_open(session, browser_id.as_ptr(), browser_id.len(), &mut input),
        EBI_OK
    );
    assert_eq!(
        ebi_browser_input_set_mouse_latest(input, 10, 20, 1, 2, 3, 1),
        EBI_OK
    );
    assert_eq!(
        ebi_browser_input_set_mouse_latest(input, 30, 40, 2, 4, 5, 1),
        EBI_OK
    );

    let mut channel = ChannelMappedFile::open_in_dir(
        temp.path(),
        &input_latest_spec("session-42", "browser-A"),
        ChannelOpenMode::OpenExisting,
    )
    .unwrap();
    let snapshot = channel.try_read_latest().unwrap().expect("mouse latest");
    assert_eq!(snapshot.header.header_commit, 4);
    assert_eq!(
        decode_mouse_latest_payload(&snapshot.payload),
        (30, 40, 2, 4, 5, true)
    );
    assert_eq!(ebi_browser_input_close(input), EBI_OK);
    assert_eq!(ebi_session_close(session), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn browser_input_open_validates_browser_id_and_pointers() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let session = open_test_session();
    let browser_id = b"browser-A";
    let mut input: EbiBrowserInputHandle = ptr::null_mut();

    assert_eq!(
        ebi_browser_input_open(
            ptr::null_mut(),
            browser_id.as_ptr(),
            browser_id.len(),
            &mut input
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_browser_input_open(session, ptr::null(), browser_id.len(), &mut input),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_browser_input_open(session, browser_id.as_ptr(), 0, &mut input),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_browser_input_open(session, b"../bad".as_ptr(), b"../bad".len(), &mut input),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_browser_input_open(session, [0xff, 0xfe].as_ptr(), 2, &mut input),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_browser_input_open(
            session,
            browser_id.as_ptr(),
            browser_id.len(),
            ptr::null_mut()
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );

    assert_eq!(ebi_session_close(session), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn browser_input_push_event_reports_payload_too_large() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let session = open_test_session();
    let browser_id = b"browser-A";
    let mut input: EbiBrowserInputHandle = ptr::null_mut();
    let payload = vec![1u8; 16 * 1024 + 1];
    assert_eq!(
        ebi_browser_input_open(session, browser_id.as_ptr(), browser_id.len(), &mut input),
        EBI_OK
    );

    assert_eq!(
        ebi_browser_input_push_event(input, 3, 12, payload.as_ptr(), payload.len()),
        EBI_ERROR_BUFFER_TOO_SMALL
    );

    assert_eq!(ebi_browser_input_close(input), EBI_OK);
    assert_eq!(ebi_session_close(session), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn browser_input_push_event_reports_queue_full_without_overwriting_events() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let session = open_test_session();
    let browser_id = b"browser-A";
    let mut input: EbiBrowserInputHandle = ptr::null_mut();
    let payload = b"key";
    assert_eq!(
        ebi_browser_input_open(session, browser_id.as_ptr(), browser_id.len(), &mut input),
        EBI_OK
    );

    for sequence in 0..1024 {
        assert_eq!(
            ebi_browser_input_push_event(input, 3, sequence, payload.as_ptr(), payload.len()),
            EBI_OK
        );
    }

    assert_eq!(
        ebi_browser_input_push_event(input, 3, 1024, payload.as_ptr(), payload.len()),
        EBI_ERROR_QUEUE_FULL
    );

    let mut queue = MappedSpscQueue::open_in_dir(
        temp.path(),
        &input_queue_spec("session-42", "browser-A"),
        ChannelOpenMode::OpenExisting,
    )
    .unwrap();
    assert_eq!(queue.metadata().dropped_count, 1);
    assert_eq!(queue.try_pop().unwrap().unwrap().sequence, 0);
    assert_eq!(ebi_browser_input_close(input), EBI_OK);
    assert_eq!(ebi_session_close(session), EBI_OK);
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
        &OutputPayload::PageEvent(ipc::PageEventOutput {
            event_type: 1,
            url: "https://example.test".to_string(),
            detail: "ready".to_string(),
        })
        .encode(),
    );
    let browser_id = b"browser-A";
    let mut buffer = [0u8; 128];
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

    assert_eq!(
        OutputPayload::decode(&buffer[..written]).unwrap(),
        OutputPayload::PageEvent(ipc::PageEventOutput {
            event_type: 1,
            url: "https://example.test".to_string(),
            detail: "ready".to_string(),
        })
    );
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
fn browser_output_handle_reads_latest_and_queue_without_browser_id_on_hot_path() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let session = open_test_session();
    let browser_id = b"browser-A";
    let mut output: EbiBrowserOutputHandle = ptr::null_mut();

    assert_eq!(
        ebi_browser_output_open(session, browser_id.as_ptr(), browser_id.len(), &mut output),
        EBI_OK
    );
    assert!(!output.is_null());

    publish_browser_payload(
        temp.path(),
        "session-42",
        "browser-A",
        ChannelKind::Output,
        7,
        b"latest",
    );
    let mut output_queue = MappedSpscQueue::open_in_dir(
        temp.path(),
        &output_queue_spec("session-42", "browser-A"),
        ChannelOpenMode::OpenExisting,
    )
    .unwrap();
    output_queue
        .try_push(ipc::MappedQueueItem::new(4, 8, b"queued"))
        .unwrap();
    drop(output_queue);

    let mut latest = [0u8; 16];
    let mut latest_written = 0;
    assert_eq!(
        ebi_browser_output_try_read_latest(
            output,
            latest.as_mut_ptr(),
            latest.len(),
            &mut latest_written
        ),
        EBI_OK
    );
    assert_eq!(&latest[..latest_written], b"latest");

    let mut kind = 0;
    let mut sequence = 0;
    let mut queued = [0u8; 16];
    let mut queued_written = 0;
    assert_eq!(
        ebi_browser_output_try_pop_event(
            output,
            &mut kind,
            &mut sequence,
            queued.as_mut_ptr(),
            queued.len(),
            &mut queued_written
        ),
        EBI_OK
    );
    assert_eq!(kind, 4);
    assert_eq!(sequence, 8);
    assert_eq!(&queued[..queued_written], b"queued");

    assert_eq!(ebi_browser_output_close(output), EBI_OK);
    assert_eq!(ebi_session_close(session), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn frame_try_copy_latest_returns_zero_bytes_when_frame_channel_is_absent() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    let browser_id = b"browser-A";
    let mut buffer = [0u8; 16];
    let mut width = i32::MIN;
    let mut height = i32::MIN;
    let mut sequence = u64::MAX;
    let mut written = usize::MAX;

    assert_eq!(
        ebi_frame_try_copy_latest(
            handle,
            browser_id.as_ptr(),
            browser_id.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut width,
            &mut height,
            &mut sequence,
            &mut written
        ),
        EBI_OK
    );

    assert_eq!(width, 0);
    assert_eq!(height, 0);
    assert_eq!(sequence, 0);
    assert_eq!(written, 0);
    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn frame_try_copy_latest_reports_required_length_when_buffer_is_too_small() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    publish_frame(temp.path(), "session-42", "browser-A", 9, 2, 2, &[1u8; 16]);
    let browser_id = b"browser-A";
    let mut buffer = [0u8; 15];
    let mut width = 0;
    let mut height = 0;
    let mut sequence = 0;
    let mut written = 0;

    assert_eq!(
        ebi_frame_try_copy_latest(
            handle,
            browser_id.as_ptr(),
            browser_id.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut width,
            &mut height,
            &mut sequence,
            &mut written
        ),
        EBI_ERROR_BUFFER_TOO_SMALL
    );

    assert_eq!(written, 16);
    assert_eq!(width, 2);
    assert_eq!(height, 2);
    assert_eq!(sequence, 9);
    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn frame_try_copy_latest_copies_latest_frame_metadata_and_pixels() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    publish_frame(
        temp.path(),
        "session-42",
        "browser-A",
        9,
        2,
        2,
        &[1, 2, 3, 4],
    );
    let browser_id = b"browser-A";
    let mut buffer = [0u8; 16];
    let mut width = 0;
    let mut height = 0;
    let mut sequence = 0;
    let mut written = 0;

    assert_eq!(
        ebi_frame_try_copy_latest(
            handle,
            browser_id.as_ptr(),
            browser_id.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut width,
            &mut height,
            &mut sequence,
            &mut written
        ),
        EBI_OK
    );

    assert_eq!(width, 2);
    assert_eq!(height, 2);
    assert_eq!(sequence, 9);
    assert_eq!(written, 4);
    assert_eq!(&buffer[..4], &[1, 2, 3, 4]);
    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn browser_frame_handle_copies_and_acks_without_browser_id_on_hot_path() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let session = open_test_session();
    let browser_id = b"browser-A";
    let mut frame: EbiBrowserFrameHandle = ptr::null_mut();

    assert_eq!(
        ebi_browser_frame_open(session, browser_id.as_ptr(), browser_id.len(), &mut frame),
        EBI_OK
    );
    assert!(!frame.is_null());
    publish_frame(
        temp.path(),
        "session-42",
        "browser-A",
        9,
        2,
        2,
        &[1, 2, 3, 4],
    );

    let mut buffer = [0u8; 16];
    let mut width = 0;
    let mut height = 0;
    let mut sequence = 0;
    let mut written = 0;
    assert_eq!(
        ebi_browser_frame_try_copy_latest(
            frame,
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut width,
            &mut height,
            &mut sequence,
            &mut written
        ),
        EBI_OK
    );
    assert_eq!(width, 2);
    assert_eq!(height, 2);
    assert_eq!(sequence, 9);
    assert_eq!(written, 4);
    assert_eq!(&buffer[..written], &[1, 2, 3, 4]);

    assert_eq!(ebi_browser_frame_ack(frame, 9), EBI_OK);
    let channel = FrameChannel::open_in_dir(
        temp.path(),
        &frame_spec("session-42", "browser-A", 2, 64 * 1024 * 1024),
        ChannelOpenMode::OpenExisting,
    )
    .unwrap();
    assert_eq!(channel.frame_header().acknowledged_frame, 9);

    assert_eq!(ebi_browser_frame_close(frame), EBI_OK);
    assert_eq!(ebi_session_close(session), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn frame_ack_updates_shared_consumer_ack() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    publish_frame(
        temp.path(),
        "session-42",
        "browser-A",
        9,
        2,
        2,
        &[1, 2, 3, 4],
    );
    let browser_id = b"browser-A";

    assert_eq!(
        ebi_frame_ack(handle, browser_id.as_ptr(), browser_id.len(), 9),
        EBI_OK
    );

    let channel = FrameChannel::open_in_dir(
        temp.path(),
        &frame_spec("session-42", "browser-A", 2, 64 * 1024 * 1024),
        ChannelOpenMode::OpenExisting,
    )
    .unwrap();
    assert_eq!(channel.header().consumer_ack, 9);
    assert_eq!(channel.frame_header().acknowledged_frame, 9);
    assert_eq!(ebi_session_close(handle), EBI_OK);
    std::env::remove_var("EBI_IPC_DIR");
}

#[test]
fn frame_ffi_validates_pointers_empty_buffer_and_browser_id() {
    let _guard = lock_env();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("EBI_IPC_DIR", temp.path());
    let handle = open_test_session();
    let browser_id = b"browser-A";
    let mut buffer = [0u8; 4];
    let mut width = 0;
    let mut height = 0;
    let mut sequence = 0;
    let mut written = 0;

    assert_eq!(
        ebi_frame_try_copy_latest(
            ptr::null_mut(),
            browser_id.as_ptr(),
            browser_id.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut width,
            &mut height,
            &mut sequence,
            &mut written
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_frame_try_copy_latest(
            handle,
            ptr::null(),
            browser_id.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut width,
            &mut height,
            &mut sequence,
            &mut written
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_frame_try_copy_latest(
            handle,
            browser_id.as_ptr(),
            0,
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut width,
            &mut height,
            &mut sequence,
            &mut written
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_frame_try_copy_latest(
            handle,
            browser_id.as_ptr(),
            browser_id.len(),
            ptr::null_mut(),
            buffer.len(),
            &mut width,
            &mut height,
            &mut sequence,
            &mut written
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_frame_try_copy_latest(
            handle,
            browser_id.as_ptr(),
            browser_id.len(),
            buffer.as_mut_ptr(),
            0,
            &mut width,
            &mut height,
            &mut sequence,
            &mut written
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_frame_try_copy_latest(
            handle,
            browser_id.as_ptr(),
            browser_id.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            ptr::null_mut(),
            &mut height,
            &mut sequence,
            &mut written
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_frame_try_copy_latest(
            handle,
            browser_id.as_ptr(),
            browser_id.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut width,
            ptr::null_mut(),
            &mut sequence,
            &mut written
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_frame_try_copy_latest(
            handle,
            browser_id.as_ptr(),
            browser_id.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut width,
            &mut height,
            ptr::null_mut(),
            &mut written
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_frame_try_copy_latest(
            handle,
            browser_id.as_ptr(),
            browser_id.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut width,
            &mut height,
            &mut sequence,
            ptr::null_mut()
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_frame_try_copy_latest(
            handle,
            b"../bad".as_ptr(),
            b"../bad".len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut width,
            &mut height,
            &mut sequence,
            &mut written
        ),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_frame_ack(ptr::null_mut(), browser_id.as_ptr(), browser_id.len(), 9),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_frame_ack(handle, ptr::null(), browser_id.len(), 9),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_frame_ack(handle, browser_id.as_ptr(), 0, 9),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_frame_ack(handle, b"../bad".as_ptr(), b"../bad".len(), 9),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert_eq!(
        ebi_frame_ack(handle, [0xff, 0xfe].as_ptr(), 2, 9),
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

fn input_latest_spec(session_id: &str, browser_id: &str) -> ChannelSpec {
    ChannelSpec::new(
        build_browser_channel_name(session_id, browser_id, ChannelKind::Input),
        ChannelKind::Input,
        64,
    )
}

fn input_queue_spec(session_id: &str, browser_id: &str) -> MappedQueueSpec {
    let name = format!(
        "{}_queue",
        build_browser_channel_name(session_id, browser_id, ChannelKind::Input)
    );
    MappedQueueSpec::new(name, ChannelKind::Input, 1024, 16 * 1024)
}

fn output_queue_spec(session_id: &str, browser_id: &str) -> MappedQueueSpec {
    let name = format!(
        "{}_queue",
        build_browser_channel_name(session_id, browser_id, ChannelKind::Output)
    );
    MappedQueueSpec::new(name, ChannelKind::Output, 1024, 16 * 1024)
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

fn decode_mouse_latest_payload(payload: &[u8]) -> (i32, i32, u32, i32, i32, bool) {
    assert_eq!(payload.len(), 24);
    assert_eq!(&payload[21..24], &[0, 0, 0]);
    (
        i32::from_le_bytes(payload[0..4].try_into().unwrap()),
        i32::from_le_bytes(payload[4..8].try_into().unwrap()),
        u32::from_le_bytes(payload[8..12].try_into().unwrap()),
        i32::from_le_bytes(payload[12..16].try_into().unwrap()),
        i32::from_le_bytes(payload[16..20].try_into().unwrap()),
        payload[20] != 0,
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

fn precreate_session_channels(directory: &std::path::Path, session_id: &str) {
    let session_spec = ChannelSpec::new(
        build_session_channel_name(session_id, ChannelKind::Session),
        ChannelKind::Session,
        4096,
    );
    ChannelMappedFile::open_in_dir(directory, &session_spec, ChannelOpenMode::Create).unwrap();
    let status_spec = ChannelSpec::new(
        build_session_channel_name(session_id, ChannelKind::Status),
        ChannelKind::Status,
        64 * 1024,
    );
    ChannelMappedFile::open_in_dir(directory, &status_spec, ChannelOpenMode::Create).unwrap();
    MappedSpscQueue::open_in_dir(directory, &control_queue_spec(), ChannelOpenMode::Create)
        .unwrap();
}

fn publish_frame(
    directory: &std::path::Path,
    session_id: &str,
    browser_id: &str,
    sequence: u64,
    width: u32,
    height: u32,
    pixels: &[u8],
) {
    let mut channel = FrameChannel::open_in_dir(
        directory,
        &frame_spec(session_id, browser_id, 2, 64 * 1024 * 1024),
        ChannelOpenMode::OpenExisting,
    )
    .or_else(|_| {
        FrameChannel::open_in_dir(
            directory,
            &frame_spec(session_id, browser_id, 2, 64 * 1024 * 1024),
            ChannelOpenMode::Create,
        )
    })
    .unwrap();
    channel
        .publish_full(sequence, width, height, pixels)
        .unwrap();
}
