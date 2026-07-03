use ipc::{
    CaretOutput, ImeCompositionInput, InputPayload, InputPayloadDecodeError, InputPayloadKind,
    KeyboardKeyInput, MouseButtonInput, MouseLatest, MouseWheelInput, OutputPayload,
    OutputPayloadKind, PageEventOutput, ScriptRequestInput, ScriptResultOutput,
    SurroundingTextOutput,
};

fn input_header(kind: InputPayloadKind) -> Vec<u8> {
    let mut bytes = Vec::from(*b"EBIP");
    bytes.extend_from_slice(&[1, 0]);
    bytes.extend_from_slice(&(kind as u16).to_le_bytes());
    bytes
}

fn output_header(kind: OutputPayloadKind) -> Vec<u8> {
    let mut bytes = Vec::from(*b"EBOP");
    bytes.extend_from_slice(&[1, 0]);
    bytes.extend_from_slice(&(kind as u16).to_le_bytes());
    bytes
}

fn string_bytes(value: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
    bytes
}

#[test]
fn mouse_latest_encodes_absolute_position_and_accumulated_delta() {
    let payload = MouseLatest {
        x: -10,
        y: 20,
        buttons: 3,
        delta_x: -7,
        delta_y: 9,
        valid: true,
    }
    .encode_payload();

    assert_eq!(payload.len(), 24);
    assert_eq!(&payload[0..4], &(-10i32).to_le_bytes());
    assert_eq!(&payload[4..8], &20i32.to_le_bytes());
    assert_eq!(&payload[8..12], &3u32.to_le_bytes());
    assert_eq!(&payload[12..16], &(-7i32).to_le_bytes());
    assert_eq!(&payload[16..20], &9i32.to_le_bytes());
    assert_eq!(payload[20], 1);
    assert_eq!(&payload[21..24], &[0, 0, 0]);
    assert_eq!(
        MouseLatest::decode_payload(&payload).unwrap(),
        MouseLatest {
            x: -10,
            y: 20,
            buttons: 3,
            delta_x: -7,
            delta_y: 9,
            valid: true,
        }
    );
}

#[test]
fn input_payload_encodes_mouse_button_and_wheel_golden_bytes() {
    let button = InputPayload::MouseButton(MouseButtonInput {
        x: 100,
        y: 200,
        button: 1,
        buttons: 3,
        pressed: true,
        click_count: 2,
        modifiers: 4,
    });
    let mut expected_button = input_header(InputPayloadKind::MouseButton);
    expected_button.extend_from_slice(&100i32.to_le_bytes());
    expected_button.extend_from_slice(&200i32.to_le_bytes());
    expected_button.extend_from_slice(&1u32.to_le_bytes());
    expected_button.extend_from_slice(&3u32.to_le_bytes());
    expected_button.extend_from_slice(&4u32.to_le_bytes());
    expected_button.extend_from_slice(&[1, 2, 0, 0]);

    assert_eq!(button.encode(), expected_button);
    assert_eq!(InputPayload::decode(&expected_button).unwrap(), button);

    let wheel = InputPayload::MouseWheel(MouseWheelInput {
        x: -1,
        y: 2,
        delta_x: -120,
        delta_y: 240,
        modifiers: 8,
    });
    let mut expected_wheel = input_header(InputPayloadKind::MouseWheel);
    expected_wheel.extend_from_slice(&(-1i32).to_le_bytes());
    expected_wheel.extend_from_slice(&2i32.to_le_bytes());
    expected_wheel.extend_from_slice(&(-120i32).to_le_bytes());
    expected_wheel.extend_from_slice(&240i32.to_le_bytes());
    expected_wheel.extend_from_slice(&8u32.to_le_bytes());

    assert_eq!(wheel.encode(), expected_wheel);
    assert_eq!(InputPayload::decode(&expected_wheel).unwrap(), wheel);
}

