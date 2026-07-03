//! Lichora main entry point
//!
//! This is the standalone executable for testing and production use.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use lichora_core::browser::{shutdown_browser_runtime, BrowserConfig, BrowserEntry};
use log::{error, info, warn};

const SINGLE_INSTANCE_LOCK: &str = "com.kimtech.headless-browser";

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

fn run_unity_handler_mode(args: CliArgs) {
    let mut run_log = UnityHandlerRunLog::open(&args.guid);
    run_log.write_line(&format!(
        "start args={:?}",
        std::env::args().collect::<Vec<_>>()
    ));
    info!("Running Unity handler mode: {}", args.guid);
    info!("Unity handler log: {}", run_log.path().to_string_lossy());
    run_log.write_line(&format!(
        "graphics requested={:?} effective={:?} reason={}",
        args.graphics_mode.requested_mode,
        args.graphics_mode.effective_mode,
        args.graphics_mode.reason
    ));
    info!(
        "Graphics mode: requested={:?}, effective={:?}, reason={}",
        args.graphics_mode.requested_mode,
        args.graphics_mode.effective_mode,
        args.graphics_mode.reason
    );
    let mut control_queue = match open_control_queue(&args.guid, &mut run_log) {
        Ok(queue) => queue,
        Err(error) => {
            error!(
                "Failed to open control queue for handler {}: {error}",
                args.guid
            );
            run_log.write_line(&format!("control queue open failed: {error}"));
            std::process::exit(1);
        }
    };

    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    ctrlc_handler(running.clone());
    let mut browsers: HashMap<String, BrowserEntry> = HashMap::new();
    let mut exit_reason = HandlerLoopExit::CtrlC;

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        let action = drain_control_queue(&mut control_queue, &args, &mut browsers, &mut run_log);
        if let HandlerLoopAction::Stop(reason) = action {
            run_log.write_line(&format!("exit requested: {}", reason.as_str()));
            exit_reason = reason;
            break;
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
    run_log.write_line(&format!("shutdown complete: {}", exit_reason.as_str()));
    if exit_reason.is_failure() {
        std::process::exit(1);
    }
}

fn open_control_queue(
    handler_guid: &str,
    run_log: &mut UnityHandlerRunLog,
) -> Result<ipc::MappedSpscQueue, ipc::MappedQueueError> {
    open_control_queue_in_dir(handler_guid, ipc_directory(), run_log)
}

fn open_control_queue_in_dir(
    handler_guid: &str,
    directory: impl AsRef<Path>,
    run_log: &mut UnityHandlerRunLog,
) -> Result<ipc::MappedSpscQueue, ipc::MappedQueueError> {
    let spec = control_queue_spec(handler_guid);
    match ipc::MappedSpscQueue::open_in_dir(&directory, &spec, ipc::ChannelOpenMode::OpenExisting)
        .or_else(|_| {
            ipc::MappedSpscQueue::open_in_dir(&directory, &spec, ipc::ChannelOpenMode::Create)
        }) {
        Ok(queue) => {
            let message = format!(
                "control queue ready: name={} path={}",
                spec.name,
                queue.path().to_string_lossy()
            );
            info!("{message}");
            run_log.write_line(&message);
            Ok(queue)
        }
        Err(error) => {
            run_log.write_line(&format!(
                "control queue unavailable: name={} error={error}",
                spec.name
            ));
            Err(error)
        }
    }
}

fn control_queue_spec(handler_guid: &str) -> ipc::MappedQueueSpec {
    ipc::control_queue_spec(handler_guid)
}

fn ipc_directory() -> PathBuf {
    std::env::var_os("EBI_IPC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("LichoraIpc"))
}

fn drain_control_queue(
    queue: &mut ipc::MappedSpscQueue,
    args: &CliArgs,
    browsers: &mut HashMap<String, BrowserEntry>,
    run_log: &mut UnityHandlerRunLog,
) -> HandlerLoopAction {
    loop {
        let item = match queue.try_pop() {
            Ok(Some(item)) => item,
            Ok(None) => return HandlerLoopAction::Continue,
            Err(error) => {
                warn!("Failed to read control queue item: {error}");
                run_log.write_line(&format!("control queue read failed: {error}"));
                return HandlerLoopAction::Continue;
            }
        };

        let command = match ipc::ControlCommand::decode(&item.payload) {
            Ok(command) => command,
            Err(error) => {
                warn!(
                    "Invalid control command sequence={} len={}: {}",
                    item.sequence,
                    item.payload.len(),
                    error
                );
                run_log.write_line(&format!(
                    "control command invalid sequence={} len={} error={error}",
                    item.sequence,
                    item.payload.len()
                ));
                continue;
            }
        };

        run_log.write_line(&format!(
            "control command sequence={} len={} {}",
            item.sequence,
            item.payload.len(),
            describe_control_command(&command)
        ));
        let action = apply_unity_command(command, args, browsers, run_log);
        if matches!(action, HandlerLoopAction::Stop(_)) {
            return action;
        }
    }
}

fn apply_unity_command(
    command: ipc::ControlCommand,
    args: &CliArgs,
    browsers: &mut HashMap<String, BrowserEntry>,
    run_log: &mut UnityHandlerRunLog,
) -> HandlerLoopAction {
    match command {
        ipc::ControlCommand::Shutdown => HandlerLoopAction::Stop(HandlerLoopExit::ShutdownCommand),
        ipc::ControlCommand::AddBrowser {
            browser_id,
            width,
            height,
            address,
        } => {
            if let Some(entry) = browsers.get_mut(&browser_id) {
                entry.load_url(&address);
                entry.set_size(width, height);
                HandlerLoopAction::Continue
            } else {
                let insert_guid = browser_id.clone();
                run_log.write_line(&format!("AddBrowser initialize begin guid={insert_guid}"));
                let mut entry = BrowserEntry::with_session_config(
                    BrowserConfig {
                        width,
                        height,
                        url: address,
                        memory_guid: browser_id,
                        device_scale_factor: args.scale,
                        frame_rate: args.fps,
                        gpu_enabled: args.graphics_mode.effective_mode == GraphicsMode::On,
                    },
                    args.guid.clone(),
                );
                if let Err(error) = entry.initialize() {
                    let message = format!("AddBrowser failed for {insert_guid}: {error}");
                    error!("{message}");
                    HandlerLoopAction::Stop(HandlerLoopExit::BrowserInitializeFailed(message))
                } else {
                    run_log.write_line(&format!("AddBrowser initialize ok guid={insert_guid}"));
                    browsers.insert(insert_guid, entry);
                    HandlerLoopAction::Continue
                }
            }
        }
        ipc::ControlCommand::RemoveBrowser { browser_id } => {
            if let Some(mut entry) = browsers.remove(&browser_id) {
                entry.shutdown();
            }
            HandlerLoopAction::Continue
        }
        ipc::ControlCommand::ResizeBrowser {
            browser_id,
            width,
            height,
        } => {
            if let Some(entry) = browsers.get_mut(&browser_id) {
                entry.set_size(width, height);
            }
            HandlerLoopAction::Continue
        }
    }
}

#[derive(Debug)]
enum HandlerLoopAction {
    Continue,
    Stop(HandlerLoopExit),
}

#[derive(Debug)]
enum HandlerLoopExit {
    CtrlC,
    ShutdownCommand,
    BrowserInitializeFailed(String),
}

impl HandlerLoopExit {
    fn as_str(&self) -> &str {
        match self {
            Self::CtrlC => "ctrl-c",
            Self::ShutdownCommand => "shutdown command",
            Self::BrowserInitializeFailed(message) => message.as_str(),
        }
    }

    fn is_failure(&self) -> bool {
        matches!(self, Self::BrowserInitializeFailed(_))
    }
}

struct UnityHandlerRunLog {
    path: PathBuf,
    file: Option<File>,
}

impl UnityHandlerRunLog {
    fn open(handler_guid: &str) -> Self {
        let path = std::env::temp_dir().join(format!("lichora-{handler_guid}.log"));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok();
        let mut result = Self { path, file };
        result.write_line("--- run ---");
        result
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }

    fn write_line(&mut self, message: &str) {
        let Some(file) = self.file.as_mut() else {
            return;
        };
        let _ = writeln!(file, "{message}");
        let _ = file.flush();
    }

    #[cfg(test)]
    fn disabled_for_test() -> Self {
        Self {
            path: PathBuf::new(),
            file: None,
        }
    }
}

fn describe_control_command(command: &ipc::ControlCommand) -> String {
    match command {
        ipc::ControlCommand::Shutdown => "Shutdown".to_string(),
        ipc::ControlCommand::AddBrowser {
            browser_id,
            width,
            height,
            address,
        } => format!(
            "AddBrowser guid={browser_id} size={width}x{height} address={}",
            truncate_for_log(address, 240)
        ),
        ipc::ControlCommand::RemoveBrowser { browser_id } => {
            format!("RemoveBrowser guid={browser_id}")
        }
        ipc::ControlCommand::ResizeBrowser {
            browser_id,
            width,
            height,
        } => format!("ResizeBrowser guid={browser_id} size={width}x{height}"),
    }
}

fn truncate_for_log(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let mut result = value.chars().take(max_chars).collect::<String>();
    result.push_str("...");
    result
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

fn parse_args() -> CliArgs {
    match parse_args_from(std::env::args().collect()) {
        Ok(args) => args,
        Err(error) => {
            error!("{error}");
            print_usage();
            std::process::exit(2);
        }
    }
}

fn parse_args_from(args: Vec<String>) -> Result<CliArgs, String> {
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
            "--help" => {
                print_usage();
                std::process::exit(0);
            }
            value if value.starts_with("--graphics-mode=") => {
                result.graphics_mode_request =
                    GraphicsModeRequest::parse(&value["--graphics-mode=".len()..]);
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown option: {value}"));
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
    Ok(result)
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
        true
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        false
    }
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
    println!("Lichora - embedded off-screen CEF runtime");
    println!();
    println!("Usage: lichora [OPTIONS] [URL]");
    println!();
    println!("Options:");
    println!("  -u, --url <URL>      Initial URL to load (default: https://example.com)");
    println!("  -w, --width <WIDTH>  Browser width (default: 1280)");
    println!("  -h, --height <HEIGHT> Browser height (default: 720)");
    println!("  -g, --guid <GUID>    Shared memory GUID (default: auto-generated)");
    println!("  -s, --scale <SCALE>  Device scale factor (default: 1.0)");
    println!("  -f, --fps <FPS>      Frame rate (default: 60)");
    println!("      --graphics-mode <auto|on|off> Graphics mode (default: auto)");
    println!("      --help           Show this help message");
    println!();
    println!("Examples:");
    println!("  lichora https://google.com");
    println!("  lichora -w 1920 -h 1080 -f 30");
    println!("  lichora --guid abc123 --url https://example.com");
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
        std::iter::once("lichora")
            .chain(args.iter().copied())
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn parses_packed_handler_graphics_arguments() {
        let args = parse_args_from(test_args(&[
            "12345678-1234-1234-1234-123456789abc --graphics-mode=off",
        ]))
        .unwrap();

        assert!(args.unity_handler_mode);
        assert_eq!(args.guid, "12345678-1234-1234-1234-123456789abc");
        assert_eq!(args.graphics_mode_request, GraphicsModeRequest::Off);
    }

    #[test]
    fn rejects_legacy_heartbeat_flags() {
        let error = match parse_args_from(test_args(&[
            "12345678-1234-1234-1234-123456789abc",
            "--heartbeat-timeout-ms",
            "1500",
        ])) {
            Ok(_) => panic!("legacy heartbeat timeout flag should be rejected"),
            Err(error) => error,
        };

        assert_eq!(error, "unknown option: --heartbeat-timeout-ms");

        let error = match parse_args_from(test_args(&["--heartbeat-stall-grace-ms=2500"])) {
            Ok(_) => panic!("legacy heartbeat stall grace flag should be rejected"),
            Err(error) => error,
        };

        assert_eq!(error, "unknown option: --heartbeat-stall-grace-ms=2500");
    }

    #[test]
    fn parses_graphics_mode_on_and_url() {
        let args = parse_args_from(test_args(&[
            "--graphics-mode",
            "on",
            "https://example.test",
        ]))
        .unwrap();

        assert!(!args.unity_handler_mode);
        assert_eq!(args.url, "https://example.test");
        assert_eq!(args.graphics_mode_request, GraphicsModeRequest::On);
    }

    #[test]
    fn open_control_queue_fails_when_directory_cannot_be_created() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let mut run_log = UnityHandlerRunLog::disabled_for_test();

        let result =
            open_control_queue_in_dir("handler", temp.path().join("control"), &mut run_log);

        assert!(result.is_err());
    }

    #[test]
    fn decodes_control_queue_payload_to_control_command() {
        let command = ipc::ControlCommand::AddBrowser {
            browser_id: "browser-b".to_string(),
            width: 800,
            height: 600,
            address: "https://control.example".to_string(),
        };

        assert_eq!(
            ipc::ControlCommand::decode(&command.encode()).unwrap(),
            command
        );
        assert!(ipc::ControlCommand::decode(b"not a command").is_err());
    }
}
