//! Headless Browser main entry point
//!
//! This is the standalone executable for testing and production use.

use std::collections::HashMap;
use std::time::Duration;

use headless_browser::browser::{shutdown_browser_runtime, BrowserConfig, BrowserEntry};
use headless_browser::ipc::SharedMemoryWrapper;
use headless_browser::modules::{HandlerCommand, HeartbeatPayload};
use log::{error, info, warn};

const SINGLE_INSTANCE_LOCK: &str = "com.kimtech.headless-browser";

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
    let Some(_instance_lock) = SingleInstanceLock::acquire(SINGLE_INSTANCE_LOCK) else {
        error!(
            "HeadlessBrowser single-instance lock is already held: {}. handlerGuid={}",
            SINGLE_INSTANCE_LOCK, args.guid
        );
        std::process::exit(73);
    };

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
    let app = AppBuilder::build(HeadlessApp::new(false));
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

fn run_unity_handler_mode(args: CliArgs) {
    info!("Running Unity handler mode: {}", args.guid);
    info!(
        "Graphics mode: requested={:?}, effective={:?}, reason={}",
        args.graphics_mode.requested_mode,
        args.graphics_mode.effective_mode,
        args.graphics_mode.reason
    );
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
    let mut browsers: HashMap<String, BrowserEntry> = HashMap::new();
    let started_at = std::time::Instant::now();
    let mut watchdog = HeartbeatWatchdog::new(
        args.guid.clone(),
        args.heartbeat_options.timeout,
        args.heartbeat_options.stall_grace,
    );

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        let now = started_at.elapsed();
        observe_heartbeat(&heartbeat, &mut watchdog, now);
        let check = watchdog.check(now);
        if check.should_shutdown {
            warn!("{}", watchdog.format_status(&check));
            break;
        }
        if check.scheduler_stalled {
            warn!(
                "Heartbeat: {}, monitor stalled ({}ms), granting recovery window",
                args.guid,
                check.scheduler_gap.as_millis()
            );
        }

        if let Ok(bytes) = handler.read_bytes() {
            if let Some(command) = HandlerCommand::from_bytes(&bytes) {
                let should_continue =
                    handle_unity_command(command, &args, &mut browsers, &mut handler);
                if !should_continue {
                    break;
                }
            }
        }

        for entry in browsers.values_mut() {
            entry.do_message_loop_work();
        }
        std::thread::sleep(Duration::from_millis(1));
    }

    for (_, mut entry) in browsers {
        entry.shutdown();
    }
    shutdown_browser_runtime();
}

fn observe_heartbeat(
    heartbeat: &SharedMemoryWrapper,
    watchdog: &mut HeartbeatWatchdog,
    now: Duration,
) {
    let Ok(bytes) = heartbeat.read_bytes() else {
        return;
    };
    let Some(payload) = HeartbeatPayload::from_bytes(&bytes) else {
        return;
    };
    watchdog.observe(payload, now);
}

