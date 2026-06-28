//! IPC v2 typed input decoding for browser interaction.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MouseEventType {
    LeftDown = 1,
    LeftUp = 2,
    RightDown = 3,
    RightUp = 4,
    MiddleDown = 5,
    MiddleUp = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IpcMouseButtonEvent {
    pub event_type: u8,
    pub x: i32,
    pub y: i32,
    pub buttons: u32,
    pub click_count: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IpcMouseWheelEvent {
    pub x: i32,
    pub y: i32,
    pub delta_x: i32,
    pub delta_y: i32,
    pub modifiers: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IpcMouseInputEvent {
    Button(IpcMouseButtonEvent),
    Wheel(IpcMouseWheelEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IpcKeyboardEvent {
    pub event_type: IpcKeyboardEventType,
    pub key_code: u32,
    pub native_key_code: u32,
    pub modifiers: u32,
    pub code_point: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IpcKeyboardEventType {
    KeyDown,
    KeyUp,
    Char,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IpcImeEvent {
    Composition {
        text: String,
        selection_start: i32,
        selection_end: i32,
    },
    Commit {
        text: String,
    },
    Cancel,
    DeleteSurroundingText {
        before: i32,
        after: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IpcInputEvent {
    Mouse(IpcMouseInputEvent),
    Keyboard(IpcKeyboardEvent),
    Ime(IpcImeEvent),
    Script(ipc::ScriptRequestInput),
}

pub(crate) fn decode_ipc_input_payload(payload: &[u8]) -> Option<IpcInputEvent> {
    match ipc::InputPayload::decode(payload).ok()? {
        ipc::InputPayload::MouseButton(input) => Some(IpcInputEvent::Mouse(
            IpcMouseInputEvent::Button(IpcMouseButtonEvent {
                event_type: mouse_button_event_type(input.button, input.pressed)?,
                x: input.x,
                y: input.y,
                buttons: input.buttons,
                click_count: i32::from(input.click_count.max(1)),
            }),
        )),
        ipc::InputPayload::MouseWheel(input) => Some(IpcInputEvent::Mouse(
            IpcMouseInputEvent::Wheel(IpcMouseWheelEvent {
                x: input.x,
                y: input.y,
                delta_x: input.delta_x,
                delta_y: input.delta_y,
                modifiers: input.modifiers,
            }),
        )),
        ipc::InputPayload::KeyboardKeyDown(input) => {
            Some(IpcInputEvent::Keyboard(IpcKeyboardEvent {
                event_type: IpcKeyboardEventType::KeyDown,
                key_code: input.key_code,
                native_key_code: input.native_key_code,
                modifiers: input.modifiers,
                code_point: 0,
            }))
        }
        ipc::InputPayload::KeyboardKeyUp(input) => {
            Some(IpcInputEvent::Keyboard(IpcKeyboardEvent {
                event_type: IpcKeyboardEventType::KeyUp,
                key_code: input.key_code,
                native_key_code: input.native_key_code,
                modifiers: input.modifiers,
                code_point: 0,
            }))
        }
        ipc::InputPayload::KeyboardChar {
            code_point,
            modifiers,
        } => Some(IpcInputEvent::Keyboard(IpcKeyboardEvent {
            event_type: IpcKeyboardEventType::Char,
            key_code: code_point,
            native_key_code: 0,
            modifiers,
            code_point,
        })),
        ipc::InputPayload::ImeComposition(input) => {
            Some(IpcInputEvent::Ime(IpcImeEvent::Composition {
                text: input.text,
                selection_start: input.selection_start,
                selection_end: input.selection_end,
            }))
        }
        ipc::InputPayload::ImeCommit { text } => {
            Some(IpcInputEvent::Ime(IpcImeEvent::Commit { text }))
        }
        ipc::InputPayload::ImeCancel => Some(IpcInputEvent::Ime(IpcImeEvent::Cancel)),
        ipc::InputPayload::ImeDeleteSurroundingText { before, after } => {
            Some(IpcInputEvent::Ime(IpcImeEvent::DeleteSurroundingText {
                before,
                after,
            }))
        }
        ipc::InputPayload::ScriptRequest(input) => Some(IpcInputEvent::Script(input)),
    }
}

fn mouse_button_event_type(button: u32, pressed: bool) -> Option<u8> {
    let event_type = match (button, pressed) {
        (1, true) => MouseEventType::LeftDown,
        (1, false) => MouseEventType::LeftUp,
        (2, true) => MouseEventType::RightDown,
        (2, false) => MouseEventType::RightUp,
        (3, true) => MouseEventType::MiddleDown,
        (3, false) => MouseEventType::MiddleUp,
        _ => return None,
    };
    Some(event_type as u8)
}

pub(crate) fn button_flags_to_event_flags(button_state: u32) -> u32 {
    const EVENTFLAG_LEFT_MOUSE_BUTTON: u32 = 16;
    const EVENTFLAG_MIDDLE_MOUSE_BUTTON: u32 = 32;
    const EVENTFLAG_RIGHT_MOUSE_BUTTON: u32 = 64;

    let mut modifiers: u32 = 0;
    if button_state & 0x01 != 0 {
        modifiers |= EVENTFLAG_LEFT_MOUSE_BUTTON;
    }
    if button_state & 0x02 != 0 {
        modifiers |= EVENTFLAG_RIGHT_MOUSE_BUTTON;
    }
    if button_state & 0x04 != 0 {
        modifiers |= EVENTFLAG_MIDDLE_MOUSE_BUTTON;
    }
    modifiers
}

#[cfg(test)]
mod tests {
    use super::{
        decode_ipc_input_payload, IpcInputEvent, IpcMouseButtonEvent, IpcMouseInputEvent,
        IpcMouseWheelEvent, MouseEventType,
    };
    use ipc::{
        InputPayload, KeyboardKeyInput, MouseButtonInput, MouseWheelInput, ScriptRequestInput,
    };

    #[test]
    fn decodes_ipc_v2_typed_mouse_button_payload() {
        let payload = InputPayload::MouseButton(MouseButtonInput {
            x: 10,
            y: 20,
            button: 1,
            buttons: 1,
            pressed: true,
            click_count: 2,
            modifiers: 0,
        })
        .encode();

        let event = decode_ipc_input_payload(&payload).unwrap();

        assert_eq!(
            IpcInputEvent::Mouse(IpcMouseInputEvent::Button(IpcMouseButtonEvent {
                event_type: MouseEventType::LeftDown as u8,
                x: 10,
                y: 20,
                buttons: 1,
                click_count: 2,
            })),
            event
        );
        assert!(decode_ipc_input_payload(&payload[..8]).is_none());
    }

    #[test]
    fn decodes_ipc_v2_typed_mouse_wheel_payload() {
        let payload = InputPayload::MouseWheel(MouseWheelInput {
            x: 10,
            y: 20,
            delta_x: -3,
            delta_y: 120,
            modifiers: 2,
        })
        .encode();

        let event = decode_ipc_input_payload(&payload).unwrap();

        assert_eq!(
            IpcInputEvent::Mouse(IpcMouseInputEvent::Wheel(IpcMouseWheelEvent {
                x: 10,
                y: 20,
                delta_x: -3,
                delta_y: 120,
                modifiers: 2,
            })),
            event
        );
        assert!(decode_ipc_input_payload(&payload[..16]).is_none());
    }

    #[test]
    fn decodes_ipc_v2_non_mouse_payloads_without_legacy_queue_kind() {
        assert!(decode_ipc_input_payload(
            &InputPayload::KeyboardKeyDown(KeyboardKeyInput {
                key_code: 65,
                native_key_code: 30,
                modifiers: 4,
            })
            .encode()
        )
        .is_some());
        assert!(decode_ipc_input_payload(
            &InputPayload::ScriptRequest(ScriptRequestInput {
                request_id: 7,
                script: "window.__probe = true".to_string(),
            })
            .encode()
        )
        .is_some());
    }
}
