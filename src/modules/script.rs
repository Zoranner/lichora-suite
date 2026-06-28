//! JavaScript snippets used by the IPC v2 browser runtime.

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

pub struct ScriptModule;

impl ScriptModule {
    #[cfg(feature = "cef")]
    pub fn execute_script(browser: &cef::Browser, script: &str) {
        use cef::{CefString, ImplBrowser, ImplFrame};

        if let Some(frame) = browser.main_frame() {
            let code = CefString::from(script);
            frame.execute_java_script(Some(&code), None, 0);
        }
    }
}