#[test]
fn input_payload_roundtrips_keyboard_ime_and_script_variants() {
    let payloads = [
        InputPayload::KeyboardKeyDown(KeyboardKeyInput {
            key_code: 65,
            native_key_code: 30,
            modifiers: 1,
        }),
        InputPayload::KeyboardKeyUp(KeyboardKeyInput {
            key_code: 65,
            native_key_code: 30,
            modifiers: 1,
        }),
        InputPayload::KeyboardChar {
            code_point: '好' as u32,
            modifiers: 0,
        },
        InputPayload::ImeComposition(ImeCompositionInput {
            text: "ni".to_string(),
            selection_start: 0,
            selection_end: 2,
        }),
        InputPayload::ImeCommit {
            text: "你".to_string(),
        },
        InputPayload::ImeCancel,
        InputPayload::ImeDeleteSurroundingText {
            before: 1,
            after: 2,
        },
        InputPayload::ScriptRequest(ScriptRequestInput {
            request_id: 42,
            script: "document.title".to_string(),
        }),
    ];

    for payload in payloads {
        assert_eq!(InputPayload::decode(&payload.encode()).unwrap(), payload);
    }
}

#[test]
fn input_payload_rejects_unknown_kind_bad_utf8_and_trailing_bytes() {
    let mut unknown = input_header(InputPayloadKind::MouseButton);
    unknown[6..8].copy_from_slice(&99u16.to_le_bytes());
    assert_eq!(
        InputPayload::decode(&unknown).unwrap_err(),
        InputPayloadDecodeError::UnknownKind(99)
    );

    let mut bad_utf8 = input_header(InputPayloadKind::ImeCommit);
    bad_utf8.extend_from_slice(&1u32.to_le_bytes());
    bad_utf8.push(0xff);
    assert!(matches!(
        InputPayload::decode(&bad_utf8),
        Err(InputPayloadDecodeError::InvalidUtf8 { field: "text" })
    ));

    let mut trailing = InputPayload::ImeCancel.encode();
    trailing.push(0);
    assert_eq!(
        InputPayload::decode(&trailing).unwrap_err(),
        InputPayloadDecodeError::TrailingGarbage {
            expected_end: 8,
            actual: 9
        }
    );
}

#[test]
fn output_payload_encodes_core_events_as_golden_bytes() {
    let caret = OutputPayload::Caret(CaretOutput {
        x: 10,
        y: 20,
        width: 1,
        height: 18,
        visible: true,
    });
    let mut expected_caret = output_header(OutputPayloadKind::Caret);
    expected_caret.extend_from_slice(&10i32.to_le_bytes());
    expected_caret.extend_from_slice(&20i32.to_le_bytes());
    expected_caret.extend_from_slice(&1i32.to_le_bytes());
    expected_caret.extend_from_slice(&18i32.to_le_bytes());
    expected_caret.extend_from_slice(&[1, 0, 0, 0]);

    assert_eq!(caret.encode(), expected_caret);
    assert_eq!(OutputPayload::decode(&expected_caret).unwrap(), caret);

    let page_event = OutputPayload::PageEvent(PageEventOutput {
        event_type: 2,
        url: "https://example.test".to_string(),
        detail: "loaded".to_string(),
    });
    let mut expected_event = output_header(OutputPayloadKind::PageEvent);
    expected_event.extend_from_slice(&2u32.to_le_bytes());
    expected_event.extend_from_slice(&string_bytes("https://example.test"));
    expected_event.extend_from_slice(&string_bytes("loaded"));

    assert_eq!(page_event.encode(), expected_event);
    assert_eq!(OutputPayload::decode(&expected_event).unwrap(), page_event);
}

#[test]
fn output_payload_roundtrips_surrounding_text_and_script_result() {
    let payloads = [
        OutputPayload::SurroundingText(SurroundingTextOutput {
            text: "hello".to_string(),
            selection_start: 1,
            selection_end: 3,
        }),
        OutputPayload::ScriptResult(ScriptResultOutput {
            request_id: 42,
            succeeded: false,
            value: "ReferenceError".to_string(),
        }),
    ];

    for payload in payloads {
        assert_eq!(OutputPayload::decode(&payload.encode()).unwrap(), payload);
    }
}
