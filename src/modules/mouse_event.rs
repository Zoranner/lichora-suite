//! Mouse Event Module - Handles mouse clicks and scroll events from Unity
//!
//! Protocol: 10 bytes max
//! [flag(1), eventType(1), x(2), y(2), deltaX(2), deltaY(2)]

use anyhow::Result;
use log::{debug, warn};

use super::base::MemoryModuleBase;
use super::protocol::{MouseEvent, MouseEventType};
use crate::ipc::SharedMemoryWrapper;

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
    pub fn poll(&mut self, host: &cef::BrowserHost) {
        use cef::{ImplBrowserHost, MouseButtonType, MouseEvent as CefMouseEvent};

        let Some(event) = self.read_event() else {
            return;
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
            return;
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
                return;
            }
        };

        debug!(
            "Mouse click: ({},{}) button={:?} up={}",
            event.x, event.y, button, mouse_up
        );
        host.send_mouse_click_event(Some(&mouse_ev), button, mouse_up, 1);
    }
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
