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
use log::{info, warn};

#[cfg(feature = "cef")]
use cef::*;

use super::render::OsrRenderHandler;
#[cfg(feature = "cef")]
use super::reveal_and_focus_for_linux;
use crate::modules::{
    CaptureModule, CaretModule, ImeModule, KeyboardModule, MemoryModuleBase, MouseEventModule,
    MouseStateModule, ScriptModule, SurroundingTextModule, CARET_PROBE_SCRIPT,
    SURROUNDING_TEXT_PROBE_SCRIPT,
};

#[cfg(feature = "cef")]
static CEF_RUNTIME_INITIALIZED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "cef")]
static CEF_API_VERSION_CONFIGURED: AtomicBool = AtomicBool::new(false);

const MAX_WIDTH: i32 = 2560;
const MAX_HEIGHT: i32 = 1440;
const REPAINT_MAX_ATTEMPTS: u8 = 12;
const REPAINT_DELAY: Duration = Duration::from_millis(33);
const CLOSE_WAIT_TIMEOUT: Duration = Duration::from_secs(5);

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
    repaint_loop_id: u64,
    pending_repaint: Option<PendingRepaint>,
    closed: Arc<AtomicBool>,
    page_loaded: Arc<AtomicBool>,
    loading: Arc<AtomicBool>,
    close_requested: bool,

    #[cfg(feature = "cef")]
    browser: Option<cef::Browser>,
    #[cfg(feature = "cef")]
    client: Option<cef::Client>,
    #[cfg(feature = "cef")]
    browser_slot: Arc<Mutex<Option<cef::Browser>>>,

    #[cfg(feature = "cef")]
    app: Option<cef::App>,
}

#[derive(Debug, Clone)]
struct PendingRepaint {
    loop_id: u64,
    reason: &'static str,
    expected_width: i32,
    expected_height: i32,
    start_paint_sequence: u64,
    attempts: u8,
    next_attempt_at: Instant,
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
            repaint_loop_id: 0,
            pending_repaint: None,
            closed: Arc::new(AtomicBool::new(false)),
            page_loaded: Arc::new(AtomicBool::new(false)),
            loading: Arc::new(AtomicBool::new(false)),
            close_requested: false,
            #[cfg(feature = "cef")]
            browser: None,
            #[cfg(feature = "cef")]
            client: None,
            #[cfg(feature = "cef")]
            browser_slot: Arc::new(Mutex::new(None)),
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

