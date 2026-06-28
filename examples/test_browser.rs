//! Test browser example
//!
//! Basic test to verify browser initialization

use headless_browser_core::browser::BrowserEntry;

fn main() {
    // Initialize logger
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    log::info!("=== Headless Browser Test ===");

    // Create browser entry
    let mut browser = BrowserEntry::new();

    // Initialize browser
    match browser.initialize() {
        Ok(_) => log::info!("Browser initialized successfully"),
        Err(e) => {
            log::error!("Failed to initialize browser: {}", e);
            std::process::exit(1);
        }
    }

    log::info!("Test completed successfully!");
}
