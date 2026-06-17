//! Browser Entry - Main browser instance management
//!
//! Corresponds to BrowserEntry in C# implementation.
//! Creates all shared-memory modules, initialises CEF, drives the message loop,
//! and polls input modules every tick.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
#[cfg(feature = "cef")]
use anyhow::{ensure, Context};
use log::info;

#[cfg(feature = "cef")]
use cef::*;

use super::render::OsrRenderHandler;
use crate::modules::{
    CaptureModule, CaretModule, ImeModule, KeyboardModule, MemoryModuleBase, MouseEventModule,
    MouseStateModule, ScriptModule, SurroundingTextModule, CARET_PROBE_SCRIPT,
    SURROUNDING_TEXT_PROBE_SCRIPT,
};

#[cfg(feature = "cef")]
static CEF_RUNTIME_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Configuration for a browser instance.
#[derive(Clone)]
pub struct BrowserConfig {
    pub width: i32,
    pub height: i32,
    pub url: String,
    pub memory_guid: String,
    pub device_scale_factor: f32,
    pub frame_rate: i32,
    pub gpu_enabled: bool,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            url: "about:blank".to_string(),
            memory_guid: uuid::Uuid::new_v4().to_string(),
            device_scale_factor: 1.0,
            frame_rate: 60,
            gpu_enabled: detect_gpu_available(),
        }
    }
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

// ======================================================================= //

/// Top-level browser manager.
pub struct BrowserEntry {
    config: BrowserConfig,
    initialized: bool,
    running: bool,

    // Output modules (render thread writes to these via Arc)
    capture_module: Option<Arc<Mutex<CaptureModule>>>,
    caret_module: Option<Arc<Mutex<CaretModule>>>,
    surrounding_text_module: Option<Arc<Mutex<SurroundingTextModule>>>,

    // Input modules (polled on main thread each tick)
    keyboard_module: Option<KeyboardModule>,
    mouse_state_module: Option<MouseStateModule>,
    mouse_event_module: Option<MouseEventModule>,
    ime_module: Option<ImeModule>,
    script_module: Option<ScriptModule>,

    render_handler: Option<OsrRenderHandler>,
    pending_probe_at: Option<Instant>,

    #[cfg(feature = "cef")]
    browser: Option<cef::Browser>,

    #[cfg(feature = "cef")]
    app: Option<cef::App>,
}

impl BrowserEntry {
    pub fn new() -> Self {
        Self::with_config(BrowserConfig::default())
    }

    pub fn with_config(config: BrowserConfig) -> Self {
        Self {
            config,
            initialized: false,
            running: false,
            capture_module: None,
            caret_module: None,
            surrounding_text_module: None,
            keyboard_module: None,
            mouse_state_module: None,
            mouse_event_module: None,
            ime_module: None,
            script_module: None,
            render_handler: None,
            pending_probe_at: None,
            #[cfg(feature = "cef")]
            browser: None,
            #[cfg(feature = "cef")]
            app: None,
        }
    }

    // ------------------------------------------------------------------ //
    // Public API                                                           //
    // ------------------------------------------------------------------ //

    /// Initialise all modules and CEF, then create the off-screen browser.
    pub fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        info!("Initializing Headless Browser…");
        info!("  Size   : {}x{}", self.config.width, self.config.height);
        info!("  URL    : {}", self.config.url);
        info!("  GUID   : {}", self.config.memory_guid);

        self.initialize_modules()?;

        #[cfg(feature = "cef")]
        self.initialize_cef()?;

