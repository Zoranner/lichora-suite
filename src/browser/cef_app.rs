//! CEF Application and Client implementations (Stub for non-CEF builds)
//!
//! This module provides the CEF App, Client, and handlers needed for off-screen rendering

#[cfg(feature = "cef")]
mod cef_impl {
    use crate::browser::dom_bridge::handle_dom_bridge_message;
    use crate::browser::output::BrowserOutputIpcChannels;
    use crate::browser::render::OsrRenderHandler;
    use cef::*;
    use log::info;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };

    /// Lichora browser application
    #[derive(Clone)]
    pub struct HeadlessApp {
        is_cef_ready: Rc<RefCell<bool>>,
        gpu_enabled: bool,
    }

    impl Default for HeadlessApp {
        fn default() -> Self {
            Self::new(false)
        }
    }

    impl HeadlessApp {
        pub fn new(gpu_enabled: bool) -> Self {
            Self {
                is_cef_ready: Rc::new(RefCell::new(false)),
                gpu_enabled,
            }
        }

        pub fn is_cef_ready(&self) -> bool {
            *self.is_cef_ready.borrow()
        }
    }

    wrap_app! {
        pub struct AppBuilder {
            app: HeadlessApp,
        }

        impl App {
            fn on_before_command_line_processing(
                &self,
                _process_type: Option<&CefStringUtf16>,
                command_line: Option<&mut CommandLine>,
            ) {
                let Some(command_line) = command_line else {
                    return;
                };

                if _process_type.and_then(CefString::as_slice).is_some() {
                    return;
                }

                append_graphics_mode_switches(command_line, self.app.gpu_enabled);
                command_line.append_switch(Some(&"enable-media-stream".into()));
                command_line.append_switch(Some(&"use-fake-ui-for-media-stream".into()));
                append_linux_switches(command_line);
            }

            fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
                Some(BrowserProcessHandlerBuilder::build(
                    LichoraProcessHandler::new(self.app.is_cef_ready.clone()),
                ))
            }
        }
    }

    impl AppBuilder {
        pub fn build(app: HeadlessApp) -> App {
            Self::new(app)
        }
    }

    /// Browser Process Handler
    #[derive(Clone)]
    pub struct LichoraProcessHandler {
        is_cef_ready: Rc<RefCell<bool>>,
    }

    impl LichoraProcessHandler {
        pub fn new(is_cef_ready: Rc<RefCell<bool>>) -> Self {
            Self { is_cef_ready }
        }
    }

    wrap_browser_process_handler! {
        pub struct BrowserProcessHandlerBuilder {
            handler: LichoraProcessHandler,
        }

        impl BrowserProcessHandler {
            fn on_context_initialized(&self) {
                info!("CEF context initialized");
                *self.handler.is_cef_ready.borrow_mut() = true;
            }

            fn on_before_child_process_launch(&self, _command_line: Option<&mut CommandLine>) {}
        }
    }

    impl BrowserProcessHandlerBuilder {
        pub fn build(handler: LichoraProcessHandler) -> BrowserProcessHandler {
            Self::new(handler)
        }
    }

    fn append_graphics_mode_switches(command_line: &mut CommandLine, gpu_enabled: bool) {
        if !gpu_enabled {
            command_line.append_switch(Some(&"disable-gpu".into()));
            command_line.append_switch(Some(&"disable-software-rasterizer".into()));
            command_line.append_switch(Some(&"disable-gpu-compositing".into()));
        }
    }

    #[cfg(target_os = "linux")]
    fn append_linux_switches(command_line: &mut CommandLine) {
        command_line.append_switch(Some(&"no-sandbox".into()));
        command_line.append_switch(Some(&"no-zygote".into()));
        command_line.append_switch(Some(&"disable-setuid-sandbox".into()));
    }

    #[cfg(not(target_os = "linux"))]
    fn append_linux_switches(_command_line: &mut CommandLine) {}

    /// Browser Client - handles browser events
    #[derive(Clone)]
    pub struct HeadlessClient {
        render_handler: OsrRenderHandler,
    }

    impl HeadlessClient {
        pub fn new(render_handler: OsrRenderHandler) -> Self {
            Self { render_handler }
        }

        pub fn render_handler(&self) -> &OsrRenderHandler {
            &self.render_handler
        }
    }

    #[derive(Clone)]
    pub struct HeadlessLifeSpanHandler {
        closed: Arc<AtomicBool>,
        browser_slot: Arc<Mutex<Option<Browser>>>,
    }

    impl HeadlessLifeSpanHandler {
        pub fn new(closed: Arc<AtomicBool>, browser_slot: Arc<Mutex<Option<Browser>>>) -> Self {
            Self {
                closed,
                browser_slot,
            }
        }
    }

    wrap_life_span_handler! {
        pub struct LifeSpanHandlerBuilder {
            handler: HeadlessLifeSpanHandler,
        }

        impl LifeSpanHandler {
            fn on_after_created(&self, browser: Option<&mut Browser>) {
                if let Some(browser) = browser {
                    if let Ok(mut slot) = self.handler.browser_slot.lock() {
                        *slot = Some(browser.clone());
                    }
                    self.handler.closed.store(false, Ordering::SeqCst);
                    info!("CEF browser after created");
                }
            }

            fn on_before_popup(
                &self,
                browser: Option<&mut Browser>,
                _frame: Option<&mut Frame>,
                _popup_id: ::std::os::raw::c_int,
                target_url: Option<&CefString>,
                _target_frame_name: Option<&CefString>,
                _target_disposition: WindowOpenDisposition,
                _user_gesture: ::std::os::raw::c_int,
                _popup_features: Option<&PopupFeatures>,
                _window_info: Option<&mut WindowInfo>,
                _client: Option<&mut Option<Client>>,
                _settings: Option<&mut BrowserSettings>,
                _extra_info: Option<&mut Option<DictionaryValue>>,
                _no_javascript_access: Option<&mut ::std::os::raw::c_int>,
            ) -> ::std::os::raw::c_int {
                if let (Some(browser), Some(target_url)) = (browser, target_url) {
                    if let Some(frame) = browser.main_frame() {
                        frame.load_url(Some(target_url));
                    }
                }
                true as _
            }

            fn on_before_close(&self, _browser: Option<&mut Browser>) {
                self.handler.closed.store(true, Ordering::SeqCst);
                info!("CEF browser before close");
            }
        }
    }

    impl LifeSpanHandlerBuilder {
        pub fn build(handler: HeadlessLifeSpanHandler) -> LifeSpanHandler {
            Self::new(handler)
        }
    }

    #[derive(Clone)]
    pub struct HeadlessLoadHandler {
        page_loaded: Arc<AtomicBool>,
        loading: Arc<AtomicBool>,
    }

    impl HeadlessLoadHandler {
        pub fn new(page_loaded: Arc<AtomicBool>, loading: Arc<AtomicBool>) -> Self {
            Self {
                page_loaded,
                loading,
            }
        }
    }

    wrap_load_handler! {
        pub struct LoadHandlerBuilder {
            handler: HeadlessLoadHandler,
        }

        impl LoadHandler {
            fn on_loading_state_change(
                &self,
                browser: Option<&mut Browser>,
                is_loading: ::std::os::raw::c_int,
                _can_go_back: ::std::os::raw::c_int,
                _can_go_forward: ::std::os::raw::c_int,
            ) {
                self.handler
                    .loading
                    .store(is_loading != 0, Ordering::SeqCst);
                if is_loading != 0 {
                    return;
                }

                self.handler.page_loaded.store(true, Ordering::SeqCst);
                let Some(browser) = browser else {
                    return;
                };
                if let Some(host) = browser.host() {
                    reveal_and_focus_for_linux(&host);
                    host.invalidate(PaintElementType::VIEW);
                }
            }
        }
    }

    impl LoadHandlerBuilder {
        pub fn build(handler: HeadlessLoadHandler) -> LoadHandler {
            Self::new(handler)
        }
    }

    #[derive(Clone)]
    pub struct HeadlessDisplayHandler {
        output_ipc: Arc<Mutex<BrowserOutputIpcChannels>>,
    }

    impl HeadlessDisplayHandler {
        pub(crate) fn new(output_ipc: Arc<Mutex<BrowserOutputIpcChannels>>) -> Self {
            Self { output_ipc }
        }
    }

    wrap_display_handler! {
        pub struct DisplayHandlerBuilder {
            handler: HeadlessDisplayHandler,
        }

        impl DisplayHandler {
            fn on_console_message(
                &self,
                _browser: Option<&mut Browser>,
                _level: LogSeverity,
                message: Option<&CefStringUtf16>,
                _source: Option<&CefStringUtf16>,
                _line: ::std::os::raw::c_int,
            ) -> ::std::os::raw::c_int {
                let Some(message) = message else {
                    return false as _;
                };
                let message = message.to_string();

                if handle_dom_bridge_message(&message, |payload| {
                        if let Ok(mut output) = self.handler.output_ipc.lock() {
                            let _ = output.publish(payload);
                        }
                    }) {
                    return true as _;
                }

                false as _
            }
        }
    }

    impl DisplayHandlerBuilder {
        pub fn build(handler: HeadlessDisplayHandler) -> DisplayHandler {
            Self::new(handler)
        }
    }

    wrap_client! {
        pub struct ClientBuilder {
            client: HeadlessClient,
            render_handler: RenderHandler,
            display_handler: DisplayHandler,
            life_span_handler: LifeSpanHandler,
            load_handler: LoadHandler,
        }

        impl Client {
            fn display_handler(&self) -> Option<DisplayHandler> {
                Some(self.display_handler.clone())
            }

            fn render_handler(&self) -> Option<RenderHandler> {
                Some(self.render_handler.clone())
            }

            fn life_span_handler(&self) -> Option<LifeSpanHandler> {
                Some(self.life_span_handler.clone())
            }

            fn load_handler(&self) -> Option<LoadHandler> {
                Some(self.load_handler.clone())
            }
        }
    }

    impl ClientBuilder {
        pub(crate) fn build(
            render_handler: OsrRenderHandler,
            output_ipc: Arc<Mutex<BrowserOutputIpcChannels>>,
            closed: Arc<AtomicBool>,
            browser_slot: Arc<Mutex<Option<Browser>>>,
            page_loaded: Arc<AtomicBool>,
            loading: Arc<AtomicBool>,
        ) -> Client {
            use crate::browser::render::RenderHandlerBuilder;
            let cef_render_handler = RenderHandlerBuilder::build(render_handler.clone());
            let display_handler =
                DisplayHandlerBuilder::build(HeadlessDisplayHandler::new(output_ipc));
            let life_span_handler =
                LifeSpanHandlerBuilder::build(HeadlessLifeSpanHandler::new(closed, browser_slot));
            let load_handler =
                LoadHandlerBuilder::build(HeadlessLoadHandler::new(page_loaded, loading));
            Self::new(
                HeadlessClient::new(render_handler),
                cef_render_handler,
                display_handler,
                life_span_handler,
                load_handler,
            )
        }
    }

    pub fn reveal_and_focus_for_linux(host: &BrowserHost) {
        reveal_and_focus_for_linux_impl(host);
    }

    #[cfg(target_os = "linux")]
    fn reveal_and_focus_for_linux_impl(host: &BrowserHost) {
        host.was_hidden(0);
        host.set_focus(1);
    }

    #[cfg(not(target_os = "linux"))]
    fn reveal_and_focus_for_linux_impl(_host: &BrowserHost) {}
}

#[cfg(feature = "cef")]
pub use cef_impl::*;
