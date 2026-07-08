use crate::modules::{
    input_ownership_map_from_legacy_pass_map, parse_caret_console_payload,
    parse_input_ownership_map_console_payload, parse_overlay_pass_map_console_payload,
    SurroundingTextPayload, INPUT_OWNERSHIP_MAP_CONSOLE_PREFIX, OVERLAY_PASS_MAP_CONSOLE_PREFIX,
};

const CARET_CONSOLE_PREFIX: &str = "__CARET__:";
const SURROUNDING_TEXT_CONSOLE_PREFIX: &str = "__SURROUNDING_TEXT__:";
const CARET_BRIDGE_TYPE: &str = "caret";
const INPUT_OWNERSHIP_MAP_BRIDGE_TYPE: &str = "inputOwnershipMap";
const OVERLAY_PASS_MAP_BRIDGE_TYPE: &str = "overlayPassMap";
const SURROUNDING_TEXT_BRIDGE_TYPE: &str = "surroundingText";

pub(crate) fn handle_dom_bridge_message(
    message: &str,
    publish: impl FnMut(ipc::OutputPayload),
) -> bool {
    if let Some(payload) = message.strip_prefix(CARET_CONSOLE_PREFIX) {
        return handle_dom_bridge_payload(CARET_BRIDGE_TYPE, payload, publish);
    }

    if let Some(payload) = message.strip_prefix(SURROUNDING_TEXT_CONSOLE_PREFIX) {
        return handle_dom_bridge_payload(SURROUNDING_TEXT_BRIDGE_TYPE, payload, publish);
    }

    if let Some(payload) = message.strip_prefix(INPUT_OWNERSHIP_MAP_CONSOLE_PREFIX) {
        return handle_dom_bridge_payload(INPUT_OWNERSHIP_MAP_BRIDGE_TYPE, payload, publish);
    }

    if let Some(payload) = message.strip_prefix(OVERLAY_PASS_MAP_CONSOLE_PREFIX) {
        return handle_dom_bridge_payload(OVERLAY_PASS_MAP_BRIDGE_TYPE, payload, publish);
    }

    false
}

