use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use lichora_core::browser::{shutdown_browser_runtime, BrowserConfig, BrowserEntry};
use log::{error, info, warn};

use crate::cli::{CliArgs, GraphicsMode};
use crate::ctrlc_handler;

pub(crate) fn run_unity_handler_mode(args: CliArgs) {
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

pub(crate) fn open_control_queue_in_dir(
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

pub(crate) struct UnityHandlerRunLog {
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

    pub(crate) fn path(&self) -> &std::path::Path {
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
    pub(crate) fn disabled_for_test() -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

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
