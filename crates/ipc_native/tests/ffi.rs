use ipc_native::{
    ebi_control_send, ebi_error_message, ebi_input_push_event, ebi_input_set_mouse_latest,
    ebi_session_close, ebi_session_open, EbiSessionHandle, EBI_ERROR_INVALID_ARGUMENT,
    EBI_ERROR_NOT_IMPLEMENTED, EBI_OK,
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
fn control_send_publishes_payload_to_control_channel() {
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

    let control_path = temp.path().join("EmbeddedBrowser_session-42_control");
    let bytes = std::fs::read(control_path).unwrap();
    assert_eq!(&bytes[68..68 + payload.len()], payload);
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
    ENV_LOCK.lock().expect("env lock")
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
