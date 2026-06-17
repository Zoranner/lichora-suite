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

pub use base::MemoryModuleBase;
pub use capture::CaptureModule;
pub use caret::CaretModule;
pub use ime::ImeModule;
pub use keyboard::KeyboardModule;
pub use mouse_event::MouseEventModule;
pub use mouse_state::MouseStateModule;
pub use protocol::*;
pub use script::ScriptModule;
