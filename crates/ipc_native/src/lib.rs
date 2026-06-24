use std::ffi::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::ptr;

use ipc::{
    build_session_channel_name, ChannelKind, ChannelMappedFile, ChannelOpenMode, ChannelSpec,
};

pub type EbiErrorCode = i32;
pub type EbiSessionHandle = *mut EbiSession;

pub const EBI_OK: EbiErrorCode = 0;
pub const EBI_ERROR_INVALID_ARGUMENT: EbiErrorCode = 1;
pub const EBI_ERROR_NOT_IMPLEMENTED: EbiErrorCode = 2;
pub const EBI_ERROR_IO: EbiErrorCode = 3;
pub const EBI_ERROR_PANIC: EbiErrorCode = 100;

#[repr(C)]
pub struct EbiSession {
    session_id: String,
    session_channel: ChannelMappedFile,
    control_channel: ChannelMappedFile,
    status_channel: ChannelMappedFile,
}

#[no_mangle]
pub extern "C" fn ebi_error_message(code: EbiErrorCode) -> *const c_char {
    match code {
        EBI_OK => c"ok".as_ptr(),
        EBI_ERROR_INVALID_ARGUMENT => c"invalid argument".as_ptr(),
        EBI_ERROR_NOT_IMPLEMENTED => c"not implemented".as_ptr(),
        EBI_ERROR_IO => c"io error".as_ptr(),
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

        let Ok(session_channel) = open_session_channel(session_id, ChannelKind::Session, 4096)
        else {
            return EBI_ERROR_IO;
        };
        let Ok(control_channel) = open_session_channel(session_id, ChannelKind::Control, 64 * 1024)
        else {
            return EBI_ERROR_IO;
        };
        let Ok(status_channel) = open_session_channel(session_id, ChannelKind::Status, 64 * 1024)
        else {
            return EBI_ERROR_IO;
        };

        let session = Box::new(EbiSession {
            session_id: session_id.to_owned(),
            session_channel,
            control_channel,
            status_channel,
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

#[no_mangle]
pub extern "C" fn ebi_control_send(
    handle: EbiSessionHandle,
    sequence: u64,
    payload_ptr: *const u8,
    payload_len: usize,
) -> EbiErrorCode {
    ffi_boundary(|| {
        if handle.is_null() || payload_ptr.is_null() {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let payload = unsafe { std::slice::from_raw_parts(payload_ptr, payload_len) };
        let session = unsafe { &mut *handle };
        match session.control_channel.publish_latest(sequence, 0, payload) {
            Ok(()) => EBI_OK,
            Err(_) => EBI_ERROR_IO,
        }
    })
}

fn ffi_boundary(call: impl FnOnce() -> EbiErrorCode) -> EbiErrorCode {
    catch_unwind(AssertUnwindSafe(call)).unwrap_or(EBI_ERROR_PANIC)
}

fn open_session_channel(
    session_id: &str,
    channel_kind: ChannelKind,
    capacity_bytes: u32,
) -> Result<ChannelMappedFile, ipc::IpcError> {
    let name = build_session_channel_name(session_id, channel_kind);
    let spec = ChannelSpec::new(name, channel_kind, capacity_bytes);
    ChannelMappedFile::open_in_dir(ipc_directory(), &spec, ChannelOpenMode::Create)
}

fn ipc_directory() -> PathBuf {
    std::env::var_os("EBI_IPC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("EmbeddedBrowserIpc"))
}
