//! Script Module - Handles JavaScript execution requests from Unity
//!
//! Protocol: 10005 bytes max
//! [flag(1), scriptLength(2), reserved(2), script(variable, UTF-8)]

use anyhow::Result;
use log::{debug, warn};

use super::base::MemoryModuleBase;
use super::protocol::ScriptRequest;
use crate::ipc::SharedMemoryWrapper;

/// JavaScript execution module
pub struct ScriptModule {
    memory_name: String,
    shmem: SharedMemoryWrapper,
    running: bool,
}

impl ScriptModule {
    pub fn new(memory_name: &str) -> Result<Self> {
        let mut shmem = SharedMemoryWrapper::new(memory_name, ScriptRequest::SIZE);
        shmem.initialize()?;
        Ok(Self {
            memory_name: memory_name.to_string(),
            shmem,
            running: true,
        })
    }

    fn read_request(&mut self) -> Option<ScriptRequest> {
        let data = self.shmem.read_bytes().ok()?;
        if data.len() < 5 || data[0] != 1 {
            return None;
        }
        let req = ScriptRequest::from_bytes(&data)?;
        let _ = self.shmem.write_byte_at(0, 0);
        Some(req)
    }

    /// Poll shared memory and execute any pending JavaScript via the browser's main frame.
    #[cfg(feature = "cef")]
    pub fn poll(&mut self, browser: &cef::Browser) {
        use cef::{CefString, ImplBrowser, ImplFrame};

        let Some(req) = self.read_request() else {
            return;
        };
        let script = String::from_utf8_lossy(&req.script);
        if script.is_empty() {
            warn!("Received empty script request");
            return;
        }
        debug!("Executing JS ({} bytes)", req.script_length);

        if let Some(frame) = browser.main_frame() {
            let code = CefString::from(script.as_ref());
            frame.execute_java_script(Some(&code), None, 0);
        }
    }
}

impl MemoryModuleBase for ScriptModule {
    fn get_memory_name(&self) -> &str {
        &self.memory_name
    }
    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }
    fn shutdown(&mut self) {
        self.running = false;
    }
    fn is_running(&self) -> bool {
        self.running
    }
}