    /// Poll input modules, then perform one iteration of the CEF message loop.
    /// Call this in a tight loop (~10 ms sleep between calls).
    pub fn do_message_loop_work(&mut self) {
        #[cfg(feature = "cef")]
        {
            self.poll_input();
            cef::do_message_loop_work();
            self.consume_page_loaded_event();
            self.run_repaint_loop();
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
        self.repaint_loop_id = self.repaint_loop_id.wrapping_add(1);
        self.pending_repaint = None;

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
            let close_posted = self
                .browser
                .as_ref()
                .and_then(|browser| browser.host())
                .map(|host| {
                    self.close_requested = true;
                    host.close_browser(1);
                })
                .is_some();
            if close_posted {
                self.wait_for_close();
            }
            self.browser = None;
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
        if self.loading.load(Ordering::SeqCst) {
            return;
        }
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

    pub fn close_devtools(&mut self) {
        #[cfg(feature = "cef")]
        if let Some(ref browser) = self.browser {
            if let Some(host) = browser.host() {
                host.close_dev_tools();
            }
        }
    }

    pub fn set_size(&mut self, width: i32, height: i32) {
        if width <= 0 || height <= 0 {
            return;
        }

        let clamped_width = width.min(MAX_WIDTH);
        let clamped_height = height.min(MAX_HEIGHT);
        if width > MAX_WIDTH || height > MAX_HEIGHT {
            warn!(
                "Resize request {}x{} exceeds maximum {}x{}; clamped to {}x{}",
                width, height, MAX_WIDTH, MAX_HEIGHT, clamped_width, clamped_height
            );
        }

        if clamped_width == self.config.width && clamped_height == self.config.height {
            return;
        }

        self.config.width = clamped_width;
        self.config.height = clamped_height;
        if let Some(ref rh) = self.render_handler {
            rh.set_size(clamped_width, clamped_height);
        }
        #[cfg(feature = "cef")]
        if let Some(ref browser) = self.browser {
            if let Some(host) = browser.host() {
                host.was_resized();
                reveal_and_focus_for_linux(&host);
            }
        }
        self.start_repaint_loop("resize", clamped_width, clamped_height);
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

        configure_cef_api_version();
        if CEF_RUNTIME_INITIALIZED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let mut app = AppBuilder::build(HeadlessApp::new(self.config.gpu_enabled));
            let args = cef::args::Args::new();
            let settings = Settings {
                windowless_rendering_enabled: true as _,
                external_message_pump: false as _,
                cache_path: CefString::from(cache_path().as_str()),
                multi_threaded_message_loop: false as _,
                ..Default::default()
            };

            let ok = initialize(
                Some(args.as_main_args()),
                Some(&settings),
                Some(&mut app),
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
        let mut client = ClientBuilder::build(
            render_handler.clone(),
            caret_module,
            surrounding_text_module,
            self.closed.clone(),
            self.browser_slot.clone(),
            self.page_loaded.clone(),
            self.loading.clone(),
        );

        let created = cef::browser_host_create_browser(
            Some(&window_info),
            Some(&mut client),
            Some(&url),
            Some(&browser_settings),
            None,
            None,
        ) != 0;
        ensure!(created, "browser_host_create_browser() failed");

        let browser = self
            .wait_for_browser_created()
            .context("Timed out waiting for CEF browser creation")?;

        info!("Browser created, loading: {}", self.config.url);
        if let Some(host) = browser.host() {
            reveal_and_focus_for_linux(&host);
            host.invalidate(PaintElementType::VIEW);
        }
        self.closed.store(false, Ordering::SeqCst);
        self.page_loaded.store(false, Ordering::SeqCst);
        self.loading.store(false, Ordering::SeqCst);
        self.browser = Some(browser);
        self.client = Some(client);
        self.start_repaint_loop("browser-created", self.config.width, self.config.height);
        Ok(())
    }

    #[cfg(feature = "cef")]
    fn wait_for_browser_created(&mut self) -> Option<cef::Browser> {
        let deadline = Instant::now() + CLOSE_WAIT_TIMEOUT;
        while Instant::now() < deadline {
            if let Ok(mut slot) = self.browser_slot.lock() {
                if let Some(browser) = slot.take() {
                    return Some(browser);
                }
            }
            cef::do_message_loop_work();
            std::thread::sleep(Duration::from_millis(1));
        }
        None
    }

    #[cfg(feature = "cef")]
    fn consume_page_loaded_event(&mut self) {
        if self.page_loaded.swap(false, Ordering::SeqCst) {
            self.start_repaint_loop("page-loaded", self.config.width, self.config.height);
        }
    }

    fn start_repaint_loop(
        &mut self,
        reason: &'static str,
        expected_width: i32,
        expected_height: i32,
    ) {
        if expected_width <= 0 || expected_height <= 0 {
            return;
        }
        let Some(render_handler) = self.render_handler.as_ref() else {
            return;
        };

        self.repaint_loop_id = self.repaint_loop_id.wrapping_add(1);
        self.pending_repaint = Some(PendingRepaint {
            loop_id: self.repaint_loop_id,
            reason,
            expected_width,
            expected_height,
            start_paint_sequence: render_handler.paint_snapshot().sequence,
            attempts: 0,
            next_attempt_at: Instant::now(),
        });
    }

    #[cfg(feature = "cef")]
    fn run_repaint_loop(&mut self) {
        let Some(pending) = self.pending_repaint.as_mut() else {
            return;
        };
        if pending.loop_id != self.repaint_loop_id {
            self.pending_repaint = None;
            return;
        }

        let Some(render_handler) = self.render_handler.as_ref() else {
            self.pending_repaint = None;
            return;
        };

        let snapshot = render_handler.paint_snapshot();
        if snapshot.sequence > pending.start_paint_sequence
            && snapshot.width == pending.expected_width
            && snapshot.height == pending.expected_height
        {
            self.pending_repaint = None;
            return;
        }

        if Instant::now() < pending.next_attempt_at {
            return;
        }

        if pending.attempts >= REPAINT_MAX_ATTEMPTS {
            warn!(
                "Repaint loop timeout [{}]: target={}x{}, last={}x{}",
                pending.reason,
                pending.expected_width,
                pending.expected_height,
                snapshot.width,
                snapshot.height
            );
            self.pending_repaint = None;
            return;
        }

        pending.attempts += 1;
        pending.next_attempt_at = Instant::now() + REPAINT_DELAY;
        self.post_repaint_request();
    }

    #[cfg(feature = "cef")]
    fn post_repaint_request(&self) {
        let Some(browser) = self.browser.as_ref() else {
            return;
        };
        let Some(host) = browser.host() else {
            return;
        };
        reveal_and_focus_for_linux(&host);
        host.invalidate(PaintElementType::VIEW);
    }

    #[cfg(feature = "cef")]
    fn wait_for_close(&mut self) {
        let deadline = Instant::now() + CLOSE_WAIT_TIMEOUT;
        while !self.closed.load(Ordering::SeqCst) && Instant::now() < deadline {
            cef::do_message_loop_work();
            std::thread::sleep(Duration::from_millis(10));
        }
        if !self.closed.load(Ordering::SeqCst) {
            warn!("Timed out waiting for CEF browser close callback");
        }
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
            m.poll(&browser, self.loading.load(Ordering::SeqCst));
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
        if self.loading.load(Ordering::SeqCst) {
            return;
        }
        ScriptModule::execute_script(browser, CARET_PROBE_SCRIPT);
        ScriptModule::execute_script(browser, SURROUNDING_TEXT_PROBE_SCRIPT);
    }
}

fn cache_path() -> String {
    let base = std::env::var("LOCALAPPDATA")
        .or_else(|_| std::env::var("HOME").map(|home| format!("{home}/.local/share")))
        .unwrap_or_else(|_| ".".to_string());
    let path = std::path::Path::new(&base)
        .join("HeadlessBrowser")
        .join("Cache");
    if let Err(error) = std::fs::create_dir_all(&path) {
        warn!("Failed to create cache directory: {error}");
    }
    path.to_string_lossy().into_owned()
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
pub fn configure_cef_api_version() {
    if CEF_API_VERSION_CONFIGURED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        let _ = cef::api_hash(cef::sys::CEF_API_VERSION_LAST, 0);
    }
}

#[cfg(not(feature = "cef"))]
pub fn configure_cef_api_version() {}

#[cfg(feature = "cef")]
pub fn shutdown_browser_runtime() {
    if CEF_RUNTIME_INITIALIZED.swap(false, Ordering::SeqCst) {
        cef::shutdown();
    }
}

#[cfg(not(feature = "cef"))]
pub fn shutdown_browser_runtime() {}
