//! Browser Entry stub for builds without CEF.

use anyhow::Result;

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
            gpu_enabled: false,
        }
    }
}

/// Browser manager stub used when CEF is not compiled in.
pub struct BrowserEntry {
    config: BrowserConfig,
    initialized: bool,
    running: bool,
}

impl BrowserEntry {
    pub fn new() -> Self {
        Self::with_config(BrowserConfig::default())
    }

    pub fn with_config(config: BrowserConfig) -> Self {
        Self::with_session_config(config, String::new())
    }

    pub fn with_session_config(config: BrowserConfig, _session_id: String) -> Self {
        Self {
            config,
            initialized: false,
            running: false,
        }
    }

    pub fn initialize(&mut self) -> Result<()> {
        self.initialized = true;
        Ok(())
    }

    pub fn do_message_loop_work(&mut self) {}

    pub fn run_message_loop(&mut self) {
        self.running = self.initialized;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn shutdown(&mut self) {
        self.running = false;
        self.initialized = false;
    }

    pub fn load_url(&mut self, url: &str) {
        self.config.url = url.to_string();
    }

    pub fn reload(&mut self) {}

    pub fn execute_javascript(&mut self, _script: &str) {}

    pub fn show_devtools(&mut self) {}

    pub fn close_devtools(&mut self) {}

    pub fn set_size(&mut self, width: i32, height: i32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
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

pub fn configure_cef_api_version() {}

pub fn shutdown_browser_runtime() {}

#[cfg(test)]
mod tests {
    use super::{BrowserConfig, BrowserEntry};

    #[test]
    fn no_cef_browser_entry_preserves_public_state_changes() {
        let mut entry = BrowserEntry::with_config(BrowserConfig::default());

        assert!(!entry.is_initialized());
        entry.initialize().unwrap();
        assert!(entry.is_initialized());

        entry.load_url("https://example.test");
        entry.set_size(800, 600);
        entry.shutdown();

        assert!(!entry.is_initialized());
    }
}
