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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MouseMoveSnapshot {
    x: i16,
    y: i16,
    buttons: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IpcMouseLatest {
    pub x: i32,
    pub y: i32,
    pub buttons: u32,
    pub valid: bool,
}

impl IpcMouseLatest {
    pub const SIZE: usize = 16;

    pub(crate) fn decode(payload: &[u8]) -> Option<Self> {
        if payload.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            x: i32::from_le_bytes(payload[0..4].try_into().ok()?),
            y: i32::from_le_bytes(payload[4..8].try_into().ok()?),
            buttons: u32::from_le_bytes(payload[8..12].try_into().ok()?),
            valid: payload[12] != 0,
        })
    }
}

#[derive(Debug, Default)]
struct MouseMoveCoalescer {
    latest: Option<MouseMoveSnapshot>,
    post_pending: bool,
}

impl MouseMoveCoalescer {
    fn queue(&mut self, snapshot: MouseMoveSnapshot) -> bool {
        self.latest = Some(snapshot);

        if self.post_pending {
            return false;
        }

        self.post_pending = true;
        true
    }

    fn try_take_latest(&mut self) -> Option<MouseMoveSnapshot> {
        match self.latest.take() {
            Some(snapshot) => Some(snapshot),
            None => {
                self.post_pending = false;
                None
            }
        }
    }
}

/// Mouse state module (continuous position + button state)
pub struct MouseStateModule {
    memory_name: String,
    shmem: SharedMemoryWrapper,
    running: bool,
    move_coalescer: MouseMoveCoalescer,
    last_mouse_x: i16,
    last_mouse_y: i16,
    has_last_position: bool,
    has_focus: bool,
}

