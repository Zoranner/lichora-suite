//! Headless Browser Rust - CEF-based browser for Unity integration
//!
//! This library provides a CEF-based headless browser implementation
//! using shared memory for communication with Unity.
//!
//! # Features
//!
//! - Off-screen rendering via CEF
//! - Shared memory IPC with Unity
//! - Full input support (mouse, keyboard, IME)
//! - JavaScript execution
//! - DevTools support
//!
//! # Example
//!
//! ```rust,no_run
//! use headless_browser_rust::browser::{BrowserEntry, BrowserConfig};
//!
//! let config = BrowserConfig {
//!     width: 1280,
//!     height: 720,
//!     url: "https://example.com".to_string(),
//!     ..Default::default()
//! };
//!
//! let mut browser = BrowserEntry::with_config(config);
//! browser.initialize().unwrap();
//!
//! // Run message loop
//! loop {
//!     browser.do_message_loop_work();
//!     std::thread::sleep(std::time::Duration::from_millis(10));
//! }
//! ```

pub mod browser;
pub mod ipc;
pub mod modules;

use anyhow::Result;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the headless browser (FFI entry point)
#[no_mangle]
pub extern "C" fn headless_browser_init() -> i32 {
    // Initialize logger for FFI callers
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    match init_browser() {
        Ok(_) => 0,
        Err(e) => {
            log::error!("Failed to initialize browser: {}", e);
            -1
        }
    }
}

/// Shutdown the headless browser (FFI entry point)
#[no_mangle]
pub extern "C" fn headless_browser_shutdown() -> i32 {
    // TODO: Implement global shutdown logic
    0
}

/// Get library version (FFI entry point)
#[no_mangle]
pub extern "C" fn headless_browser_version() -> *const i8 {
    static VERSION_BYTES: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
    VERSION_BYTES.as_ptr() as *const i8
}

fn init_browser() -> Result<()> {
    // TODO: Implement browser initialization
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
