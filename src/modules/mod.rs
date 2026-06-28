mod capture;
mod caret;
mod mouse_event;
mod script;
mod surrounding_text;

pub use capture::CaptureModule;
pub use caret::parse_caret_console_payload;
pub(crate) use mouse_event::{
    button_flags_to_event_flags, decode_ipc_input_payload, IpcImeEvent, IpcInputEvent,
    IpcKeyboardEvent, IpcKeyboardEventType, IpcMouseInputEvent, MouseEventType,
};
pub use script::{ScriptModule, CARET_PROBE_SCRIPT, SURROUNDING_TEXT_PROBE_SCRIPT};
pub use surrounding_text::{
    SurroundingTextContentType, SurroundingTextKind, SurroundingTextPayload,
    SurroundingTextSnapshot,
};
