use ipc_native::{
    ebi_error_message, ebi_session_close, ebi_session_open, EbiSessionHandle,
    EBI_ERROR_INVALID_ARGUMENT, EBI_ERROR_NOT_IMPLEMENTED, EBI_OK,
};
use std::ffi::CStr;
use std::ptr;

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
}

#[test]
fn session_close_accepts_null_as_noop() {
    assert_eq!(ebi_session_close(ptr::null_mut()), EBI_OK);
}

#[test]
fn session_open_rejects_non_utf8_session_id() {
    let session_id = [0xff, 0xfe];
    let mut handle: EbiSessionHandle = ptr::null_mut();

    assert_eq!(
        ebi_session_open(session_id.as_ptr(), session_id.len(), &mut handle),
        EBI_ERROR_INVALID_ARGUMENT
    );
    assert!(handle.is_null());
}
