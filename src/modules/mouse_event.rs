//! Mouse Event Module - Handles mouse clicks and scroll events from Unity
//!
//! Protocol: 10 bytes max
//! [flag(1), eventType(1), x(2), y(2), deltaX(2), deltaY(2)]

use anyhow::Result;
use log::{debug, warn};

use super::base::MemoryModuleBase;
use super::protocol::{MouseEvent, MouseEventType};
use crate::ipc::SharedMemoryWrapper;

pub(crate) const IPC_INPUT_KIND_MOUSE_BUTTON: u32 = 1;
pub(crate) const IPC_INPUT_KIND_MOUSE_WHEEL: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IpcMouseButtonEvent {
    pub event_type: u8,
    pub x: i32,
    pub y: i32,
    pub buttons: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IpcMouseWheelEvent {
    pub x: i32,
    pub y: i32,
    pub delta_x: i32,
    pub delta_y: i32,
    pub buttons: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IpcMouseInputEvent {
    Button(IpcMouseButtonEvent),
    Wheel(IpcMouseWheelEvent),
}

/// Mouse event module (click + scroll queue)
pub struct MouseEventModule {
    memory_name: String,
    shmem: SharedMemoryWrapper,
    running: bool,
}

impl MouseEventModule {
    pub fn new(memory_name: &str) -> Result<Self> {
        let mut shmem = SharedMemoryWrapper::new(memory_name, MouseEvent::SIZE);
        shmem.initialize()?;
        Ok(Self {
            memory_name: memory_name.to_string(),
            shmem,
            running: true,
        })
    }

    fn read_event(&mut self) -> Option<MouseEvent> {
        let data = self.shmem.read_bytes().ok()?;
        if data.len() < 6 || data[0] != 1 {
            return None;
        }
        let event = MouseEvent::from_bytes(&data)?;
        let _ = self.shmem.write_byte_at(0, 0);
        Some(event)
    }

    /// Poll shared memory and forward any pending mouse event to the CEF browser host.
    #[cfg(feature = "cef")]
    pub fn poll(&mut self, host: &cef::BrowserHost) -> bool {
        use cef::{ImplBrowserHost, MouseButtonType, MouseEvent as CefMouseEvent};

        let Some(event) = self.read_event() else {
            return false;
        };

        let mouse_ev = CefMouseEvent {
            x: event.x as i32,
            y: event.y as i32,
            modifiers: 0,
        };

        if event.event_type == MouseEventType::Scroll as u8 {
            debug!(
                "Mouse scroll: ({},{}) delta=({},{})",
                event.x, event.y, event.delta_x, event.delta_y
            );
            host.send_mouse_wheel_event(
                Some(&mouse_ev),
                event.delta_x as i32,
                event.delta_y as i32,
            );
            return false;
        }

        let (button, mouse_up) = match event.event_type {
            x if x == MouseEventType::LeftDown as u8 => (MouseButtonType::LEFT, 0),
            x if x == MouseEventType::LeftUp as u8 => (MouseButtonType::LEFT, 1),
            x if x == MouseEventType::RightDown as u8 => (MouseButtonType::RIGHT, 0),
            x if x == MouseEventType::RightUp as u8 => (MouseButtonType::RIGHT, 1),
            x if x == MouseEventType::MiddleDown as u8 => (MouseButtonType::MIDDLE, 0),
            x if x == MouseEventType::MiddleUp as u8 => (MouseButtonType::MIDDLE, 1),
            other => {
                warn!("Unknown mouse event type: {}", other);
                return false;
            }
        };

        if cfg!(target_os = "linux") && event.event_type == MouseEventType::LeftDown as u8 {
            host.set_focus(1);
        }

        debug!(
            "Mouse click: ({},{}) button={:?} up={}",
            event.x, event.y, button, mouse_up
        );
        host.send_mouse_click_event(Some(&mouse_ev), button, mouse_up, 1);
        event.event_type == MouseEventType::LeftUp as u8
    }
}

pub(crate) fn decode_ipc_input_event(kind: u32, payload: &[u8]) -> Option<IpcMouseInputEvent> {
    match kind {
        IPC_INPUT_KIND_MOUSE_BUTTON => {
            decode_ipc_mouse_button_event(payload).map(IpcMouseInputEvent::Button)
        }
        IPC_INPUT_KIND_MOUSE_WHEEL => {
            decode_ipc_mouse_wheel_event(payload).map(IpcMouseInputEvent::Wheel)
        }
        _ => None,
    }
}

fn decode_ipc_mouse_button_event(payload: &[u8]) -> Option<IpcMouseButtonEvent> {
    if payload.len() < 13 {
        return None;
    }

    Some(IpcMouseButtonEvent {
        event_type: payload[0],
        x: i32::from_le_bytes(payload[1..5].try_into().ok()?),
        y: i32::from_le_bytes(payload[5..9].try_into().ok()?),
        buttons: u32::from_le_bytes(payload[9..13].try_into().ok()?),
    })
}

fn decode_ipc_mouse_wheel_event(payload: &[u8]) -> Option<IpcMouseWheelEvent> {
    if payload.len() < 20 {
        return None;
    }

    Some(IpcMouseWheelEvent {
        x: i32::from_le_bytes(payload[0..4].try_into().ok()?),
        y: i32::from_le_bytes(payload[4..8].try_into().ok()?),
        delta_x: i32::from_le_bytes(payload[8..12].try_into().ok()?),
        delta_y: i32::from_le_bytes(payload[12..16].try_into().ok()?),
        buttons: u32::from_le_bytes(payload[16..20].try_into().ok()?),
    })
}

impl MemoryModuleBase for MouseEventModule {
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
        decode_ipc_input_event, IpcMouseButtonEvent, IpcMouseInputEvent, IpcMouseWheelEvent,
    };
    use crate::modules::protocol::MouseEventType;

    #[test]
    fn decodes_ipc_v2_mouse_button_payload() {
        let mut payload = Vec::new();
        payload.push(MouseEventType::LeftDown as u8);
        payload.extend_from_slice(&10i32.to_le_bytes());
        payload.extend_from_slice(&20i32.to_le_bytes());
        payload.extend_from_slice(&1u32.to_le_bytes());

        let event = decode_ipc_input_event(1, &payload).unwrap();

        assert_eq!(
            IpcMouseInputEvent::Button(IpcMouseButtonEvent {
                event_type: MouseEventType::LeftDown as u8,
                x: 10,
                y: 20,
                buttons: 1,
            }),
            event
        );
        assert!(decode_ipc_input_event(1, &payload[..8]).is_none());
    }

    #[test]
    fn decodes_ipc_v2_mouse_wheel_payload() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&10i32.to_le_bytes());
        payload.extend_from_slice(&20i32.to_le_bytes());
        payload.extend_from_slice(&(-3i32).to_le_bytes());
        payload.extend_from_slice(&120i32.to_le_bytes());
        payload.extend_from_slice(&2u32.to_le_bytes());

        let event = decode_ipc_input_event(2, &payload).unwrap();

        assert_eq!(
            IpcMouseInputEvent::Wheel(IpcMouseWheelEvent {
                x: 10,
                y: 20,
                delta_x: -3,
                delta_y: 120,
                buttons: 2,
            }),
            event
        );
        assert!(decode_ipc_input_event(2, &payload[..16]).is_none());
    }
}
