//! OffScreen Render Handler - Captures rendered frames and caret positions
//!
//! Implements CEF RenderHandler for off-screen (windowless) rendering.
//! Frames are written to shared memory so Unity can read them directly.
//! Caret position is updated via `on_ime_composition_range_changed`.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{
    atomic::{AtomicI32, AtomicU64, Ordering},
    Arc, Mutex,
};

use anyhow::Result;
use log::debug;
#[cfg(feature = "cef")]
use log::error;

use super::output::BrowserOutputIpcChannels;
use crate::modules::CaptureModule;

/// OffScreen render handler state (shared between CEF callbacks and BrowserEntry).
#[derive(Clone)]
pub struct OsrRenderHandler {
    width: Rc<RefCell<i32>>,
    height: Rc<RefCell<i32>>,
    device_scale_factor: f32,
    paint_state: Arc<PaintState>,
    /// Written from `on_paint` (CEF render thread).
    capture_module: Arc<Mutex<CaptureModule>>,
    /// Written from `on_ime_composition_range_changed` (CEF render thread).
    output_ipc: Arc<Mutex<BrowserOutputIpcChannels>>,
}

#[derive(Debug, Default)]
pub struct PaintState {
    sequence: AtomicU64,
    width: AtomicI32,
    height: AtomicI32,
}

impl PaintState {
    pub fn record_paint(&self, width: i32, height: i32) {
        self.width.store(width, Ordering::SeqCst);
        self.height.store(height, Ordering::SeqCst);
        self.sequence.fetch_add(1, Ordering::SeqCst);
    }

    pub fn snapshot(&self) -> PaintSnapshot {
        PaintSnapshot {
            sequence: self.sequence.load(Ordering::SeqCst),
            width: self.width.load(Ordering::SeqCst),
            height: self.height.load(Ordering::SeqCst),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaintSnapshot {
    pub sequence: u64,
    pub width: i32,
    pub height: i32,
}

impl OsrRenderHandler {
    pub(crate) fn new(
        width: i32,
        height: i32,
        device_scale_factor: f32,
        capture_module: Arc<Mutex<CaptureModule>>,
        output_ipc: Arc<Mutex<BrowserOutputIpcChannels>>,
    ) -> Self {
        Self {
            width: Rc::new(RefCell::new(width)),
            height: Rc::new(RefCell::new(height)),
            device_scale_factor,
            paint_state: Arc::new(PaintState::default()),
            capture_module,
            output_ipc,
        }
    }

    /// Update browser viewport size (call `BrowserHost::was_resized()` after this).
    pub fn set_size(&self, width: i32, height: i32) {
        *self.width.borrow_mut() = width;
        *self.height.borrow_mut() = height;
    }

    pub fn get_size(&self) -> (i32, i32) {
        (*self.width.borrow(), *self.height.borrow())
    }

    pub fn paint_snapshot(&self) -> PaintSnapshot {
        self.paint_state.snapshot()
    }

    // ------------------------------------------------------------------ //
    // Internal helpers called from the CEF macro-generated callbacks       //
    // ------------------------------------------------------------------ //

    fn on_paint_impl(
        &self,
        buffer: *const u8,
        width: i32,
        height: i32,
        dirty_rects: &[(i32, i32, i32, i32)],
    ) -> Result<()> {
        if buffer.is_null() || width <= 0 || height <= 0 {
            return Ok(());
        }

        let pixel_count = (width * height * 4) as usize;
        let pixels = unsafe { std::slice::from_raw_parts(buffer, pixel_count) };

        debug!(
            "on_paint: {}x{}, {} dirty rects",
            width,
            height,
            dirty_rects.len()
        );

        let published = self
            .capture_module
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))?
            .write_paint_frame(width, height, pixels, dirty_rects)?;
        if published {
            self.paint_state.record_paint(width, height);
        }

        Ok(())
    }

    fn update_caret(&self, x: i16, y: i16, height: i16) {
        if let Ok(mut output) = self.output_ipc.lock() {
            let _ = output.publish(ipc::OutputPayload::Caret(ipc::CaretOutput {
                x: i32::from(x),
                y: i32::from(y),
                width: 0,
                height: i32::from(height),
                visible: true,
            }));
            debug!("Caret updated: ({}, {}, h={})", x, y, height);
        }
    }
}

// ======================================================================= //
// CEF RenderHandler implementation via cef-rs macros                       //
// ======================================================================= //

#[cfg(feature = "cef")]
mod cef_impl {
    use super::*;
    use cef::*;

    wrap_render_handler! {
        pub struct RenderHandlerBuilder {
            handler: OsrRenderHandler,
        }

        impl RenderHandler {
            fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
                if let Some(rect) = rect {
                    let (w, h) = self.handler.get_size();
                    rect.x = 0;
                    rect.y = 0;
                    rect.width  = w;
                    rect.height = h;
                }
            }

            fn screen_info(
                &self,
                _browser: Option<&mut Browser>,
                screen_info: Option<&mut ScreenInfo>,
            ) -> ::std::os::raw::c_int {
                if let Some(info) = screen_info {
                    info.device_scale_factor = self.handler.device_scale_factor;
                    info.depth               = 24;
                    info.depth_per_component = 8;
                    info.is_monochrome       = 0;
                    return true as _;
                }
                false as _
            }

            fn screen_point(
                &self,
                _browser: Option<&mut Browser>,
                view_x: ::std::os::raw::c_int,
                view_y: ::std::os::raw::c_int,
                screen_x: Option<&mut ::std::os::raw::c_int>,
                screen_y: Option<&mut ::std::os::raw::c_int>,
            ) -> ::std::os::raw::c_int {
                if let Some(x) = screen_x { *x = view_x; }
                if let Some(y) = screen_y { *y = view_y; }
                true as _
            }

            fn on_paint(
                &self,
                _browser: Option<&mut Browser>,
                type_: PaintElementType,
                dirty_rects: Option<&[Rect]>,
                buffer: *const u8,
                width:  ::std::os::raw::c_int,
                height: ::std::os::raw::c_int,
            ) {
                if type_ != PaintElementType::VIEW {
                    return;
                }

                let rects: Vec<(i32, i32, i32, i32)> = dirty_rects
                    .unwrap_or(&[])
                    .iter()
                    .map(|r| (r.x, r.y, r.width, r.height))
                    .collect();

                if let Err(e) = self.handler.on_paint_impl(buffer, width, height, &rects) {
                    error!("on_paint error: {}", e);
                }
            }

            fn on_ime_composition_range_changed(
                &self,
                _browser: Option<&mut Browser>,
                _selected_range: Option<&Range>,
                character_bounds: Option<&[Rect]>,
            ) {
                if let Some(rect) = character_bounds.and_then(|s| s.last()) {
                    self.handler.update_caret(
                        (rect.x + rect.width) as i16,
                        (rect.y + rect.height) as i16,
                        rect.height as i16,
                    );
                }
            }
        }
    }

    impl RenderHandlerBuilder {
        pub fn build(handler: OsrRenderHandler) -> RenderHandler {
            Self::new(handler)
        }
    }
}

#[cfg(feature = "cef")]
pub use cef_impl::RenderHandlerBuilder;

// Stub for non-CEF builds (unit tests / Windows dev without CEF)
#[cfg(not(feature = "cef"))]
pub struct RenderHandlerBuilder;

#[cfg(not(feature = "cef"))]
impl RenderHandlerBuilder {
    pub fn build(handler: OsrRenderHandler) -> OsrRenderHandler {
        handler
    }
}
