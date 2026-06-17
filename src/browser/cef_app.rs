//! CEF Application and Client implementations (Stub for non-CEF builds)
//!
//! This module provides the CEF App, Client, and handlers needed for off-screen rendering

#[cfg(feature = "cef")]
mod cef_impl {
    use crate::browser::render::OsrRenderHandler;
    use crate::modules::{parse_caret_console_payload, CaretModule, SurroundingTextModule};
    use cef::*;
    use log::info;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};

    /// Headless Browser Application
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

                // Enable off-screen rendering
                command_line.append_switch(Some(&"no-startup-window".into()));
                command_line.append_switch(Some(&"noerrdialogs".into()));
                command_line.append_switch(Some(&"hide-crash-restore-bubble".into()));
                command_line.append_switch(Some(&"use-mock-keychain".into()));
                command_line.append_switch(Some(&"enable-logging=stderr".into()));

                append_graphics_mode_switches(command_line, self.app.gpu_enabled);

                // Enable remote debugging
                command_line.append_switch_with_value(
                    Some(&"remote-debugging-port".into()),
                    Some(&"9229".into()),
                );
                command_line.append_switch(Some(&"enable-media-stream".into()));
                command_line.append_switch(Some(&"use-fake-ui-for-media-stream".into()));

                // Security settings
                command_line.append_switch(Some(&"disable-web-security".into()));
                command_line.append_switch(Some(&"allow-running-insecure-content".into()));
                command_line.append_switch(Some(&"ignore-certificate-errors".into()));

                // Stability settings
                command_line.append_switch(Some(&"disable-session-crashed-bubble".into()));
                command_line.append_switch(Some(&"disable-hang-monitor".into()));

                append_linux_switches(command_line);
            }

            fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
                Some(BrowserProcessHandlerBuilder::build(
                    HeadlessBrowserProcessHandler::new(
                        self.app.is_cef_ready.clone(),
                        self.app.gpu_enabled,
                    ),
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
    pub struct HeadlessBrowserProcessHandler {
        is_cef_ready: Rc<RefCell<bool>>,
        gpu_enabled: bool,
    }

    impl HeadlessBrowserProcessHandler {
        pub fn new(is_cef_ready: Rc<RefCell<bool>>, gpu_enabled: bool) -> Self {
            Self {
                is_cef_ready,
                gpu_enabled,
            }
        }
    }

    wrap_browser_process_handler! {
        pub struct BrowserProcessHandlerBuilder {
            handler: HeadlessBrowserProcessHandler,
        }

        impl BrowserProcessHandler {
            fn on_context_initialized(&self) {
                info!("CEF context initialized");
                *self.handler.is_cef_ready.borrow_mut() = true;
            }

            fn on_before_child_process_launch(&self, command_line: Option<&mut CommandLine>) {
                let Some(command_line) = command_line else {
                    return;
                };

                command_line.append_switch(Some(&"disable-web-security".into()));
                command_line.append_switch(Some(&"allow-running-insecure-content".into()));
                command_line.append_switch(Some(&"disable-session-crashed-bubble".into()));
                command_line.append_switch(Some(&"ignore-certificate-errors".into()));
                command_line.append_switch(Some(&"enable-logging=stderr".into()));
                command_line.append_switch(Some(&"enable-media-stream".into()));
                command_line.append_switch(Some(&"use-fake-ui-for-media-stream".into()));
                append_graphics_mode_switches(command_line, self.handler.gpu_enabled);
                append_linux_switches(command_line);
            }
        }
    }

    impl BrowserProcessHandlerBuilder {
        pub fn build(handler: HeadlessBrowserProcessHandler) -> BrowserProcessHandler {
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
    pub struct HeadlessDisplayHandler {
        caret_module: Arc<Mutex<CaretModule>>,
        surrounding_text_module: Arc<Mutex<SurroundingTextModule>>,
    }

    impl HeadlessDisplayHandler {
        pub fn new(
            caret_module: Arc<Mutex<CaretModule>>,
            surrounding_text_module: Arc<Mutex<SurroundingTextModule>>,
        ) -> Self {
            Self {
                caret_module,
                surrounding_text_module,
            }
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

                if let Some(payload) = message.strip_prefix("__CARET__:") {
                    if let Some((x, y, height)) = parse_caret_console_payload(payload) {
                        if let Ok(caret) = self.handler.caret_module.lock() {
                            let _ = caret.write_position_with_height(x, y, height);
                        }
                    }
                    return true as _;
                }

                if let Some(payload) = message.strip_prefix("__SURROUNDING_TEXT__:") {
                    if let Ok(mut surrounding_text) = self.handler.surrounding_text_module.lock() {
                        let _ = surrounding_text.update_from_json(payload);
                    }
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
        }

        impl Client {
            fn display_handler(&self) -> Option<DisplayHandler> {
                Some(self.display_handler.clone())
            }

            fn render_handler(&self) -> Option<RenderHandler> {
                Some(self.render_handler.clone())
            }
        }
    }

    impl ClientBuilder {
        pub fn build(
            render_handler: OsrRenderHandler,
            caret_module: Arc<Mutex<CaretModule>>,
            surrounding_text_module: Arc<Mutex<SurroundingTextModule>>,
        ) -> Client {
            use crate::browser::render::RenderHandlerBuilder;
            let cef_render_handler = RenderHandlerBuilder::build(render_handler.clone());
            let display_handler = DisplayHandlerBuilder::build(HeadlessDisplayHandler::new(
                caret_module,
                surrounding_text_module,
            ));
            Self::new(
                HeadlessClient::new(render_handler),
                cef_render_handler,
                display_handler,
            )
        }
    }
}

#[cfg(feature = "cef")]
pub use cef_impl::*;
