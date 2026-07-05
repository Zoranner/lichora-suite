mod capture;
mod caret;
mod mouse_event;
mod overlay;
mod ownership;
mod script;
mod surrounding_text;

pub use capture::CaptureModule;
pub use caret::parse_caret_console_payload;
pub(crate) use mouse_event::{
    button_flags_to_event_flags, decode_ipc_input_payload, IpcImeEvent, IpcInputEvent,
    IpcKeyboardEvent, IpcKeyboardEventType, IpcMouseInputEvent, MouseEventType,
};
pub use overlay::{
    parse_overlay_pass_map_console_payload, OVERLAY_PASS_MAP_CONSOLE_PREFIX,
    OVERLAY_PASS_MAP_MAX_REGIONS,
};
pub use ownership::{
    input_ownership_map_from_legacy_pass_map, parse_input_ownership_map_console_payload,
    INPUT_OWNERSHIP_MAP_CONSOLE_PREFIX, INPUT_OWNERSHIP_MAP_MAX_REGIONS,
};
pub use script::{ScriptModule, CARET_PROBE_SCRIPT, SURROUNDING_TEXT_PROBE_SCRIPT};
pub use surrounding_text::{
    SurroundingTextContentType, SurroundingTextKind, SurroundingTextPayload,
    SurroundingTextSnapshot,
};
