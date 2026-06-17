//! Surrounding text snapshot module.

use std::fmt;

use anyhow::Result;
use log::{debug, warn};
use serde::Deserialize;

use super::base::MemoryModuleBase;
use crate::ipc::SharedMemoryWrapper;

pub const SURROUNDING_TEXT_STACK_SIZE: usize = 8192;
const TEXT_CONTROL_HEADER_SIZE: usize = 13;
const TEXT_CONTROL_V2_HEADER_SIZE: usize = 17;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SurroundingTextKind {
    None = 0,
    TextControl = 1,
    UnsupportedContentEditable = 2,
    TextControlV2 = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SurroundingTextContentType {
    Normal = 0,
    Password = 1,
    Number = 2,
    Phone = 3,
    Url = 4,
    Email = 5,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurroundingTextSnapshot {
    pub kind: SurroundingTextKind,
    pub text: String,
    pub cursor_byte_offset: usize,
    pub anchor_byte_offset: usize,
    pub content_type: SurroundingTextContentType,
}

impl SurroundingTextSnapshot {
    pub fn none() -> Self {
        Self {
            kind: SurroundingTextKind::None,
            text: String::new(),
            cursor_byte_offset: 0,
            anchor_byte_offset: 0,
            content_type: SurroundingTextContentType::Normal,
        }
    }

    pub fn unsupported_contenteditable() -> Self {
        Self {
            kind: SurroundingTextKind::UnsupportedContentEditable,
            text: String::new(),
            cursor_byte_offset: 0,
            anchor_byte_offset: 0,
            content_type: SurroundingTextContentType::Normal,
        }
    }

    pub fn text_control(
        text: String,
        selection_start: usize,
        selection_end: usize,
        content_type: SurroundingTextContentType,
    ) -> Self {
        Self {
            cursor_byte_offset: utf16_offset_to_utf8_byte_offset(&text, selection_start),
            anchor_byte_offset: utf16_offset_to_utf8_byte_offset(&text, selection_end),
            kind: SurroundingTextKind::TextControlV2,
            text,
            content_type,
        }
    }
}

pub struct SurroundingTextPayload;

impl SurroundingTextPayload {
    pub fn encode(snapshot: &SurroundingTextSnapshot) -> Vec<u8> {
        let mut buffer = vec![0u8; SURROUNDING_TEXT_STACK_SIZE];
        buffer[0] = snapshot.kind as u8;
        if snapshot.kind == SurroundingTextKind::None {
            return buffer;
        }

        let header_size = header_size(snapshot.kind);
        let max_text_bytes = buffer.len().saturating_sub(header_size);
        let text_byte_length = boundary_safe_text_length(&snapshot.text, max_text_bytes);
        let cursor = snapshot.cursor_byte_offset.min(text_byte_length);
        let anchor = snapshot.anchor_byte_offset.min(text_byte_length);
        buffer[1..5].copy_from_slice(&(cursor as i32).to_le_bytes());
        buffer[5..9].copy_from_slice(&(anchor as i32).to_le_bytes());
        buffer[9..13].copy_from_slice(&(text_byte_length as i32).to_le_bytes());
        if snapshot.kind == SurroundingTextKind::TextControlV2 {
            buffer[13] = snapshot.content_type as u8;
        }
        buffer[header_size..header_size + text_byte_length]
            .copy_from_slice(&snapshot.text.as_bytes()[..text_byte_length]);
        buffer
    }

    pub fn create_diagnostics(snapshot: &SurroundingTextSnapshot) -> SurroundingTextDiagnostics {
        let max_text_bytes = SURROUNDING_TEXT_STACK_SIZE.saturating_sub(header_size(snapshot.kind));
        let text_byte_length = if matches!(
            snapshot.kind,
            SurroundingTextKind::TextControl | SurroundingTextKind::TextControlV2
        ) {
            boundary_safe_text_length(&snapshot.text, max_text_bytes)
        } else {
            0
        };
        let cursor = snapshot.cursor_byte_offset.min(text_byte_length);
        let anchor = snapshot.anchor_byte_offset.min(text_byte_length);
        let native_ime_state = if snapshot.kind == SurroundingTextKind::UnsupportedContentEditable {
            "not-sent"
        } else {
            "sent"
        };

        SurroundingTextDiagnostics {
            kind: snapshot.kind,
            text_utf8_byte_length: text_byte_length,
            cursor_byte_offset: cursor,
            anchor_byte_offset: anchor,
            content_type: snapshot.content_type,
            native_ime_state,
        }
    }

    pub fn from_json(json: &str) -> Option<SurroundingTextSnapshot> {
        let probe: ProbeResult = serde_json::from_str(json).ok()?;
        match probe.kind.as_str() {
            "text-control" => Some(SurroundingTextSnapshot::text_control(
                probe.text,
                probe.selection_start.max(0) as usize,
                probe.selection_end.max(0) as usize,
                resolve_content_type(&probe.tag_name, &probe.type_, &probe.input_mode),
            )),
            "contenteditable-unsupported" => {
                Some(SurroundingTextSnapshot::unsupported_contenteditable())
            }
            _ => Some(SurroundingTextSnapshot::none()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurroundingTextDiagnostics {
    pub kind: SurroundingTextKind,
    pub text_utf8_byte_length: usize,
    pub cursor_byte_offset: usize,
    pub anchor_byte_offset: usize,
    pub content_type: SurroundingTextContentType,
    pub native_ime_state: &'static str,
}

impl fmt::Display for SurroundingTextDiagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "kind={:?}, textUtf8Bytes={}, cursor={}, anchor={}, contentType={:?}, nativeIme={}",
            self.kind,
            self.text_utf8_byte_length,
            self.cursor_byte_offset,
            self.anchor_byte_offset,
            self.content_type,
            self.native_ime_state
        )
    }
}

pub fn resolve_content_type(
    tag_name: &str,
    type_: &str,
    input_mode: &str,
) -> SurroundingTextContentType {
    let tag_name = normalize(tag_name);
    let type_ = normalize(type_);
    let input_mode = normalize(input_mode);

    if type_ == "password" {
        return SurroundingTextContentType::Password;
    }
    if type_ == "number" || matches!(input_mode.as_str(), "numeric" | "decimal") {
        return SurroundingTextContentType::Number;
    }
    if type_ == "tel" || input_mode == "tel" {
        return SurroundingTextContentType::Phone;
    }
    if type_ == "url" || input_mode == "url" {
        return SurroundingTextContentType::Url;
    }
    if type_ == "email" || input_mode == "email" {
        return SurroundingTextContentType::Email;
    }
    if matches!(tag_name.as_str(), "textarea" | "input") {
        return SurroundingTextContentType::Normal;
    }

    SurroundingTextContentType::Normal
}

pub fn utf16_offset_to_utf8_byte_offset(text: &str, utf16_offset: usize) -> usize {
    let mut utf16_seen = 0usize;
    let mut byte_offset = 0usize;
    for ch in text.chars() {
        let units = ch.len_utf16();
        if utf16_seen + units > utf16_offset {
            break;
        }
        utf16_seen += units;
        byte_offset += ch.len_utf8();
    }
    byte_offset
}

pub fn boundary_safe_text_length(text: &str, max_text_bytes: usize) -> usize {
    let mut byte_count = 0usize;
    for ch in text.chars() {
        let char_bytes = ch.len_utf8();
        if byte_count + char_bytes > max_text_bytes {
            return byte_count;
        }
        byte_count += char_bytes;
    }
    byte_count
}

fn header_size(kind: SurroundingTextKind) -> usize {
    if kind == SurroundingTextKind::TextControlV2 {
        TEXT_CONTROL_V2_HEADER_SIZE
    } else {
        TEXT_CONTROL_HEADER_SIZE
    }
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[derive(Debug, Deserialize)]
struct ProbeResult {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    text: String,
    #[serde(default, rename = "selectionStart")]
    selection_start: i32,
    #[serde(default, rename = "selectionEnd")]
    selection_end: i32,
    #[serde(default, rename = "tagName")]
    tag_name: String,
    #[serde(default, rename = "type")]
    type_: String,
    #[serde(default, rename = "inputMode")]
    input_mode: String,
}

pub struct SurroundingTextModule {
    memory_name: String,
    shmem: SharedMemoryWrapper,
    running: bool,
    last_payload: Vec<u8>,
}

impl SurroundingTextModule {
    pub fn new(memory_name: &str) -> Result<Self> {
        let mut shmem = SharedMemoryWrapper::new(memory_name, SURROUNDING_TEXT_STACK_SIZE);
        shmem.initialize()?;
        Ok(Self {
            memory_name: memory_name.to_string(),
            shmem,
            running: true,
            last_payload: Vec::new(),
        })
    }

    pub fn update_from_json(&mut self, json: &str) -> Result<()> {
        let Some(snapshot) = SurroundingTextPayload::from_json(json) else {
            warn!("SurroundingText parse ignored: invalid JSON");
            return Ok(());
        };
        self.update(snapshot)
    }

    pub fn update(&mut self, snapshot: SurroundingTextSnapshot) -> Result<()> {
        let payload = SurroundingTextPayload::encode(&snapshot);
        self.shmem.write_bytes(&payload)?;
        if payload != self.last_payload {
            let diagnostics = SurroundingTextPayload::create_diagnostics(&snapshot);
            debug!("SurroundingText snapshot changed: {diagnostics}");
            self.last_payload = payload;
        }
        Ok(())
    }
}

impl MemoryModuleBase for SurroundingTextModule {
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

#[cfg(test)]
mod tests {
    use super::{
        resolve_content_type, SurroundingTextContentType, SurroundingTextKind,
        SurroundingTextPayload, SurroundingTextSnapshot, SURROUNDING_TEXT_STACK_SIZE,
    };

    #[test]
    fn converts_utf16_selection_offsets_to_utf8_byte_offsets() {
        let snapshot = SurroundingTextSnapshot::text_control(
            "a中😀b".to_string(),
            4,
            5,
            SurroundingTextContentType::Normal,
        );
        assert_eq!(8, snapshot.cursor_byte_offset);
        assert_eq!(9, snapshot.anchor_byte_offset);
    }

    #[test]
    fn clamps_selection_offset_inside_surrogate_pair() {
        let snapshot = SurroundingTextSnapshot::text_control(
            "a😀b".to_string(),
            2,
            3,
            SurroundingTextContentType::Normal,
        );
        assert_eq!(1, snapshot.cursor_byte_offset);
        assert_eq!(5, snapshot.anchor_byte_offset);
    }

    #[test]
    fn clamps_malformed_selection_offsets_from_probe_json() {
        let snapshot = SurroundingTextPayload::from_json(
            r#"{"kind":"text-control","text":"中","selectionStart":-10,"selectionEnd":50,"tagName":"INPUT","type":"text","inputMode":""}"#,
        )
        .unwrap();
        assert_eq!(0, snapshot.cursor_byte_offset);
        assert_eq!(3, snapshot.anchor_byte_offset);
    }

    #[test]
    fn encodes_v1_text_control_payload() {
        let snapshot = SurroundingTextSnapshot {
            kind: SurroundingTextKind::TextControl,
            text: "a中".to_string(),
            cursor_byte_offset: 4,
            anchor_byte_offset: 4,
            content_type: SurroundingTextContentType::Normal,
        };
        let bytes = SurroundingTextPayload::encode(&snapshot);
        assert_eq!(SURROUNDING_TEXT_STACK_SIZE, bytes.len());
        assert_eq!(SurroundingTextKind::TextControl as u8, bytes[0]);
        assert_eq!(4, i32::from_le_bytes(bytes[1..5].try_into().unwrap()));
        assert_eq!(4, i32::from_le_bytes(bytes[5..9].try_into().unwrap()));
        assert_eq!(4, i32::from_le_bytes(bytes[9..13].try_into().unwrap()));
        assert_eq!("a中".as_bytes(), &bytes[13..17]);
    }

    #[test]
    fn encodes_v2_text_control_payload() {
        let snapshot = SurroundingTextSnapshot::text_control(
            "ab中".to_string(),
            3,
            1,
            SurroundingTextContentType::Email,
        );
        let bytes = SurroundingTextPayload::encode(&snapshot);
        assert_eq!(SURROUNDING_TEXT_STACK_SIZE, bytes.len());
        assert_eq!(SurroundingTextKind::TextControlV2 as u8, bytes[0]);
        assert_eq!(5, i32::from_le_bytes(bytes[1..5].try_into().unwrap()));
        assert_eq!(1, i32::from_le_bytes(bytes[5..9].try_into().unwrap()));
        assert_eq!(5, i32::from_le_bytes(bytes[9..13].try_into().unwrap()));
        assert_eq!(SurroundingTextContentType::Email as u8, bytes[13]);
        assert_eq!(0, bytes[14]);
        assert_eq!(0, bytes[15]);
        assert_eq!(0, bytes[16]);
        assert_eq!("ab中".as_bytes(), &bytes[17..22]);
    }

    #[test]
    fn diagnostics_omits_text_content() {
        let secret_text = "secret-不要出现在日志里";
        let snapshot = SurroundingTextSnapshot::text_control(
            secret_text.to_string(),
            secret_text.encode_utf16().count(),
            secret_text.encode_utf16().count(),
            SurroundingTextContentType::Email,
        );
        let diagnostics = SurroundingTextPayload::create_diagnostics(&snapshot);
        let log_text = diagnostics.to_string();

        assert_eq!(SurroundingTextKind::TextControlV2, diagnostics.kind);
        assert_eq!(secret_text.len(), diagnostics.text_utf8_byte_length);
        assert_eq!(SurroundingTextContentType::Email, diagnostics.content_type);
        assert_eq!("sent", diagnostics.native_ime_state);
        assert!(!log_text.contains(secret_text));
    }

    #[test]
    fn diagnostics_marks_unsupported_contenteditable_as_not_sent() {
        let snapshot = SurroundingTextSnapshot::unsupported_contenteditable();
        let diagnostics = SurroundingTextPayload::create_diagnostics(&snapshot);
        let log_text = diagnostics.to_string();

        assert_eq!(
            SurroundingTextKind::UnsupportedContentEditable,
            diagnostics.kind
        );
        assert_eq!(0, diagnostics.text_utf8_byte_length);
        assert_eq!(0, diagnostics.cursor_byte_offset);
        assert_eq!(0, diagnostics.anchor_byte_offset);
        assert_eq!(SurroundingTextContentType::Normal, diagnostics.content_type);
        assert_eq!("not-sent", diagnostics.native_ime_state);
        assert!(log_text.contains("nativeIme=not-sent"));
    }

    #[test]
    fn truncates_payload_to_fixed_buffer() {
        let text = "a".repeat(SURROUNDING_TEXT_STACK_SIZE);
        let snapshot = SurroundingTextSnapshot::text_control(
            text,
            SURROUNDING_TEXT_STACK_SIZE,
            SURROUNDING_TEXT_STACK_SIZE,
            SurroundingTextContentType::Normal,
        );
        let bytes = SurroundingTextPayload::encode(&snapshot);
        let expected_text_length = SURROUNDING_TEXT_STACK_SIZE - 17;
        assert_eq!(SURROUNDING_TEXT_STACK_SIZE, bytes.len());
        assert_eq!(
            expected_text_length as i32,
            i32::from_le_bytes(bytes[9..13].try_into().unwrap())
        );
        assert_eq!(
            expected_text_length as i32,
            i32::from_le_bytes(bytes[1..5].try_into().unwrap())
        );
        assert_eq!(
            expected_text_length as i32,
            i32::from_le_bytes(bytes[5..9].try_into().unwrap())
        );
    }

    #[test]
    fn truncates_payload_on_utf8_character_boundary() {
        let text = "a".repeat(SURROUNDING_TEXT_STACK_SIZE - 18) + "中";
        let snapshot = SurroundingTextSnapshot::text_control(
            text,
            SURROUNDING_TEXT_STACK_SIZE,
            SURROUNDING_TEXT_STACK_SIZE,
            SurroundingTextContentType::Normal,
        );
        let bytes = SurroundingTextPayload::encode(&snapshot);
        assert_eq!(
            (SURROUNDING_TEXT_STACK_SIZE - 18) as i32,
            i32::from_le_bytes(bytes[9..13].try_into().unwrap())
        );
        assert_eq!(
            (SURROUNDING_TEXT_STACK_SIZE - 18) as i32,
            i32::from_le_bytes(bytes[1..5].try_into().unwrap())
        );
        assert_eq!(
            (SURROUNDING_TEXT_STACK_SIZE - 18) as i32,
            i32::from_le_bytes(bytes[5..9].try_into().unwrap())
        );
        assert_eq!(0, bytes[SURROUNDING_TEXT_STACK_SIZE - 1]);
    }

    #[test]
    fn maps_dom_content_type() {
        assert_eq!(
            SurroundingTextContentType::Password,
            resolve_content_type("input", "password", "")
        );
        assert_eq!(
            SurroundingTextContentType::Number,
            resolve_content_type("input", "text", "decimal")
        );
        assert_eq!(
            SurroundingTextContentType::Phone,
            resolve_content_type("input", "tel", "")
        );
        assert_eq!(
            SurroundingTextContentType::Url,
            resolve_content_type("input", "text", "url")
        );
        assert_eq!(
            SurroundingTextContentType::Email,
            resolve_content_type("input", "email", "")
        );
    }

    #[test]
    fn parses_probe_json_to_text_control_snapshot() {
        let snapshot = SurroundingTextPayload::from_json(
            r#"{"kind":"text-control","text":"a中","selectionStart":2,"selectionEnd":1,"tagName":"INPUT","type":"email","inputMode":""}"#,
        )
        .unwrap();
        assert_eq!(SurroundingTextKind::TextControlV2, snapshot.kind);
        assert_eq!(4, snapshot.cursor_byte_offset);
        assert_eq!(1, snapshot.anchor_byte_offset);
        assert_eq!(SurroundingTextContentType::Email, snapshot.content_type);
    }
}