fn handle_unity_command(
    command: HandlerCommand,
    args: &CliArgs,
    browsers: &mut HashMap<String, BrowserEntry>,
    handler: &mut SharedMemoryWrapper,
) -> bool {
    let should_continue = match command {
        HandlerCommand::Shutdown => false,
        HandlerCommand::AddBrowser {
            guid,
            width,
            height,
            address,
        } => {
            if let Some(entry) = browsers.get_mut(&guid) {
                entry.load_url(&address);
                entry.set_size(width, height);
                return true;
            }

            let insert_guid = guid.clone();
            let mut entry = BrowserEntry::with_config(BrowserConfig {
                width,
                height,
                url: address,
                memory_guid: guid,
                device_scale_factor: args.scale,
                frame_rate: args.fps,
                gpu_enabled: args.graphics_mode.effective_mode == GraphicsMode::On,
            });
            if let Err(error) = entry.initialize() {
                error!("Failed to initialize browser from Unity command: {error}");
                return false;
            }
            browsers.insert(insert_guid, entry);
            true
        }
        HandlerCommand::RemoveBrowser { guid } => {
            if let Some(mut entry) = browsers.remove(&guid) {
                entry.shutdown();
            }
            true
        }
        HandlerCommand::ResizeBrowser {
            guid,
            width,
            height,
        } => {
            if let Some(entry) = browsers.get_mut(&guid) {
                entry.set_size(width, height);
            }
            true
        }
    };
    let _ = handler.write_byte_at(0, 0);
    should_continue
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
    graphics_mode_request: GraphicsModeRequest,
    graphics_mode: GraphicsModeProfile,
    heartbeat_options: HeartbeatOptions,
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
            gpu_enabled: self.graphics_mode.effective_mode == GraphicsMode::On,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphicsModeRequest {
    Auto,
    Off,
    On,
}

impl GraphicsModeRequest {
    fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" => Self::Off,
            "on" => Self::On,
            _ => Self::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphicsMode {
    Off,
    On,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GraphicsModeProfile {
    requested_mode: GraphicsModeRequest,
    effective_mode: GraphicsMode,
    reason: &'static str,
}

impl GraphicsModeProfile {
    fn resolve(requested_mode: GraphicsModeRequest) -> Self {
        Self::resolve_with_gpu_available(requested_mode, detect_gpu_available())
    }

    fn resolve_with_gpu_available(
        requested_mode: GraphicsModeRequest,
        gpu_available: bool,
    ) -> Self {
        match requested_mode {
            GraphicsModeRequest::Off => Self {
                requested_mode,
                effective_mode: GraphicsMode::Off,
                reason: "GPU disabled by explicit off graphics mode",
            },
            GraphicsModeRequest::On => Self {
                requested_mode,
                effective_mode: GraphicsMode::On,
                reason: "GPU enabled by explicit on graphics mode",
            },
            GraphicsModeRequest::Auto if gpu_available => Self {
                requested_mode,
                effective_mode: GraphicsMode::On,
                reason: "GPU detected by auto graphics mode",
            },
            GraphicsModeRequest::Auto => Self {
                requested_mode,
                effective_mode: GraphicsMode::Off,
                reason: "GPU not detected by auto graphics mode",
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HeartbeatOptions {
    timeout: Duration,
    stall_grace: Duration,
}

impl Default for HeartbeatOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_millis(30_000),
            stall_grace: Duration::from_millis(30_000),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HeartbeatWatchdogCheck {
    should_shutdown: bool,
    scheduler_stalled: bool,
    since_last_heartbeat: Duration,
    scheduler_gap: Duration,
    last_sequence: i64,
    last_heartbeat_utc_ticks: i64,
}

#[derive(Debug, Clone)]
struct HeartbeatWatchdog {
    guid: String,
    timeout: Duration,
    stall_grace: Duration,
    last_heartbeat_monotonic: Duration,
    last_sequence: i64,
    last_heartbeat_utc_ticks: i64,
    last_check_monotonic: Duration,
    recovery_deadline: Option<Duration>,
    has_last_check: bool,
}

impl HeartbeatWatchdog {
    fn new(guid: String, timeout: Duration, stall_grace: Duration) -> Self {
        Self {
            guid,
            timeout,
            stall_grace,
            last_heartbeat_monotonic: Duration::ZERO,
            last_sequence: 0,
            last_heartbeat_utc_ticks: 0,
            last_check_monotonic: Duration::ZERO,
            recovery_deadline: None,
            has_last_check: false,
        }
    }

    fn observe(&mut self, payload: HeartbeatPayload, now: Duration) -> bool {
        if payload.sequence <= self.last_sequence {
            return false;
        }

        self.last_sequence = payload.sequence;
        self.last_heartbeat_utc_ticks = payload.utc_ticks;
        self.last_heartbeat_monotonic = now;
        if !self.has_last_check {
            self.last_check_monotonic = now;
            self.has_last_check = true;
        }
        self.recovery_deadline = None;
        true
    }

    fn check(&mut self, now: Duration) -> HeartbeatWatchdogCheck {
        let scheduler_gap = if self.has_last_check {
            now.saturating_sub(self.last_check_monotonic)
        } else {
            now
        };
        self.last_check_monotonic = now;
        self.has_last_check = true;

        let since_last_heartbeat = now.saturating_sub(self.last_heartbeat_monotonic);
        let scheduler_stalled = scheduler_gap > self.timeout;
        if scheduler_stalled && self.recovery_deadline.is_none() {
            self.recovery_deadline = Some(now + self.stall_grace);
        }

        let recovery_allows_wait = self
            .recovery_deadline
            .map(|deadline| now <= deadline)
            .unwrap_or(false);
        let should_shutdown = since_last_heartbeat > self.timeout && !recovery_allows_wait;

        HeartbeatWatchdogCheck {
            should_shutdown,
            scheduler_stalled,
            since_last_heartbeat,
            scheduler_gap,
            last_sequence: self.last_sequence,
            last_heartbeat_utc_ticks: self.last_heartbeat_utc_ticks,
        }
    }

    fn format_status(&self, check: &HeartbeatWatchdogCheck) -> String {
        format!(
            "Heartbeat: {}, Timeout ({}ms), lastSequence={}, lastHeartbeatUtcTicks={}, schedulerGap={}ms",
            self.guid,
            check.since_last_heartbeat.as_millis(),
            check.last_sequence,
            check.last_heartbeat_utc_ticks,
            check.scheduler_gap.as_millis()
        )
    }
}

fn parse_args() -> CliArgs {
    parse_args_from(std::env::args().collect())
}

fn parse_args_from(args: Vec<String>) -> CliArgs {
    let args = expand_packed_arguments(args);

    let mut result = CliArgs {
        url: "https://example.com".to_string(),
        width: 1280,
        height: 720,
        guid: uuid::Uuid::new_v4().to_string(),
        scale: 1.0,
        fps: 60,
        unity_handler_mode: false,
        graphics_mode_request: GraphicsModeRequest::Auto,
        graphics_mode: GraphicsModeProfile::resolve(GraphicsModeRequest::Auto),
        heartbeat_options: HeartbeatOptions::default(),
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
            "--graphics-mode" => {
                if i + 1 < args.len() {
                    result.graphics_mode_request = GraphicsModeRequest::parse(&args[i + 1]);
                    i += 1;
                }
            }
            "--heartbeat-timeout-ms" => {
                if i + 1 < args.len() {
                    if let Some(duration) = parse_positive_duration_ms(&args[i + 1]) {
                        result.heartbeat_options.timeout = duration;
                    }
                    i += 1;
                }
            }
            "--heartbeat-stall-grace-ms" => {
                if i + 1 < args.len() {
                    if let Some(duration) = parse_positive_duration_ms(&args[i + 1]) {
                        result.heartbeat_options.stall_grace = duration;
                    }
                    i += 1;
                }
            }
            "--help" => {
                print_usage();
                std::process::exit(0);
            }
            value if value.starts_with("--graphics-mode=") => {
                result.graphics_mode_request =
                    GraphicsModeRequest::parse(&value["--graphics-mode=".len()..]);
            }
            value if value.starts_with("--heartbeat-timeout-ms=") => {
                if let Some(duration) =
                    parse_positive_duration_ms(&value["--heartbeat-timeout-ms=".len()..])
                {
                    result.heartbeat_options.timeout = duration;
                }
            }
            value if value.starts_with("--heartbeat-stall-grace-ms=") => {
                if let Some(duration) =
                    parse_positive_duration_ms(&value["--heartbeat-stall-grace-ms=".len()..])
                {
                    result.heartbeat_options.stall_grace = duration;
                }
            }
            value => {
                if !value.starts_with('-') && is_first_non_option(&args, i) {
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

    result.graphics_mode = GraphicsModeProfile::resolve(result.graphics_mode_request);
    result
}

fn expand_packed_arguments(args: Vec<String>) -> Vec<String> {
    args.into_iter()
        .flat_map(|arg| {
            if arg.trim().is_empty() {
                Vec::new()
            } else if arg.contains(' ') {
                arg.split_whitespace().map(str::to_string).collect()
            } else {
                vec![arg]
            }
        })
        .collect()
}

fn parse_positive_duration_ms(value: &str) -> Option<Duration> {
    value
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .map(Duration::from_millis)
}

fn is_first_non_option(args: &[String], index: usize) -> bool {
    let mut i = 1;
    while i < index {
        let arg = &args[i];
        if !arg.starts_with('-') {
            return false;
        }
        if option_consumes_next_value(arg) {
            i += 1;
        }
        i += 1;
    }
    true
}

fn option_consumes_next_value(arg: &str) -> bool {
    matches!(
        arg.to_ascii_lowercase().as_str(),
        "--url"
            | "-u"
            | "--width"
            | "-w"
            | "--height"
            | "-h"
            | "--guid"
            | "-g"
            | "--scale"
            | "-s"
            | "--fps"
            | "-f"
            | "--graphics-mode"
            | "--heartbeat-timeout-ms"
            | "--heartbeat-stall-grace-ms"
    )
}

fn looks_like_guid(value: &str) -> bool {
    value.len() == 36
        && value.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
        && value.as_bytes()[8] == b'-'
        && value.as_bytes()[13] == b'-'
        && value.as_bytes()[18] == b'-'
        && value.as_bytes()[23] == b'-'
}

fn detect_gpu_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_dir("/dev/dri")
            .map(|entries| {
                entries.filter_map(Result::ok).any(|entry| {
                    entry
                        .file_name()
                        .to_str()
                        .map(|name| name.starts_with("renderD"))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    }

    #[cfg(target_os = "windows")]
    {
        detect_windows_gpu_available()
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        false
    }
}

#[cfg(target_os = "windows")]
fn detect_windows_gpu_available() -> bool {
    std::process::Command::new("reg")
        .args([
            "query",
            r"HKLM\SYSTEM\CurrentControlSet\Control\Video",
            "/s",
            "/v",
            "DriverDesc",
        ])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|stdout| stdout.lines().any(has_hardware_gpu_name))
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn has_hardware_gpu_name(adapter_name: &str) -> bool {
    let normalized = adapter_name.to_ascii_lowercase();
    !normalized.contains("microsoft basic")
        && !normalized.contains("remote display")
        && !normalized.contains("vmware")
        && !normalized.contains("virtualbox")
        && !normalized.contains("hyper-v")
        && !normalized.contains("virtio")
        && !normalized.contains("qxl")
        && !normalized.contains("citrix")
        && !normalized.contains("parallels")
        && (normalized.contains("nvidia")
            || normalized.contains("amd")
            || normalized.contains("radeon")
            || normalized.contains("intel")
            || normalized.contains("arc"))
}

struct SingleInstanceLock {
    #[cfg(target_os = "windows")]
    handle: winapi::shared::ntdef::HANDLE,
    #[cfg(target_os = "linux")]
    file: std::fs::File,
}

impl SingleInstanceLock {
    fn acquire(name: &str) -> Option<Self> {
        acquire_single_instance_lock(name)
    }
}

#[cfg(target_os = "windows")]
fn acquire_single_instance_lock(name: &str) -> Option<SingleInstanceLock> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use winapi::shared::winerror::ERROR_ALREADY_EXISTS;
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::synchapi::CreateMutexW;

    let wide_name: Vec<u16> = OsStr::new(name).encode_wide().chain(Some(0)).collect();
    let handle = unsafe { CreateMutexW(ptr::null_mut(), 0, wide_name.as_ptr()) };
    if handle.is_null() {
        return None;
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            winapi::um::handleapi::CloseHandle(handle);
        }
        return None;
    }
    Some(SingleInstanceLock { handle })
}

#[cfg(target_os = "linux")]
fn acquire_single_instance_lock(name: &str) -> Option<SingleInstanceLock> {
    use std::os::fd::AsRawFd;

    let path = std::env::temp_dir().join(format!("{name}.lock"));
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(path)
        .ok()?;
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    (result == 0).then_some(SingleInstanceLock { file })
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn acquire_single_instance_lock(_name: &str) -> Option<SingleInstanceLock> {
    Some(SingleInstanceLock {})
}

impl Drop for SingleInstanceLock {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        unsafe {
            winapi::um::handleapi::CloseHandle(self.handle);
        }

        #[cfg(target_os = "linux")]
        unsafe {
            use std::os::fd::AsRawFd;
            libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
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
    println!("      --graphics-mode <auto|on|off> Graphics mode (default: auto)");
    println!("      --heartbeat-timeout-ms <MS> Heartbeat timeout (default: 30000)");
    println!("      --heartbeat-stall-grace-ms <MS> Scheduler stall grace (default: 30000)");
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_args(args: &[&str]) -> Vec<String> {
        std::iter::once("headless_browser")
            .chain(args.iter().copied())
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn parses_packed_handler_graphics_and_heartbeat_arguments() {
        let args = parse_args_from(test_args(&[
            "12345678-1234-1234-1234-123456789abc --graphics-mode=off --heartbeat-timeout-ms 1500 --heartbeat-stall-grace-ms=2500",
        ]));

        assert!(args.unity_handler_mode);
        assert_eq!(args.guid, "12345678-1234-1234-1234-123456789abc");
        assert_eq!(args.graphics_mode_request, GraphicsModeRequest::Off);
        assert_eq!(args.heartbeat_options.timeout, Duration::from_millis(1500));
        assert_eq!(
            args.heartbeat_options.stall_grace,
            Duration::from_millis(2500)
        );
    }

    #[test]
    fn parses_graphics_mode_on_and_invalid_heartbeat_falls_back() {
        let args = parse_args_from(test_args(&[
            "--graphics-mode",
            "on",
            "--heartbeat-timeout-ms",
            "0",
            "--heartbeat-stall-grace-ms",
            "bad",
            "https://example.test",
        ]));

        assert!(!args.unity_handler_mode);
        assert_eq!(args.url, "https://example.test");
        assert_eq!(args.graphics_mode_request, GraphicsModeRequest::On);
        assert_eq!(args.heartbeat_options, HeartbeatOptions::default());
    }

    #[test]
    fn watchdog_ignores_duplicate_sequence_and_times_out() {
        let mut watchdog = HeartbeatWatchdog::new(
            "handler".to_string(),
            Duration::from_millis(100),
            Duration::from_millis(50),
        );

        assert!(watchdog.observe(
            HeartbeatPayload {
                sequence: 1,
                utc_ticks: 10,
            },
            Duration::from_millis(10),
        ));
        assert!(!watchdog.check(Duration::from_millis(20)).should_shutdown);
        assert!(!watchdog.observe(
            HeartbeatPayload {
                sequence: 1,
                utc_ticks: 20,
            },
            Duration::from_millis(80),
        ));

        let check = watchdog.check(Duration::from_millis(111));
        assert!(check.should_shutdown);
        assert_eq!(check.last_sequence, 1);
        assert_eq!(check.last_heartbeat_utc_ticks, 10);
    }

    #[test]
    fn watchdog_grants_recovery_grace_after_scheduler_stall() {
        let mut watchdog = HeartbeatWatchdog::new(
            "handler".to_string(),
            Duration::from_millis(100),
            Duration::from_millis(50),
        );

        assert!(watchdog.observe(
            HeartbeatPayload {
                sequence: 1,
                utc_ticks: 10,
            },
            Duration::from_millis(0),
        ));

        let stalled = watchdog.check(Duration::from_millis(150));
        assert!(stalled.scheduler_stalled);
        assert!(!stalled.should_shutdown);

        let still_grace = watchdog.check(Duration::from_millis(190));
        assert!(!still_grace.should_shutdown);

        let expired = watchdog.check(Duration::from_millis(201));
        assert!(expired.should_shutdown);
    }
}
