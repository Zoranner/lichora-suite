use lichora_core::browser::BrowserConfig;

use log::error;

pub(crate) struct CliArgs {
    pub(crate) url: String,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) guid: String,
    pub(crate) scale: f32,
    pub(crate) fps: i32,
    pub(crate) unity_handler_mode: bool,
    pub(crate) graphics_mode_request: GraphicsModeRequest,
    pub(crate) graphics_mode: GraphicsModeProfile,
}

impl CliArgs {
    pub(crate) fn to_browser_config(&self) -> BrowserConfig {
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
pub(crate) enum GraphicsModeRequest {
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
pub(crate) enum GraphicsMode {
    Off,
    On,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphicsModeProfile {
    pub(crate) requested_mode: GraphicsModeRequest,
    pub(crate) effective_mode: GraphicsMode,
    pub(crate) reason: &'static str,
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

pub(crate) fn parse_args() -> CliArgs {
    match parse_args_from(std::env::args().collect()) {
        Ok(args) => args,
        Err(error) => {
            error!("{error}");
            print_usage();
            std::process::exit(2);
        }
    }
}

pub(crate) fn parse_args_from(args: Vec<String>) -> Result<CliArgs, String> {
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

pub(crate) fn detect_gpu_available() -> bool {
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

pub(crate) fn print_usage() {
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

#[cfg(test)]
mod tests {
    fn test_args(args: &[&str]) -> Vec<String> {
        std::iter::once("lichora")
            .chain(args.iter().copied())
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn parses_packed_handler_graphics_arguments() {
        let args = super::parse_args_from(test_args(&[
            "12345678-1234-1234-1234-123456789abc --graphics-mode=off",
        ]))
        .unwrap();

        assert!(args.unity_handler_mode);
        assert_eq!(args.guid, "12345678-1234-1234-1234-123456789abc");
        assert_eq!(args.graphics_mode_request, super::GraphicsModeRequest::Off);
    }

    #[test]
    fn rejects_legacy_heartbeat_flags() {
        let error = match super::parse_args_from(test_args(&[
            "12345678-1234-1234-1234-123456789abc",
            "--heartbeat-timeout-ms",
            "1500",
        ])) {
            Ok(_) => panic!("legacy heartbeat timeout flag should be rejected"),
            Err(error) => error,
        };

        assert_eq!(error, "unknown option: --heartbeat-timeout-ms");

        let error = match super::parse_args_from(test_args(&["--heartbeat-stall-grace-ms=2500"])) {
            Ok(_) => panic!("legacy heartbeat stall grace flag should be rejected"),
            Err(error) => error,
        };

        assert_eq!(error, "unknown option: --heartbeat-stall-grace-ms=2500");
    }

    #[test]
    fn parses_graphics_mode_on_and_url() {
        let args = super::parse_args_from(test_args(&[
            "--graphics-mode",
            "on",
            "https://example.test",
        ]))
        .unwrap();

        assert!(!args.unity_handler_mode);
        assert_eq!(args.url, "https://example.test");
        assert_eq!(args.graphics_mode_request, super::GraphicsModeRequest::On);
    }
}
