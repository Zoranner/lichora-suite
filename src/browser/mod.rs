//! Browser module - Core browser functionality

mod cef_app;
mod dom_bridge;
mod entry;
mod handler;
mod output;
mod render;

pub use cef_app::*;
pub use entry::{configure_cef_api_version, shutdown_browser_runtime, BrowserConfig, BrowserEntry};
pub use handler::PageHandler;
pub use render::{OsrRenderHandler, RenderHandlerBuilder};
