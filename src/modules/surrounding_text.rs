//! Surrounding text probe payload parsing for IPC v2 output.

use serde::Deserialize;

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

#[cfg(test)]
mod tests {
    use super::{SurroundingTextContentType, SurroundingTextPayload, SurroundingTextSnapshot};

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
}