        self.initialized = true;
        info!("Browser initialized successfully");
        Ok(())
    }

    /// Perform one iteration of the CEF message loop **and** poll all input modules.
    /// Call this in a tight loop (~10 ms sleep between calls).
    pub fn do_message_loop_work(&mut self) {
        #[cfg(feature = "cef")]
        {
            cef::do_message_loop_work();
            self.poll_input();
        }
    }

    /// Block until `stop()` is called (convenience wrapper).
    pub fn run_message_loop(&mut self) {
        if !self.initialized {
            return;
        }
        self.running = true;
        info!("Running message loop…");

        #[cfg(feature = "cef")]
        while self.running {
            self.do_message_loop_work();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        #[cfg(not(feature = "cef"))]
        while self.running {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn shutdown(&mut self) {
        if !self.initialized {
            return;
        }
        info!("Shutting down…");
        self.running = false;

        // Shutdown input modules
        if let Some(mut m) = self.keyboard_module.take() {
            m.shutdown();
        }
        if let Some(mut m) = self.mouse_state_module.take() {
            m.shutdown();
        }
        if let Some(mut m) = self.mouse_event_module.take() {
            m.shutdown();
        }
        if let Some(mut m) = self.ime_module.take() {
            m.shutdown();
        }
        if let Some(mut m) = self.script_module.take() {
            m.shutdown();
        }
        if let Some(m) = self.capture_module.take() {
            if let Ok(mut module) = m.lock() {
                module.shutdown();
            }
        }
        if let Some(m) = self.surrounding_text_module.take() {
            if let Ok(mut module) = m.lock() {
                module.shutdown();
            }
        }
        if let Some(m) = self.caret_module.take() {
            if let Ok(mut module) = m.lock() {
                module.shutdown();
            }
        }

        #[cfg(feature = "cef")]
        {
            if let Some(browser) = self.browser.take() {
                if let Some(host) = browser.host() {
                    host.close_browser(1);
                }
            }
        }

        self.initialized = false;
        info!("Shutdown complete");
    }

    // Navigation helpers ------------------------------------------------ //

    pub fn load_url(&mut self, url: &str) {
        self.config.url = url.to_string();
        #[cfg(feature = "cef")]
        if let Some(ref browser) = self.browser {
            if let Some(frame) = browser.main_frame() {
                frame.load_url(Some(&CefString::from(url)));
            }
        }
    }

    pub fn reload(&mut self) {
        #[cfg(feature = "cef")]
        if let Some(ref browser) = self.browser {
            browser.reload();
        }
    }

    pub fn execute_javascript(&mut self, _script: &str) {
        #[cfg(feature = "cef")]
        if let Some(ref browser) = self.browser {
            if let Some(frame) = browser.main_frame() {
                frame.execute_java_script(Some(&CefString::from(_script)), None, 0);
            }
        }
    }

    pub fn show_devtools(&mut self) {
        #[cfg(feature = "cef")]
        if let Some(ref browser) = self.browser {
            if let Some(host) = browser.host() {
                let window_info = WindowInfo {
                    windowless_rendering_enabled: false as _,
                    ..Default::default()
                };
                host.show_dev_tools(Some(&window_info), None, None, None);
            }
        }
    }

    pub fn set_size(&mut self, width: i32, height: i32) {
        self.config.width = width;
        self.config.height = height;
        if let Some(ref rh) = self.render_handler {
            rh.set_size(width, height);
        }
        #[cfg(feature = "cef")]
        if let Some(ref browser) = self.browser {
            if let Some(host) = browser.host() {
                host.was_resized();
            }
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    // ------------------------------------------------------------------ //
    // Private helpers                                                      //
    // ------------------------------------------------------------------ //

    fn initialize_modules(&mut self) -> Result<()> {
        let guid = self.config.memory_guid.clone();
        let w = self.config.width;
        let h = self.config.height;

        // Output modules — share their shmem Arcs with the render handler.
        let capture = CaptureModule::new_shared(&format!("Capture.{}", guid), w, h)?;
        self.capture_module = Some(capture.clone());

        let caret = CaretModule::new(&format!("Caret.{}", guid))?;
        let caret_shmem = caret.get_shmem();
        let caret = Arc::new(Mutex::new(caret));
        self.caret_module = Some(caret);

        let surrounding_text = Arc::new(Mutex::new(SurroundingTextModule::new(&format!(
            "SurroundingText.{}",
            guid
        ))?));
        self.surrounding_text_module = Some(surrounding_text);

        // Input modules — each owns its own shmem.
        self.keyboard_module = Some(KeyboardModule::new(&format!("KeyEvent.{}", guid))?);
        self.mouse_state_module = Some(MouseStateModule::new(&format!("MouseState.{}", guid))?);
        self.mouse_event_module = Some(MouseEventModule::new(&format!("MouseEvents.{}", guid))?);
        self.ime_module = Some(ImeModule::new(&format!("IME.{}", guid))?);
        self.script_module = Some(ScriptModule::new(&format!("Script.{}", guid))?);

        // Render handler receives the two output Arcs.
        self.render_handler = Some(OsrRenderHandler::new(
            w,
            h,
            self.config.device_scale_factor,
            capture,
            caret_shmem,
        ));

        info!("All modules initialized");
        Ok(())
    }

    #[cfg(feature = "cef")]
    fn initialize_cef(&mut self) -> Result<()> {
        use super::{AppBuilder, HeadlessApp};

        if CEF_RUNTIME_INITIALIZED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let app = AppBuilder::build(HeadlessApp::new(self.config.gpu_enabled));
            let args = cef::args::Args::new();
            let settings = Settings {
                windowless_rendering_enabled: true as _,
                external_message_pump: true as _,
                multi_threaded_message_loop: false as _,
                ..Default::default()
            };

            let ok = initialize(
                Some(args.as_main_args()),
                Some(&settings),
                Some(&mut app.clone()),
                std::ptr::null_mut(),
            );
            if ok != 1 {
                CEF_RUNTIME_INITIALIZED.store(false, Ordering::SeqCst);
                ensure!(ok == 1, "cef::initialize() failed");
            }

            self.app = Some(app);
            info!("CEF initialized");
        }

        self.create_browser()?;
        Ok(())
    }

    #[cfg(feature = "cef")]
    fn create_browser(&mut self) -> Result<()> {
        use super::ClientBuilder;

        let render_handler = self
            .render_handler
            .as_ref()
            .context("Render handler not initialised before create_browser()")?;

        let window_info = WindowInfo {
            windowless_rendering_enabled: true as _,
            shared_texture_enabled: false as _,
            external_begin_frame_enabled: false as _,
            ..Default::default()
        };

        let browser_settings = BrowserSettings {
            windowless_frame_rate: self.config.frame_rate,
            ..Default::default()
        };

        let url = CefString::from(self.config.url.as_str());
        let caret_module = self
            .caret_module
            .as_ref()
            .context("Caret module not initialised before create_browser()")?
            .clone();
        let surrounding_text_module = self
            .surrounding_text_module
            .as_ref()
            .context("SurroundingText module not initialised before create_browser()")?
            .clone();
        let client = ClientBuilder::build(
            render_handler.clone(),
            caret_module,
            surrounding_text_module,
        );

        let browser = cef::browser_host_create_browser_sync(
            Some(&window_info),
            Some(&mut client.clone()),
            Some(&url),
            Some(&browser_settings),
            None,
            None,
        )
        .context("browser_host_create_browser_sync() returned None")?;

        info!("Browser created, loading: {}", self.config.url);
        self.browser = Some(browser);
        Ok(())
    }

    /// Poll all input modules and forward events to CEF.
    /// Must be called from the same thread as `cef::do_message_loop_work()`.
    #[cfg(feature = "cef")]
    fn poll_input(&mut self) {
        let browser = match self.browser.clone() {
            Some(b) => b,
            None => return,
        };
        let Some(host) = browser.host() else { return };
        let mut should_probe = false;

        if let Some(ref mut m) = self.keyboard_module {
            should_probe |= m.poll(&host);
        }
        if let Some(ref mut m) = self.mouse_event_module {
            should_probe |= m.poll(&host);
        }
        if let Some(ref mut m) = self.mouse_state_module {
            m.poll(&host);
        }
        if let (Some(ref mut ime), Some(ref keyboard)) =
            (&mut self.ime_module, &self.keyboard_module)
        {
            should_probe |= ime.poll(&host, keyboard);
        }
        if let Some(ref mut m) = self.script_module {
            m.poll(&browser);
        }

        if should_probe {
            self.request_delayed_probe();
        }
        self.run_delayed_probe(&browser);
    }

    fn request_delayed_probe(&mut self) {
        self.pending_probe_at = Some(Instant::now() + Duration::from_millis(50));
    }

    #[cfg(feature = "cef")]
    fn run_delayed_probe(&mut self, browser: &cef::Browser) {
        let Some(deadline) = self.pending_probe_at else {
            return;
        };
        if Instant::now() < deadline {
            return;
        }

        self.pending_probe_at = None;
        ScriptModule::execute_script(browser, CARET_PROBE_SCRIPT);
        ScriptModule::execute_script(browser, SURROUNDING_TEXT_PROBE_SCRIPT);
    }
}

impl Default for BrowserEntry {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for BrowserEntry {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(feature = "cef")]
pub fn shutdown_browser_runtime() {
    if CEF_RUNTIME_INITIALIZED.swap(false, Ordering::SeqCst) {
        cef::shutdown();
    }
}

#[cfg(not(feature = "cef"))]
pub fn shutdown_browser_runtime() {}