impl MouseStateModule {
    pub fn new(memory_name: &str) -> Result<Self> {
        let mut shmem = SharedMemoryWrapper::new(memory_name, MouseState::SIZE);
        shmem.initialize()?;
        Ok(Self {
            memory_name: memory_name.to_string(),
            shmem,
            running: true,
            move_coalescer: MouseMoveCoalescer::default(),
            last_mouse_x: 0,
            last_mouse_y: 0,
            has_last_position: false,
            has_focus: false,
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

    fn queue_position(&mut self, state: MouseState) -> bool {
        if self.has_last_position && self.last_mouse_x == state.x && self.last_mouse_y == state.y {
            return false;
        }

        self.last_mouse_x = state.x;
        self.last_mouse_y = state.y;
        self.has_last_position = true;

        self.move_coalescer.queue(MouseMoveSnapshot {
            x: state.x,
            y: state.y,
            buttons: state.button_state,
        })
    }

    /// Poll shared memory and forward any updated mouse position to the CEF browser host.
    #[cfg(feature = "cef")]
    pub fn poll(&mut self, host: &cef::BrowserHost) {
        use cef::{ImplBrowserHost, MouseEvent as CefMouseEvent};

        let Some(state) = self.read_state() else {
            return;
        };

        self.queue_position(state);
        while let Some(snapshot) = self.move_coalescer.try_take_latest() {
            if !self.has_focus {
                host.set_focus(1);
                self.has_focus = true;
            }

            let mouse_ev = CefMouseEvent {
                x: snapshot.x as i32,
                y: snapshot.y as i32,
                modifiers: button_state_to_event_flags(snapshot.buttons),
            };

            debug!("Mouse move: ({},{})", snapshot.x, snapshot.y);
            host.send_mouse_move_event(Some(&mouse_ev), 0);
        }
    }
}

fn button_state_to_event_flags(button_state: u8) -> u32 {
    button_flags_to_event_flags(u32::from(button_state))
}

pub(crate) fn button_flags_to_event_flags(button_state: u32) -> u32 {
    let mut modifiers: u32 = 0;
    if button_state & u32::from(MouseState::BUTTON_LEFT) != 0 {
        modifiers |= EVENTFLAG_LEFT_MOUSE_BUTTON;
    }
    if button_state & u32::from(MouseState::BUTTON_RIGHT) != 0 {
        modifiers |= EVENTFLAG_RIGHT_MOUSE_BUTTON;
    }
    if button_state & u32::from(MouseState::BUTTON_MIDDLE) != 0 {
        modifiers |= EVENTFLAG_MIDDLE_MOUSE_BUTTON;
    }
    modifiers
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

#[cfg(test)]
mod tests {
    use super::{
        button_flags_to_event_flags, button_state_to_event_flags, IpcMouseLatest,
        MouseMoveCoalescer, MouseMoveSnapshot, MouseStateModule, EVENTFLAG_LEFT_MOUSE_BUTTON,
        EVENTFLAG_MIDDLE_MOUSE_BUTTON, EVENTFLAG_RIGHT_MOUSE_BUTTON,
    };
    use crate::modules::protocol::MouseState;

    #[test]
    fn coalescer_keeps_latest_snapshot_while_post_is_pending() {
        let mut coalescer = MouseMoveCoalescer::default();

        assert!(coalescer.queue(snapshot(10, 20, MouseState::BUTTON_LEFT)));
        assert!(!coalescer.queue(snapshot(30, 40, MouseState::BUTTON_RIGHT)));

        assert_eq!(
            Some(snapshot(30, 40, MouseState::BUTTON_RIGHT)),
            coalescer.try_take_latest()
        );
        assert_eq!(None, coalescer.try_take_latest());
        assert!(coalescer.queue(snapshot(50, 60, MouseState::BUTTON_MIDDLE)));
    }

    #[test]
    fn queue_position_skips_repeated_coordinates() {
        let name = format!("MouseState.{}", uuid::Uuid::new_v4());
        let mut module = MouseStateModule::new(&name).unwrap();

        assert!(module.queue_position(mouse_state(10, 20, MouseState::BUTTON_LEFT)));
        assert!(!module.queue_position(mouse_state(10, 20, MouseState::BUTTON_RIGHT)));
        assert!(!module.queue_position(mouse_state(11, 20, MouseState::BUTTON_RIGHT)));

        assert_eq!(
            Some(snapshot(11, 20, MouseState::BUTTON_RIGHT)),
            module.move_coalescer.try_take_latest()
        );
    }

    #[test]
    fn converts_button_state_to_cef_event_flags() {
        assert_eq!(0, button_state_to_event_flags(0));
        assert_eq!(
            EVENTFLAG_LEFT_MOUSE_BUTTON,
            button_state_to_event_flags(MouseState::BUTTON_LEFT)
        );
        assert_eq!(
            EVENTFLAG_LEFT_MOUSE_BUTTON
                | EVENTFLAG_RIGHT_MOUSE_BUTTON
                | EVENTFLAG_MIDDLE_MOUSE_BUTTON,
            button_state_to_event_flags(
                MouseState::BUTTON_LEFT | MouseState::BUTTON_RIGHT | MouseState::BUTTON_MIDDLE
            )
        );
    }

    #[test]
    fn converts_u32_button_state_to_cef_event_flags() {
        assert_eq!(0, button_flags_to_event_flags(0));
        assert_eq!(
            EVENTFLAG_LEFT_MOUSE_BUTTON,
            button_flags_to_event_flags(MouseState::BUTTON_LEFT as u32)
        );
        assert_eq!(
            EVENTFLAG_LEFT_MOUSE_BUTTON
                | EVENTFLAG_RIGHT_MOUSE_BUTTON
                | EVENTFLAG_MIDDLE_MOUSE_BUTTON,
            button_flags_to_event_flags(
                (MouseState::BUTTON_LEFT | MouseState::BUTTON_RIGHT | MouseState::BUTTON_MIDDLE)
                    as u32
            )
        );
    }

    #[test]
    fn decodes_ipc_v2_mouse_latest_payload() {
        let mut payload = [0u8; 16];
        payload[0..4].copy_from_slice(&123i32.to_le_bytes());
        payload[4..8].copy_from_slice(&456i32.to_le_bytes());
        payload[8..12].copy_from_slice(&7u32.to_le_bytes());
        payload[12] = 1;

        let latest = IpcMouseLatest::decode(&payload).unwrap();

        assert_eq!(123, latest.x);
        assert_eq!(456, latest.y);
        assert_eq!(7, latest.buttons);
        assert!(latest.valid);
        assert!(IpcMouseLatest::decode(&payload[..12]).is_none());
    }

    fn snapshot(x: i16, y: i16, buttons: u8) -> MouseMoveSnapshot {
        MouseMoveSnapshot { x, y, buttons }
    }

    fn mouse_state(x: i16, y: i16, button_state: u8) -> MouseState {
        MouseState {
            flag: 1,
            x,
            y,
            button_state,
        }
    }
}
