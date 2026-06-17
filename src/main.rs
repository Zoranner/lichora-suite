//! Headless Browser main entry point
//!
//! This is the standalone executable for testing and production use.

use std::time::Duration;

use headless_browser::browser::{BrowserConfig, BrowserEntry};
use headless_browser::ipc::SharedMemoryWrapper;
use headless_browser::modules::{HandlerCommand, HeartbeatPayload};
use log::{error, info, warn};

fn main() {
    execute_cef_subprocess();

    // Initialize logger
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("=== Headless Browser (cef-rs) ===");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Parse command line arguments
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
    use headless_browser::browser::{AppBuilder, HeadlessApp};

    let args = cef::args::Args::new();
    let app = AppBuilder::build(HeadlessApp::new());
    let exit_code = execute_process(
        Some(args.as_main_args()),
        Some(&mut app.clone()),
        std::ptr::null_mut(),
    );
    if exit_code >= 0 {
        std::process::exit(exit_code);
    }
}

#[cfg(not(feature = "cef"))]
fn execute_cef_subprocess() {}

fn run_single_browser(config: BrowserConfig) {
    info!("Configuration:");
    info!("  URL: {}", config.url);
    info!("  Size: {}x{}", config.width, config.height);
    info!("  GUID: {}", config.memory_guid);
    info!("  Frame Rate: {}", config.frame_rate);
    info!("  Device Scale: {}", config.device_scale_factor);

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
    info!("Goodbye!");
}

fn run_unity_handler_mode(args: CliArgs) {
    info!("Running Unity handler mode: {}", args.guid);
    let mut handler = SharedMemoryWrapper::new(&format!("Handler.{}", args.guid), 3000);
    if let Err(error) = handler.initialize() {
        error!("Failed to initialize handler stack: {error}");
        std::process::exit(1);
    }
    let mut heartbeat =
        SharedMemoryWrapper::new(&format!("HEARTBEAT.{}", args.guid), HeartbeatPayload::SIZE);
    if let Err(error) = heartbeat.initialize() {
        error!("Failed to initialize heartbeat stack: {error}");
        std::process::exit(1);
    }

    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    ctrlc_handler(running.clone());
    let mut browser: Option<BrowserEntry> = None;
    let mut last_heartbeat_sequence = 0;
    let mut last_heartbeat_at: Option<std::time::Instant> = None;

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        observe_heartbeat(
            &heartbeat,
            &mut last_heartbeat_sequence,
            &mut last_heartbeat_at,
        );
        if last_heartbeat_at
            .map(|instant| instant.elapsed() > Duration::from_secs(30))
            .unwrap_or(false)
        {
            warn!("Unity heartbeat timed out, shutting down Rust browser process");
            break;
        }

        if let Ok(bytes) = handler.read_bytes() {
            if let Some(command) = HandlerCommand::from_bytes(&bytes) {
                let should_continue =
                    handle_unity_command(command, &args, &mut browser, &mut handler);
                if !should_continue {
                    break;
                }
            }
        }

        if let Some(entry) = browser.as_mut() {
            entry.do_message_loop_work();
        }
        std::thread::sleep(Duration::from_millis(1));
    }

    if let Some(mut entry) = browser {
        entry.shutdown();
    }
}

fn observe_heartbeat(
    heartbeat: &SharedMemoryWrapper,
    last_sequence: &mut i64,
    last_seen_at: &mut Option<std::time::Instant>,
) {
    let Ok(bytes) = heartbeat.read_bytes() else {
        return;
    };
    let Some(payload) = HeartbeatPayload::from_bytes(&bytes) else {
        return;
    };
    if payload.sequence <= 0 || payload.sequence == *last_sequence {
        return;
    }

    *last_sequence = payload.sequence;
    *last_seen_at = Some(std::time::Instant::now());
}

fn handle_unity_command(
    command: HandlerCommand,
    args: &CliArgs,
    browser: &mut Option<BrowserEntry>,
    handler: &mut SharedMemoryWrapper,
) -> bool {
    let _ = handler.write_byte_at(0, 0);
    match command {
        HandlerCommand::Shutdown => false,
        HandlerCommand::AddBrowser {
            guid,
            width,
            height,
            address,
        } => {
            if let Some(entry) = browser.as_mut() {
                entry.load_url(&address);
                entry.set_size(width, height);
                return true;
            }

            let mut entry = BrowserEntry::with_config(BrowserConfig {
                width,
                height,
                url: address,
                memory_guid: guid,
                device_scale_factor: args.scale,
                frame_rate: args.fps,
            });
            if let Err(error) = entry.initialize() {
                error!("Failed to initialize browser from Unity command: {error}");
                return false;
            }
            *browser = Some(entry);
            true
        }
        HandlerCommand::RemoveBrowser { .. } => {
            if let Some(mut entry) = browser.take() {
                entry.shutdown();
            }
            true
        }
        HandlerCommand::ResizeBrowser { width, height, .. } => {
            if let Some(entry) = browser.as_mut() {
                entry.set_size(width, height);
            }
            true
        }
    }
}

