//! Mouse State Module - Handles continuous mouse position updates from Unity
//!
//! Protocol: 6 bytes
//! [flag(1), x(2), y(2), buttonState(1)]

use anyhow::Result;
use log::debug;

use super::base::MemoryModuleBase;
use super::protocol::MouseState;
use crate::ipc::SharedMemoryWrapper;

// CEF event-flag bit values for mouse buttons
const EVENTFLAG_LEFT_MOUSE_BUTTON: u32 = 16;
const EVENTFLAG_MIDDLE_MOUSE_BUTTON: u32 = 32;
const EVENTFLAG_RIGHT_MOUSE_BUTTON: u32 = 64;

/// Mouse state module (continuous position + button state)
pub struct MouseStateModule {
    memory_name: String,
    shmem: SharedMemoryWrapper,
    running: bool,
}

impl MouseStateModule {
    pub fn new(memory_name: &str) -> Result<Self> {
        let mut shmem = SharedMemoryWrapper::new(memory_name, MouseState::SIZE);
        shmem.initialize()?;
        Ok(Self {
            memory_name: memory_name.to_string(),
            shmem,
            running: true,
        })
    }

    fn read_state(&mut self) -> Option<MouseState> {
        let data = self.shmem.read_bytes().ok()?;
        if data.len() < MouseState::SIZE || data[0] != 1 {
            return None;
        }
        MouseState::from_bytes(&data)
        // Mouse state is not acknowledged — Unity overwrites it continuously.
    }

    /// Poll shared memory and forward any updated mouse position to the CEF browser host.
    #[cfg(feature = "cef")]
    pub fn poll(&mut self, host: &cef::BrowserHost) {
        use cef::{ImplBrowserHost, MouseEvent as CefMouseEvent};

        let Some(state) = self.read_state() else {
            return;
        };

        let mut modifiers: u32 = 0;
        if state.button_state & MouseState::BUTTON_LEFT != 0 {
            modifiers |= EVENTFLAG_LEFT_MOUSE_BUTTON;
        }
        if state.button_state & MouseState::BUTTON_RIGHT != 0 {
            modifiers |= EVENTFLAG_RIGHT_MOUSE_BUTTON;
        }
        if state.button_state & MouseState::BUTTON_MIDDLE != 0 {
            modifiers |= EVENTFLAG_MIDDLE_MOUSE_BUTTON;
        }

        let mouse_ev = CefMouseEvent {
            x: state.x as i32,
            y: state.y as i32,
            modifiers,
        };

        debug!("Mouse move: ({},{})", state.x, state.y);
        host.send_mouse_move_event(Some(&mouse_ev), 0);
    }
}

impl MemoryModuleBase for MouseStateModule {
    fn get_memory_name(&self) -> &str {
        &self.memory_name
    }
    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }
    fn shutdown(&mut self) {
        self.running = false;
    }
    fn is_running(&self) -> bool {
        self.running
    }
}
