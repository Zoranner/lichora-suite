//! Browser module - Core browser functionality

#[cfg(feature = "cef")]
mod cef_app;
#[cfg(feature = "cef")]
mod dom_bridge;
#[cfg(feature = "cef")]
mod entry;
#[cfg(not(feature = "cef"))]
mod entry_stub;
#[cfg(feature = "cef")]
mod output;
#[cfg(feature = "cef")]
mod render;

#[cfg(feature = "cef")]
pub use cef_app::*;
#[cfg(feature = "cef")]
pub use entry::{configure_cef_api_version, shutdown_browser_runtime, BrowserConfig, BrowserEntry};
#[cfg(not(feature = "cef"))]
pub use entry_stub::{
    configure_cef_api_version, shutdown_browser_runtime, BrowserConfig, BrowserEntry,
};
#[cfg(feature = "cef")]
pub use render::{OsrRenderHandler, RenderHandlerBuilder};
