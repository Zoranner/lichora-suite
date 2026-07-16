use ipc::{ControlCommand, ControlDecodeError};

const MAGIC: [u8; 4] = *b"EBCC";
const VERSION: [u8; 2] = [1, 0];

fn header(kind: u16) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&MAGIC);
    bytes.extend_from_slice(&VERSION);
    bytes.extend_from_slice(&kind.to_le_bytes());
    bytes
}

fn string_bytes(value: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
    bytes
}

#[test]
fn encodes_shutdown_as_golden_bytes() {
    assert_eq!(ControlCommand::Shutdown.encode(), header(1));
}

#[test]
fn encodes_add_browser_as_golden_bytes() {
    let command = ControlCommand::AddBrowser {
        browser_id: "browser-1".to_string(),
        width: 1280,
        height: 720,
        address: "https://example.test".to_string(),
    };

    let mut expected = header(2);
    expected.extend_from_slice(&string_bytes("browser-1"));
    expected.extend_from_slice(&1280i32.to_le_bytes());
    expected.extend_from_slice(&720i32.to_le_bytes());
    expected.extend_from_slice(&string_bytes("https://example.test"));

    assert_eq!(command.encode(), expected);
}

#[test]
fn decodes_resize_browser_from_golden_bytes() {
    let mut bytes = header(4);
    bytes.extend_from_slice(&string_bytes("browser-1"));
    bytes.extend_from_slice(&1024i32.to_le_bytes());
    bytes.extend_from_slice(&768i32.to_le_bytes());

    assert_eq!(
        ControlCommand::decode(&bytes).unwrap(),
        ControlCommand::ResizeBrowser {
            browser_id: "browser-1".to_string(),
            width: 1024,
            height: 768,
        }
    );
}

#[test]
fn roundtrips_all_command_variants() {
    let commands = [
        ControlCommand::Shutdown,
        ControlCommand::AddBrowser {
            browser_id: "browser-1".to_string(),
            width: 1280,
            height: 720,
            address: "https://example.test".to_string(),
        },
        ControlCommand::RemoveBrowser {
            browser_id: "browser-1".to_string(),
        },
        ControlCommand::ResizeBrowser {
            browser_id: "browser-1".to_string(),
            width: 1024,
            height: 768,
        },
    ];

    for command in commands {
        assert_eq!(ControlCommand::decode(&command.encode()).unwrap(), command);
    }
}

#[test]
fn rejects_unknown_kind() {
    assert_eq!(
        ControlCommand::decode(&header(99)).unwrap_err(),
        ControlDecodeError::UnknownKind(99)
    );
}

#[test]
fn rejects_short_buffer() {
    assert_eq!(
        ControlCommand::decode(&MAGIC).unwrap_err(),
        ControlDecodeError::BufferTooSmall {
            expected: 8,
            actual: 4,
        }
    );
}

#[test]
fn rejects_non_utf8_address() {
    let mut bytes = header(2);
    bytes.extend_from_slice(&string_bytes("browser-1"));
    bytes.extend_from_slice(&1280i32.to_le_bytes());
    bytes.extend_from_slice(&720i32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.push(0xff);

    assert!(matches!(
        ControlCommand::decode(&bytes),
        Err(ControlDecodeError::InvalidUtf8 { field: "address" })
    ));
}

#[test]
fn rejects_trailing_garbage() {
    let mut bytes = ControlCommand::Shutdown.encode();
    bytes.push(0);

    assert_eq!(
        ControlCommand::decode(&bytes).unwrap_err(),
        ControlDecodeError::TrailingGarbage {
            expected_end: 8,
            actual: 9,
        }
    );
}
