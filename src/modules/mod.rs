//! Input modules for browser interaction
//!
//! These modules handle mouse, keyboard, and IME input from Unity

mod base;
mod capture;
mod caret;
mod ime;
mod keyboard;
mod mouse_event;
mod mouse_state;
mod protocol;
mod script;
mod surrounding_text;

pub use base::MemoryModuleBase;
pub use capture::CaptureModule;
pub use caret::{encode_caret_payload, parse_caret_console_payload, CaretModule};
pub use ime::ImeModule;
pub use keyboard::KeyboardModule;
pub use mouse_event::MouseEventModule;
pub(crate) use mouse_event::{decode_ipc_input_event, IpcMouseInputEvent};
pub use mouse_state::MouseStateModule;
pub(crate) use mouse_state::{button_flags_to_event_flags, IpcMouseLatest};
pub use protocol::*;
pub use script::{ScriptModule, CARET_PROBE_SCRIPT, SURROUNDING_TEXT_PROBE_SCRIPT};
pub use surrounding_text::{
    SurroundingTextContentType, SurroundingTextKind, SurroundingTextModule, SurroundingTextPayload,
    SurroundingTextSnapshot, SURROUNDING_TEXT_STACK_SIZE,
};
