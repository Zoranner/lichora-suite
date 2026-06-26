use std::ffi::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::ptr;

use ipc::{
    build_browser_channel_name, build_session_channel_name, ChannelKind, ChannelMappedFile,
    ChannelOpenMode, ChannelSpec, ControlCommand, MappedQueueItem, MappedQueueSpec,
    MappedSpscQueue,
};
use ipc::{FrameChannel, FrameChannelSpec, FrameCopyError};
use ipc::{InputChannelState, InputEvent, InputEventKind, MouseLatest};

pub type EbiErrorCode = i32;
pub type EbiSessionHandle = *mut EbiSession;

pub const EBI_OK: EbiErrorCode = 0;
pub const EBI_ERROR_INVALID_ARGUMENT: EbiErrorCode = 1;
pub const EBI_ERROR_NOT_IMPLEMENTED: EbiErrorCode = 2;
pub const EBI_ERROR_IO: EbiErrorCode = 3;
pub const EBI_ERROR_BUFFER_TOO_SMALL: EbiErrorCode = 4;
pub const EBI_ERROR_QUEUE_FULL: EbiErrorCode = 5;
pub const EBI_ERROR_PANIC: EbiErrorCode = 100;

const INPUT_LATEST_CAPACITY_BYTES: u32 = 64;
const INPUT_QUEUE_ITEM_CAPACITY: u32 = 1024;
const INPUT_QUEUE_MAX_PAYLOAD_LEN: u32 = 16 * 1024;
const MOUSE_LATEST_PAYLOAD_LEN: usize = 16;
const FRAME_SLOT_COUNT: u32 = 2;
const FRAME_SLOT_SIZE: u32 = 64 * 1024 * 1024;

#[repr(C)]
pub struct EbiSession {
    session_id: String,
    session_channel: ChannelMappedFile,
    control_queue: MappedSpscQueue,
    status_channel: ChannelMappedFile,
    input_state: InputChannelState,
}