/// Command line arguments
struct CliArgs {
    url: String,
    width: i32,
    height: i32,
    guid: String,
    scale: f32,
    fps: i32,
    unity_handler_mode: bool,
}

impl CliArgs {
    fn to_browser_config(&self) -> BrowserConfig {
        BrowserConfig {
            width: self.width,
            height: self.height,
            url: self.url.clone(),
            memory_guid: self.guid.clone(),
            device_scale_factor: self.scale,
            frame_rate: self.fps,
        }
    }
}

fn parse_args() -> CliArgs {
    let args: Vec<String> = std::env::args().collect();

    let mut result = CliArgs {
        url: "https://example.com".to_string(),
        width: 1280,
        height: 720,
        guid: uuid::Uuid::new_v4().to_string(),
        scale: 1.0,
        fps: 60,
        unity_handler_mode: false,
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--url" | "-u" => {
                if i + 1 < args.len() {
                    result.url = args[i + 1].clone();
                    i += 1;
                }
            }
            "--width" | "-w" => {
                if i + 1 < args.len() {
                    if let Ok(w) = args[i + 1].parse() {
                        result.width = w;
                    }
                    i += 1;
                }
            }
            "--height" | "-h" => {
                if i + 1 < args.len() {
                    if let Ok(h) = args[i + 1].parse() {
                        result.height = h;
                    }
                    i += 1;
                }
            }
            "--guid" | "-g" => {
                if i + 1 < args.len() {
                    result.guid = args[i + 1].clone();
                    i += 1;
                }
            }
            "--scale" | "-s" => {
                if i + 1 < args.len() {
                    if let Ok(s) = args[i + 1].parse() {
                        result.scale = s;
                    }
                    i += 1;
                }
            }
            "--fps" | "-f" => {
                if i + 1 < args.len() {
                    if let Ok(f) = args[i + 1].parse() {
                        result.fps = f;
                    }
                    i += 1;
                }
            }
            "--help" => {
                print_usage();
                std::process::exit(0);
            }
            _ => {
                if i == 1 && !args[i].starts_with('-') {
                    if looks_like_guid(&args[i]) {
                        result.guid = args[i].clone();
                        result.unity_handler_mode = true;
                    } else {
                        // First non-flag argument is treated as URL
                        result.url = args[i].clone();
                    }
                }
            }
        }
        i += 1;
    }

    result
}

fn looks_like_guid(value: &str) -> bool {
    value.len() == 36
        && value.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
        && value.as_bytes()[8] == b'-'
        && value.as_bytes()[13] == b'-'
        && value.as_bytes()[18] == b'-'
        && value.as_bytes()[23] == b'-'
}

fn print_usage() {
    println!("Headless Browser - Off-screen CEF browser for Unity integration");
    println!();
    println!("Usage: headless_browser [OPTIONS] [URL]");
    println!();
    println!("Options:");
    println!("  -u, --url <URL>      Initial URL to load (default: https://example.com)");
    println!("  -w, --width <WIDTH>  Browser width (default: 1280)");
    println!("  -h, --height <HEIGHT> Browser height (default: 720)");
    println!("  -g, --guid <GUID>    Shared memory GUID (default: auto-generated)");
    println!("  -s, --scale <SCALE>  Device scale factor (default: 1.0)");
    println!("  -f, --fps <FPS>      Frame rate (default: 60)");
    println!("      --help           Show this help message");
    println!();
    println!("Examples:");
    println!("  headless_browser https://google.com");
    println!("  headless_browser -w 1920 -h 1080 -f 30");
    println!("  headless_browser --guid abc123 --url https://example.com");
}

fn ctrlc_handler(running: std::sync::Arc<std::sync::atomic::AtomicBool>) {
    ctrlc::set_handler(move || {
        running.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("Failed to set Ctrl+C handler");
}
