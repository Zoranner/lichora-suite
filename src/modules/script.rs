//! Script Module - Handles JavaScript execution requests from Unity
//!
//! Protocol: 10005 bytes max
//! [flag(1), scriptLength(2), reserved(2), script(variable, UTF-8)]

use anyhow::Result;
use log::{debug, warn};

use super::base::MemoryModuleBase;
use super::protocol::ScriptRequest;
use crate::ipc::SharedMemoryWrapper;

pub const CARET_PROBE_SCRIPT: &str = r#"(function() {
    var el = document.activeElement;
    if (!el) return;
    var x = 0, y = 0, h = 0;
    function copyStyles(src, dst, names) {
        var cs = window.getComputedStyle(src);
        for (var i = 0; i < names.length; i++) dst.style[names[i]] = cs[names[i]];
        return cs;
    }
    function getTextControlCaretRect(input) {
        if (typeof input.selectionStart !== 'number') return null;
        var inputRect = input.getBoundingClientRect();
        var mirror = document.createElement('div');
        var computed = copyStyles(input, mirror, [
            'boxSizing','width','height','overflowX','overflowY',
            'borderTopWidth','borderRightWidth','borderBottomWidth','borderLeftWidth',
            'paddingTop','paddingRight','paddingBottom','paddingLeft',
            'fontStyle','fontVariant','fontWeight','fontStretch','fontSize','fontSizeAdjust',
            'lineHeight','fontFamily','textAlign','direction','textTransform','textIndent',
            'textDecoration','letterSpacing','wordSpacing','overflowWrap','wordBreak','tabSize','MozTabSize'
        ]);
        mirror.style.position = 'absolute';
        mirror.style.visibility = 'hidden';
        mirror.style.pointerEvents = 'none';
        mirror.style.left = (window.scrollX + inputRect.left) + 'px';
        mirror.style.top = (window.scrollY + inputRect.top) + 'px';
        mirror.style.whiteSpace = input.tagName === 'INPUT' ? 'pre' : 'pre-wrap';
        mirror.style.wordWrap = 'break-word';
        mirror.style.overflowWrap = 'break-word';
        mirror.style.wordBreak = computed.wordBreak || 'normal';
        mirror.style.borderColor = 'transparent';
        mirror.textContent = input.value.substring(0, input.selectionStart);
        var marker = document.createElement('span');
        marker.textContent = '\u200b';
        marker.style.display = 'inline-block';
        marker.style.width = '1px';
        marker.style.padding = '0';
        marker.style.margin = '0';
        mirror.appendChild(marker);
        document.body.appendChild(mirror);
        mirror.scrollTop = input.scrollTop;
        mirror.scrollLeft = input.scrollLeft;
        var markerRect = marker.getBoundingClientRect();
        document.body.removeChild(mirror);
        var lineHeight = parseFloat(computed.lineHeight);
        if (!isFinite(lineHeight)) lineHeight = parseFloat(computed.fontSize) || 16;
        return { left: markerRect.left, bottom: markerRect.top + lineHeight, height: lineHeight };
    }
    if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {
        var caretRect = getTextControlCaretRect(el);
        if (caretRect) {
            x = Math.round(caretRect.left);
            y = Math.round(caretRect.bottom);
            h = Math.max(1, Math.round(caretRect.height));
        }
    } else if (el.isContentEditable) {
        var sel = window.getSelection();
        if (sel && sel.rangeCount > 0) {
            var r = sel.getRangeAt(0).getBoundingClientRect();
            x = Math.round(r.left);
            y = Math.round(r.bottom);
            h = Math.max(1, Math.round(r.height || 16));
        }
    }
    if (x !== 0 || y !== 0) console.log('__CARET__:' + x + ',' + y + ',' + h);
})();"#;

pub const SURROUNDING_TEXT_PROBE_SCRIPT: &str = r#"(function() {
    var el = document.activeElement;
    var payload = { kind: 'none', text: '', selectionStart: 0, selectionEnd: 0 };
    if (el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA')) {
        var start = typeof el.selectionStart === 'number' ? el.selectionStart : 0;
        var end = typeof el.selectionEnd === 'number' ? el.selectionEnd : start;
        payload = {
            kind: 'text-control',
            text: String(el.value || ''),
            selectionStart: start,
            selectionEnd: end,
            tagName: String(el.tagName || ''),
            type: String(el.type || ''),
            inputMode: String(el.inputMode || el.getAttribute('inputmode') || '')
        };
    } else if (el && el.isContentEditable) {
        payload = { kind: 'contenteditable-unsupported', text: '', selectionStart: 0, selectionEnd: 0 };
    }
    console.log('__SURROUNDING_TEXT__:' + JSON.stringify(payload));
})();"#;

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

    #[cfg(feature = "cef")]
    pub fn execute_script(browser: &cef::Browser, script: &str) {
        use cef::{CefString, ImplBrowser, ImplFrame};

        if let Some(frame) = browser.main_frame() {
            let code = CefString::from(script);
            frame.execute_java_script(Some(&code), None, 0);
        }
    }

    /// Poll shared memory and execute any pending JavaScript via the browser's main frame.
    #[cfg(feature = "cef")]
    pub fn poll(&mut self, browser: &cef::Browser) {
        let Some(req) = self.read_request() else {
            return;
        };
        let script = String::from_utf8_lossy(&req.script);
        if script.is_empty() {
            warn!("Received empty script request");
            return;
        }
        debug!("Executing JS ({} bytes)", req.script_length);

        Self::execute_script(browser, script.as_ref());
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
