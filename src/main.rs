//! Lichora main entry point
//!
//! This is the standalone executable for testing and production use.

mod cli;
mod handler;

use cli::parse_args;
use handler::run_unity_handler_mode;

use std::time::Duration;

use lichora_core::browser::{shutdown_browser_runtime, BrowserConfig, BrowserEntry};
use log::{error, info};
fn main() {
    if is_cef_subprocess() {
        execute_cef_subprocess();
    }

    // Initialize logger
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("=== Lichora (cef-rs) ===");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    let args = parse_args();

    if args.unity_handler_mode {
        run_unity_handler_mode(args);
        return;
    }

    // Create browser configuration
    let config = args.to_browser_config();
    run_single_browser(config);
}

#[cfg(feature = "cef")]
fn execute_cef_subprocess() {
    use cef::*;

    lichora_core::browser::configure_cef_api_version();
    let args = cef::args::Args::new();
    let exit_code = execute_process(Some(args.as_main_args()), None, std::ptr::null_mut());
    if exit_code >= 0 {
        std::process::exit(exit_code);
    }
}

#[cfg(not(feature = "cef"))]
fn execute_cef_subprocess() {}

fn is_cef_subprocess() -> bool {
    std::env::args()
        .skip(1)
        .any(|arg| arg == "--type" || arg.starts_with("--type="))
}

fn run_single_browser(config: BrowserConfig) {
    info!("Configuration:");
    info!("  URL: {}", config.url);
    info!("  Size: {}x{}", config.width, config.height);
    info!("  GUID: {}", config.memory_guid);
    info!("  Frame Rate: {}", config.frame_rate);
    info!("  Device Scale: {}", config.device_scale_factor);
    info!("  GPU Enabled: {}", config.gpu_enabled);

    // Create browser entry
    let mut browser = BrowserEntry::with_config(config);

    // Initialize browser
    if let Err(e) = browser.initialize() {
        error!("Failed to initialize browser: {}", e);
        std::process::exit(1);
    }

    info!("Browser initialized successfully");

    // Handle Ctrl+C
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    ctrlc_handler(running.clone());

    // Run message loop
    info!("Running message loop (press Ctrl+C to exit)...");
    while running.load(std::sync::atomic::Ordering::SeqCst) {
        browser.do_message_loop_work();
        std::thread::sleep(Duration::from_millis(10));
    }

    info!("Shutting down...");
    browser.shutdown();
    shutdown_browser_runtime();
    info!("Goodbye!");
}

fn ctrlc_handler(running: std::sync::Arc<std::sync::atomic::AtomicBool>) {
    ctrlc::set_handler(move || {
        running.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("Failed to set Ctrl+C handler");
}
