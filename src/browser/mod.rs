//! Browser module - Core browser functionality

mod cef_app;
mod entry;
mod handler;
mod render;

pub use cef_app::*;
pub use entry::{BrowserConfig, BrowserEntry};
pub use handler::PageHandler;
pub use render::{OsrRenderHandler, RenderHandlerBuilder};