pub(crate) fn handle_dom_bridge_payload(
    bridge_type: &str,
    payload: &str,
    mut publish: impl FnMut(ipc::OutputPayload),
) -> bool {
    match bridge_type {
        CARET_BRIDGE_TYPE => {
            if let Some((x, y, height)) = parse_caret_console_payload(payload) {
                publish(ipc::OutputPayload::Caret(ipc::CaretOutput {
                    x: i32::from(x),
                    y: i32::from(y),
                    width: 0,
                    height: i32::from(height),
                    visible: true,
                }));
            }
            true
        }
        SURROUNDING_TEXT_BRIDGE_TYPE => {
            if let Some(snapshot) = SurroundingTextPayload::from_json(payload) {
                publish(ipc::OutputPayload::SurroundingText(
                    ipc::SurroundingTextOutput {
                        text: snapshot.text,
                        selection_start: snapshot.cursor_byte_offset as i32,
                        selection_end: snapshot.anchor_byte_offset as i32,
                    },
                ));
            }
            true
        }
        INPUT_OWNERSHIP_MAP_BRIDGE_TYPE => {
            if let Ok(ownership_map) = parse_input_ownership_map_console_payload(payload) {
                publish(ipc::OutputPayload::InputOwnershipMap(ownership_map));
            }
            true
        }
        OVERLAY_PASS_MAP_BRIDGE_TYPE => {
            if let Ok(pass_map) = parse_overlay_pass_map_console_payload(payload) {
                publish(ipc::OutputPayload::InputOwnershipMap(
                    input_ownership_map_from_legacy_pass_map(pass_map),
                ));
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{handle_dom_bridge_message, handle_dom_bridge_payload};
    use crate::modules::{INPUT_OWNERSHIP_MAP_CONSOLE_PREFIX, OVERLAY_PASS_MAP_CONSOLE_PREFIX};

    #[test]
    fn ignores_unrecognized_messages() {
        let mut published = Vec::new();

        assert!(!handle_dom_bridge_message("ordinary log", |payload| {
            published.push(payload);
        }));
        assert!(published.is_empty());
    }

    #[test]
    fn consumes_invalid_known_messages_without_publishing() {
        let mut published = Vec::new();

        assert!(handle_dom_bridge_message(
            "__LICHORA_INPUT_OWNERSHIP_MAP__:{invalid",
            |payload| {
                published.push(payload);
            },
        ));
        assert!(published.is_empty());
    }

    #[test]
    fn maps_caret_and_surrounding_text_messages_to_output_payloads() {
        let mut published = Vec::new();

        assert!(handle_dom_bridge_message("__CARET__:10,20,30", |payload| {
            published.push(payload);
        }));
        assert!(handle_dom_bridge_message(
            r#"__SURROUNDING_TEXT__:{"kind":"text-control","text":"a中","selectionStart":2,"selectionEnd":2,"tagName":"input","type":"text","inputMode":""}"#,
            |payload| {
                published.push(payload);
            },
        ));

        assert!(matches!(published[0], ipc::OutputPayload::Caret(_)));
        match &published[1] {
            ipc::OutputPayload::SurroundingText(output) => {
                assert_eq!("a中", output.text);
                assert_eq!(4, output.selection_start);
                assert_eq!(4, output.selection_end);
            }
            other => panic!("unexpected payload: {other:?}"),
        }
    }

    #[test]
    fn maps_new_and_legacy_ownership_messages_to_input_ownership_payloads() {
        let mut published = Vec::new();
        let ownership_payload = format!(
            r#"{INPUT_OWNERSHIP_MAP_CONSOLE_PREFIX}{{"version":1,"viewportWidth":100,"viewportHeight":80,"deviceScaleFactor":1,"enabled":true,"defaultOwner":"host","regions":[{{"id":7,"owner":"web","shape":"roundedRect","x":1,"y":2,"width":30,"height":20,"radius":4}}]}}"#
        );
        let legacy_payload = format!(
            r#"{OVERLAY_PASS_MAP_CONSOLE_PREFIX}{{"version":2,"viewportWidth":100,"viewportHeight":80,"deviceScaleFactor":1,"enabled":true,"regions":[{{"id":9,"shape":1,"disabled":false,"x":3,"y":4,"width":10,"height":12}}]}}"#
        );

        assert!(handle_dom_bridge_message(&ownership_payload, |payload| {
            published.push(payload);
        }));
        assert!(handle_dom_bridge_message(&legacy_payload, |payload| {
            published.push(payload);
        }));

        match &published[0] {
            ipc::OutputPayload::InputOwnershipMap(output) => {
                assert_eq!(1, output.version);
                assert_eq!(2, output.default_owner);
                assert_eq!(1, output.regions.len());
                assert_eq!(1, output.regions[0].owner);
                assert_eq!(2, output.regions[0].shape);
            }
            other => panic!("unexpected payload: {other:?}"),
        }
        match &published[1] {
            ipc::OutputPayload::InputOwnershipMap(output) => {
                assert_eq!(2, output.version);
                assert_eq!(1, output.default_owner);
                assert_eq!(1, output.regions.len());
                assert_eq!(2, output.regions[0].owner);
                assert_eq!(1, output.regions[0].shape);
            }
            other => panic!("unexpected payload: {other:?}"),
        }
    }

    #[test]
    fn handles_typed_bridge_payloads_without_console_prefixes() {
        let mut published = Vec::new();

        assert!(handle_dom_bridge_payload(
            "inputOwnershipMap",
            r#"{"version":3,"viewportWidth":100,"viewportHeight":80,"deviceScaleFactor":1,"enabled":true,"defaultOwner":"web","regions":[]}"#,
            |payload| {
                published.push(payload);
            },
        ));
        assert!(!handle_dom_bridge_payload("unknown", "{}", |payload| {
            published.push(payload);
        }));

        match &published[0] {
            ipc::OutputPayload::InputOwnershipMap(output) => {
                assert_eq!(3, output.version);
                assert_eq!(1, output.default_owner);
                assert!(output.regions.is_empty());
            }
            other => panic!("unexpected payload: {other:?}"),
        }
    }
}
