use std::ffi::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use ipc::{build_session_channel_name, ChannelKind};

pub type EbiErrorCode = i32;
pub type EbiSessionHandle = *mut EbiSession;

pub const EBI_OK: EbiErrorCode = 0;
pub const EBI_ERROR_INVALID_ARGUMENT: EbiErrorCode = 1;
pub const EBI_ERROR_NOT_IMPLEMENTED: EbiErrorCode = 2;
pub const EBI_ERROR_PANIC: EbiErrorCode = 100;

#[repr(C)]
pub struct EbiSession {
    session_id: String,
    session_channel: String,
    control_channel: String,
    status_channel: String,
}

#[no_mangle]
pub extern "C" fn ebi_error_message(code: EbiErrorCode) -> *const c_char {
    match code {
        EBI_OK => c"ok".as_ptr(),
        EBI_ERROR_INVALID_ARGUMENT => c"invalid argument".as_ptr(),
        EBI_ERROR_NOT_IMPLEMENTED => c"not implemented".as_ptr(),
        EBI_ERROR_PANIC => c"panic".as_ptr(),
        _ => ptr::null(),
    }
}

#[no_mangle]
pub extern "C" fn ebi_session_open(
    session_id_ptr: *const u8,
    session_id_len: usize,
    out_handle: *mut EbiSessionHandle,
) -> EbiErrorCode {
    ffi_boundary(|| {
        if session_id_ptr.is_null() || out_handle.is_null() {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let bytes = unsafe { std::slice::from_raw_parts(session_id_ptr, session_id_len) };
        let Ok(session_id) = std::str::from_utf8(bytes) else {
            return EBI_ERROR_INVALID_ARGUMENT;
        };

        let session = Box::new(EbiSession {
            session_id: session_id.to_owned(),
            session_channel: build_session_channel_name(session_id, ChannelKind::Session),
            control_channel: build_session_channel_name(session_id, ChannelKind::Control),
            status_channel: build_session_channel_name(session_id, ChannelKind::Status),
        });

        unsafe {
            *out_handle = Box::into_raw(session);
        }

        EBI_OK
    })
}

#[no_mangle]
pub extern "C" fn ebi_session_close(handle: EbiSessionHandle) -> EbiErrorCode {
    ffi_boundary(|| {
        if !handle.is_null() {
            unsafe {
                drop(Box::from_raw(handle));
            }
        }

        EBI_OK
    })
}

fn ffi_boundary(call: impl FnOnce() -> EbiErrorCode) -> EbiErrorCode {
    catch_unwind(AssertUnwindSafe(call)).unwrap_or(EBI_ERROR_PANIC)
}
