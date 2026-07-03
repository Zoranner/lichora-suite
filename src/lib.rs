//! Lichora Rust runtime - CEF-based embedded web runtime.
//!
//! This library provides a CEF-based embedded web runtime using shared memory
//! for communication with host engines.
//!
//! # Features
//!
//! - Off-screen rendering via CEF
//! - Shared memory IPC with host engines
//! - Full input support (mouse, keyboard, IME)
//! - JavaScript execution
//! - DevTools support
//!
//! # Example
//!
//! ```rust,no_run
//! use lichora_core::browser::{BrowserEntry, BrowserConfig};
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

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize Lichora (FFI entry point)
#[no_mangle]
pub extern "C" fn lichora_init() -> i32 {
    // Initialize logger for FFI callers
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    0
}

/// Shutdown Lichora (FFI entry point)
#[no_mangle]
pub extern "C" fn lichora_shutdown() -> i32 {
    browser::shutdown_browser_runtime();
    0
}

/// Get library version (FFI entry point)
#[no_mangle]
pub extern "C" fn lichora_version() -> *const i8 {
    static VERSION_BYTES: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
    VERSION_BYTES.as_ptr() as *const i8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
