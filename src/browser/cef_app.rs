//! CEF Application and Client implementations (Stub for non-CEF builds)
//!
//! This module provides the CEF App, Client, and handlers needed for off-screen rendering

#[cfg(feature = "cef")]
mod cef_impl {
    use crate::browser::render::OsrRenderHandler;
    use cef::*;
    use log::info;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Headless Browser Application
    #[derive(Clone)]
    pub struct HeadlessApp {
        is_cef_ready: Rc<RefCell<bool>>,
    }

    impl Default for HeadlessApp {
        fn default() -> Self {
            Self::new()
        }
    }

    impl HeadlessApp {
        pub fn new() -> Self {
            Self {
                is_cef_ready: Rc::new(RefCell::new(false)),
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

                // Disable GPU for headless (can be enabled later with proper setup)
                command_line.append_switch(Some(&"disable-gpu".into()));
                command_line.append_switch(Some(&"disable-gpu-compositing".into()));

                // Enable remote debugging
                command_line.append_switch_with_value(
                    Some(&"remote-debugging-port".into()),
                    Some(&"9229".into()),
                );

                // Security settings
                command_line.append_switch(Some(&"disable-web-security".into()));
                command_line.append_switch(Some(&"allow-running-insecure-content".into()));
                command_line.append_switch(Some(&"ignore-certificate-errors".into()));

                // Stability settings
                command_line.append_switch(Some(&"disable-session-crashed-bubble".into()));
                command_line.append_switch(Some(&"disable-hang-monitor".into()));
            }

            fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
                Some(BrowserProcessHandlerBuilder::build(
                    HeadlessBrowserProcessHandler::new(self.app.is_cef_ready.clone()),
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
    }

    impl HeadlessBrowserProcessHandler {
        pub fn new(is_cef_ready: Rc<RefCell<bool>>) -> Self {
            Self { is_cef_ready }
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
            }
        }
    }

    impl BrowserProcessHandlerBuilder {
        pub fn build(handler: HeadlessBrowserProcessHandler) -> BrowserProcessHandler {
            Self::new(handler)
        }
    }

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

    wrap_client! {
        pub struct ClientBuilder {
            client: HeadlessClient,
            render_handler: RenderHandler,
        }

        impl Client {
            fn render_handler(&self) -> Option<RenderHandler> {
                Some(self.render_handler.clone())
            }
        }
    }

    impl ClientBuilder {
        pub fn build(render_handler: OsrRenderHandler) -> Client {
            use crate::browser::render::RenderHandlerBuilder;
            let cef_render_handler = RenderHandlerBuilder::build(render_handler.clone());
            Self::new(HeadlessClient::new(render_handler), cef_render_handler)
        }
    }
}

#[cfg(feature = "cef")]
pub use cef_impl::*;