#[no_mangle]
pub extern "C" fn ebi_error_message(code: EbiErrorCode) -> *const c_char {
    match code {
        EBI_OK => c"ok".as_ptr(),
        EBI_ERROR_INVALID_ARGUMENT => c"invalid argument".as_ptr(),
        EBI_ERROR_NOT_IMPLEMENTED => c"not implemented".as_ptr(),
        EBI_ERROR_IO => c"io error".as_ptr(),
        EBI_ERROR_BUFFER_TOO_SMALL => c"buffer too small".as_ptr(),
        EBI_ERROR_QUEUE_FULL => c"queue full".as_ptr(),
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
        let Ok(control_queue) = open_control_queue(session_id) else {
            return EBI_ERROR_IO;
        };
        let Ok(status_channel) = open_session_channel(session_id, ChannelKind::Status, 64 * 1024)
        else {
            return EBI_ERROR_IO;
        };

        let session = Box::new(EbiSession {
            session_id: session_id.to_owned(),
            session_channel,
            control_queue,
            status_channel,
            input_state: InputChannelState::new(1024),
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
        push_control_payload(session, 0, sequence, payload)
    })
}

#[no_mangle]
pub extern "C" fn ebi_control_add_browser(
    handle: EbiSessionHandle,
    sequence: u64,
    browser_id_ptr: *const u8,
    browser_id_len: usize,
    width: i32,
    height: i32,
    address_ptr: *const u8,
    address_len: usize,
) -> EbiErrorCode {
    ffi_boundary(|| {
        if handle.is_null()
            || browser_id_ptr.is_null()
            || browser_id_len == 0
            || address_ptr.is_null()
        {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let Some(browser_id) = read_ffi_string(browser_id_ptr, browser_id_len) else {
            return EBI_ERROR_INVALID_ARGUMENT;
        };
        if !is_valid_browser_id(&browser_id) {
            return EBI_ERROR_INVALID_ARGUMENT;
        }
        let Some(address) = read_ffi_string(address_ptr, address_len) else {
            return EBI_ERROR_INVALID_ARGUMENT;
        };

        let session = unsafe { &mut *handle };
        let payload = ControlCommand::AddBrowser {
            browser_id,
            width,
            height,
            address,
        }
        .encode();
        push_control_payload(session, 1, sequence, &payload)
    })
}

#[no_mangle]
pub extern "C" fn ebi_control_remove_browser(
    handle: EbiSessionHandle,
    sequence: u64,
    browser_id_ptr: *const u8,
    browser_id_len: usize,
) -> EbiErrorCode {
    ffi_boundary(|| {
        if handle.is_null() || browser_id_ptr.is_null() || browser_id_len == 0 {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let Some(browser_id) = read_ffi_string(browser_id_ptr, browser_id_len) else {
            return EBI_ERROR_INVALID_ARGUMENT;
        };
        if !is_valid_browser_id(&browser_id) {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let session = unsafe { &mut *handle };
        let payload = ControlCommand::RemoveBrowser { browser_id }.encode();
        push_control_payload(session, 1, sequence, &payload)
    })
}

#[no_mangle]
pub extern "C" fn ebi_control_resize_browser(
    handle: EbiSessionHandle,
    sequence: u64,
    browser_id_ptr: *const u8,
    browser_id_len: usize,
    width: i32,
    height: i32,
) -> EbiErrorCode {
    ffi_boundary(|| {
        if handle.is_null() || browser_id_ptr.is_null() || browser_id_len == 0 {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let Some(browser_id) = read_ffi_string(browser_id_ptr, browser_id_len) else {
            return EBI_ERROR_INVALID_ARGUMENT;
        };
        if !is_valid_browser_id(&browser_id) {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let session = unsafe { &mut *handle };
        let payload = ControlCommand::ResizeBrowser {
            browser_id,
            width,
            height,
        }
        .encode();
        push_control_payload(session, 1, sequence, &payload)
    })
}

#[no_mangle]
pub extern "C" fn ebi_control_shutdown(handle: EbiSessionHandle, sequence: u64) -> EbiErrorCode {
    ffi_boundary(|| {
        if handle.is_null() {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let session = unsafe { &mut *handle };
        let payload = ControlCommand::Shutdown.encode();
        push_control_payload(session, 1, sequence, &payload)
    })
}

#[no_mangle]
pub extern "C" fn ebi_input_set_mouse_latest(
    handle: EbiSessionHandle,
    x: i32,
    y: i32,
    buttons: u32,
    valid: u8,
) -> EbiErrorCode {
    ffi_boundary(|| {
        if handle.is_null() {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let session = unsafe { &mut *handle };
        session.input_state.set_mouse_latest(MouseLatest {
            x,
            y,
            buttons,
            valid: valid != 0,
        });
        EBI_OK
    })
}

#[no_mangle]
pub extern "C" fn ebi_input_push_event(
    handle: EbiSessionHandle,
    kind: u32,
    sequence: u64,
    payload_ptr: *const u8,
    payload_len: usize,
) -> EbiErrorCode {
    ffi_boundary(|| {
        if handle.is_null() || payload_ptr.is_null() {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let Some(kind) = input_event_kind_from_u32(kind) else {
            return EBI_ERROR_INVALID_ARGUMENT;
        };
        let payload = unsafe { std::slice::from_raw_parts(payload_ptr, payload_len) };
        let session = unsafe { &mut *handle };
        match session
            .input_state
            .push_event(InputEvent::new(kind, sequence, payload))
        {
            Ok(()) => EBI_OK,
            Err(_) => EBI_ERROR_IO,
        }
    })
}

#[no_mangle]
pub extern "C" fn ebi_input_set_mouse_latest_for_browser(
    handle: EbiSessionHandle,
    browser_id_ptr: *const u8,
    browser_id_len: usize,
    x: i32,
    y: i32,
    buttons: u32,
    valid: u8,
) -> EbiErrorCode {
    ffi_boundary(|| {
        if handle.is_null() || browser_id_ptr.is_null() || browser_id_len == 0 {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let Some(browser_id) = read_ffi_string(browser_id_ptr, browser_id_len) else {
            return EBI_ERROR_INVALID_ARGUMENT;
        };
        if !is_valid_browser_id(&browser_id) {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let session = unsafe { &mut *handle };
        let mut channel = match open_input_latest_channel(&session.session_id, &browser_id) {
            Ok(channel) => channel,
            Err(_) => return EBI_ERROR_IO,
        };
        let payload = encode_mouse_latest_payload(x, y, buttons, valid != 0);
        match channel.publish_latest(0, 0, &payload) {
            Ok(()) => EBI_OK,
            Err(ipc::IpcError::PayloadExceedsCapacity { .. }) => EBI_ERROR_BUFFER_TOO_SMALL,
            Err(_) => EBI_ERROR_IO,
        }
    })
}

#[no_mangle]
pub extern "C" fn ebi_input_push_event_for_browser(
    handle: EbiSessionHandle,
    browser_id_ptr: *const u8,
    browser_id_len: usize,
    kind: u32,
    sequence: u64,
    payload_ptr: *const u8,
    payload_len: usize,
) -> EbiErrorCode {
    ffi_boundary(|| {
        if handle.is_null()
            || browser_id_ptr.is_null()
            || browser_id_len == 0
            || payload_ptr.is_null()
        {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let Some(browser_id) = read_ffi_string(browser_id_ptr, browser_id_len) else {
            return EBI_ERROR_INVALID_ARGUMENT;
        };
        if !is_valid_browser_id(&browser_id) {
            return EBI_ERROR_INVALID_ARGUMENT;
        }
        if input_event_kind_from_u32(kind).is_none() {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let payload = unsafe { std::slice::from_raw_parts(payload_ptr, payload_len) };
        let session = unsafe { &mut *handle };
        match open_input_queue(&session.session_id, &browser_id)
            .and_then(|mut queue| queue.try_push(MappedQueueItem::new(kind, sequence, payload)))
        {
            Ok(()) => EBI_OK,
            Err(ipc::MappedQueueError::QueueFull { .. }) => EBI_ERROR_QUEUE_FULL,
            Err(ipc::MappedQueueError::PayloadTooLarge { .. }) => EBI_ERROR_BUFFER_TOO_SMALL,
            Err(_) => EBI_ERROR_IO,
        }
    })
}

#[no_mangle]
pub extern "C" fn ebi_status_read(
    handle: EbiSessionHandle,
    buffer_ptr: *mut u8,
    buffer_len: usize,
    written_ptr: *mut usize,
) -> EbiErrorCode {
    ffi_boundary(|| {
        if handle.is_null() || buffer_ptr.is_null() || buffer_len == 0 || written_ptr.is_null() {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let session = unsafe { &mut *handle };
        match session.status_channel.try_read_latest() {
            Ok(Some(snapshot)) => {
                copy_payload_to_c_buffer(&snapshot.payload, buffer_ptr, buffer_len, written_ptr)
            }
            Ok(None) => {
                unsafe {
                    *written_ptr = 0;
                }
                EBI_OK
            }
            Err(_) => EBI_ERROR_IO,
        }
    })
}

#[no_mangle]
pub extern "C" fn ebi_output_try_read(
    handle: EbiSessionHandle,
    browser_id_ptr: *const u8,
    browser_id_len: usize,
    buffer_ptr: *mut u8,
    buffer_len: usize,
    written_ptr: *mut usize,
) -> EbiErrorCode {
    ffi_boundary(|| {
        if handle.is_null()
            || browser_id_ptr.is_null()
            || browser_id_len == 0
            || buffer_ptr.is_null()
            || buffer_len == 0
            || written_ptr.is_null()
        {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let browser_id_bytes =
            unsafe { std::slice::from_raw_parts(browser_id_ptr, browser_id_len) };
        let Ok(browser_id) = std::str::from_utf8(browser_id_bytes) else {
            return EBI_ERROR_INVALID_ARGUMENT;
        };
        if !is_valid_browser_id(browser_id) {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let session = unsafe { &mut *handle };
        match read_output_latest(&session.session_id, browser_id) {
            Ok(Some(snapshot)) => {
                copy_payload_to_c_buffer(&snapshot.payload, buffer_ptr, buffer_len, written_ptr)
            }
            Ok(None) => {
                unsafe {
                    *written_ptr = 0;
                }
                EBI_OK
            }
            Err(_) => EBI_ERROR_IO,
        }
    })
}

#[no_mangle]
pub extern "C" fn ebi_frame_try_copy_latest(
    handle: EbiSessionHandle,
    browser_id_ptr: *const u8,
    browser_id_len: usize,
    buffer_ptr: *mut u8,
    buffer_len: usize,
    width_ptr: *mut i32,
    height_ptr: *mut i32,
    sequence_ptr: *mut u64,
    written_ptr: *mut usize,
) -> EbiErrorCode {
    ffi_boundary(|| {
        if handle.is_null()
            || browser_id_ptr.is_null()
            || browser_id_len == 0
            || buffer_ptr.is_null()
            || buffer_len == 0
            || width_ptr.is_null()
            || height_ptr.is_null()
            || sequence_ptr.is_null()
            || written_ptr.is_null()
        {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let Some(browser_id) = read_ffi_string(browser_id_ptr, browser_id_len) else {
            return EBI_ERROR_INVALID_ARGUMENT;
        };
        if !is_valid_browser_id(&browser_id) {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let session = unsafe { &mut *handle };
        match open_frame_channel_if_exists(&session.session_id, &browser_id) {
            Ok(Some(mut channel)) => {
                let buffer = unsafe { std::slice::from_raw_parts_mut(buffer_ptr, buffer_len) };
                match channel.try_copy_latest(buffer) {
                    Ok(result) => {
                        unsafe {
                            *width_ptr = result.width;
                            *height_ptr = result.height;
                            *sequence_ptr = result.sequence;
                            *written_ptr = result.written;
                        }
                        EBI_OK
                    }
                    Err(FrameCopyError::BufferTooSmall { required }) => {
                        let frame_header = channel.frame_header();
                        unsafe {
                            *width_ptr = frame_header.width;
                            *height_ptr = frame_header.height;
                            *sequence_ptr = frame_header.frame_sequence;
                            *written_ptr = required;
                        }
                        EBI_ERROR_BUFFER_TOO_SMALL
                    }
                    Err(_) => EBI_ERROR_IO,
                }
            }
            Ok(None) => {
                unsafe {
                    *width_ptr = 0;
                    *height_ptr = 0;
                    *sequence_ptr = 0;
                    *written_ptr = 0;
                }
                EBI_OK
            }
            Err(_) => EBI_ERROR_IO,
        }
    })
}

#[no_mangle]
pub extern "C" fn ebi_frame_ack(
    handle: EbiSessionHandle,
    browser_id_ptr: *const u8,
    browser_id_len: usize,
    sequence: u64,
) -> EbiErrorCode {
    ffi_boundary(|| {
        if handle.is_null() || browser_id_ptr.is_null() || browser_id_len == 0 {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let Some(browser_id) = read_ffi_string(browser_id_ptr, browser_id_len) else {
            return EBI_ERROR_INVALID_ARGUMENT;
        };
        if !is_valid_browser_id(&browser_id) {
            return EBI_ERROR_INVALID_ARGUMENT;
        }

        let session = unsafe { &mut *handle };
        match open_frame_channel_if_exists(&session.session_id, &browser_id) {
            Ok(Some(mut channel)) => match channel.ack(sequence) {
                Ok(()) => EBI_OK,
                Err(_) => EBI_ERROR_IO,
            },
            Ok(None) => EBI_OK,
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
    open_or_create_session_channel(&spec)
}

fn open_control_queue(session_id: &str) -> Result<MappedSpscQueue, ipc::MappedQueueError> {
    let directory = ipc_directory();
    let spec = control_queue_spec(session_id);
    MappedSpscQueue::open_in_dir(&directory, &spec, ChannelOpenMode::OpenExisting)
        .or_else(|_| MappedSpscQueue::open_in_dir(directory, &spec, ChannelOpenMode::Create))
}

fn control_queue_spec(session_id: &str) -> MappedQueueSpec {
    MappedQueueSpec::new(
        build_session_channel_name(session_id, ChannelKind::Control),
        ChannelKind::Control,
        256,
        16 * 1024,
    )
}

fn open_input_latest_channel(
    session_id: &str,
    browser_id: &str,
) -> Result<ChannelMappedFile, ipc::IpcError> {
    let directory = ipc_directory();
    let spec = input_latest_spec(session_id, browser_id);
    ChannelMappedFile::open_in_dir(&directory, &spec, ChannelOpenMode::OpenExisting)
        .or_else(|_| ChannelMappedFile::open_in_dir(directory, &spec, ChannelOpenMode::Create))
}

fn input_latest_spec(session_id: &str, browser_id: &str) -> ChannelSpec {
    ChannelSpec::new(
        build_browser_channel_name(session_id, browser_id, ChannelKind::Input),
        ChannelKind::Input,
        INPUT_LATEST_CAPACITY_BYTES,
    )
}

fn open_input_queue(
    session_id: &str,
    browser_id: &str,
) -> Result<MappedSpscQueue, ipc::MappedQueueError> {
    let directory = ipc_directory();
    let spec = input_queue_spec(session_id, browser_id);
    MappedSpscQueue::open_in_dir(&directory, &spec, ChannelOpenMode::OpenExisting)
        .or_else(|_| MappedSpscQueue::open_in_dir(directory, &spec, ChannelOpenMode::Create))
}

fn input_queue_spec(session_id: &str, browser_id: &str) -> MappedQueueSpec {
    let name = format!(
        "{}_queue",
        build_browser_channel_name(session_id, browser_id, ChannelKind::Input)
    );
    MappedQueueSpec::new(
        name,
        ChannelKind::Input,
        INPUT_QUEUE_ITEM_CAPACITY,
        INPUT_QUEUE_MAX_PAYLOAD_LEN,
    )
}

fn ipc_directory() -> PathBuf {
    std::env::var_os("EBI_IPC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("EmbeddedBrowserIpc"))
}

fn read_output_latest(
    session_id: &str,
    browser_id: &str,
) -> Result<Option<ipc::LatestSnapshot>, ipc::IpcError> {
    let name = build_browser_channel_name(session_id, browser_id, ChannelKind::Output);
    let path = ipc_directory().join(&name);
    if !path.exists() {
        return Ok(None);
    }

    let spec = ChannelSpec::new(name, ChannelKind::Output, 64 * 1024);
    let mut channel =
        ChannelMappedFile::open_in_dir(ipc_directory(), &spec, ChannelOpenMode::OpenExisting)?;
    channel.try_read_latest()
}

fn open_or_create_session_channel(spec: &ChannelSpec) -> Result<ChannelMappedFile, ipc::IpcError> {
    let directory = ipc_directory();
    ChannelMappedFile::open_in_dir(&directory, spec, ChannelOpenMode::OpenExisting)
        .or_else(|_| ChannelMappedFile::open_in_dir(directory, spec, ChannelOpenMode::Create))
}

fn open_frame_channel_if_exists(
    session_id: &str,
    browser_id: &str,
) -> Result<Option<FrameChannel>, FrameCopyError> {
    let directory = ipc_directory();
    let spec = frame_spec(session_id, browser_id);
    let path = directory.join(&spec.name);
    if !path.exists() {
        return Ok(None);
    }

    FrameChannel::open_in_dir(directory, &spec, ChannelOpenMode::OpenExisting).map(Some)
}

fn frame_spec(session_id: &str, browser_id: &str) -> FrameChannelSpec {
    FrameChannelSpec::new(
        build_browser_channel_name(session_id, browser_id, ChannelKind::Frame),
        FRAME_SLOT_COUNT,
        FRAME_SLOT_SIZE,
    )
}

fn copy_payload_to_c_buffer(
    payload: &[u8],
    buffer_ptr: *mut u8,
    buffer_len: usize,
    written_ptr: *mut usize,
) -> EbiErrorCode {
    unsafe {
        *written_ptr = payload.len();
    }

    if payload.len() > buffer_len {
        return EBI_ERROR_BUFFER_TOO_SMALL;
    }

    unsafe {
        let buffer = std::slice::from_raw_parts_mut(buffer_ptr, buffer_len);
        buffer[..payload.len()].copy_from_slice(payload);
    }

    EBI_OK
}

fn push_control_payload(
    session: &mut EbiSession,
    kind: u32,
    sequence: u64,
    payload: &[u8],
) -> EbiErrorCode {
    match session
        .control_queue
        .try_push(MappedQueueItem::new(kind, sequence, payload))
    {
        Ok(()) => EBI_OK,
        Err(ipc::MappedQueueError::QueueFull { .. }) => EBI_ERROR_QUEUE_FULL,
        Err(ipc::MappedQueueError::PayloadTooLarge { .. }) => EBI_ERROR_BUFFER_TOO_SMALL,
        Err(_) => EBI_ERROR_IO,
    }
}

fn read_ffi_string(ptr: *const u8, len: usize) -> Option<String> {
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    std::str::from_utf8(bytes).ok().map(ToOwned::to_owned)
}

fn is_valid_browser_id(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
}

fn encode_mouse_latest_payload(x: i32, y: i32, buttons: u32, valid: bool) -> [u8; 16] {
    let mut payload = [0u8; MOUSE_LATEST_PAYLOAD_LEN];
    payload[0..4].copy_from_slice(&x.to_le_bytes());
    payload[4..8].copy_from_slice(&y.to_le_bytes());
    payload[8..12].copy_from_slice(&buttons.to_le_bytes());
    payload[12] = u8::from(valid);
    payload
}

fn input_event_kind_from_u32(kind: u32) -> Option<InputEventKind> {
    match kind {
        1 => Some(InputEventKind::MouseButton),
        2 => Some(InputEventKind::MouseWheel),
        3 => Some(InputEventKind::Keyboard),
        4 => Some(InputEventKind::Ime),
        5 => Some(InputEventKind::Script),
        _ => None,
    }
}
