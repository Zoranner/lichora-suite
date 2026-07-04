//! Overlay pass map console bridge parsing.

use serde_json::Value;

pub const OVERLAY_PASS_MAP_CONSOLE_PREFIX: &str = "__LICHORA_OVERLAY_PASS_MAP__:";
pub const OVERLAY_PASS_MAP_MAX_REGIONS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayPassMapParseError {
    InvalidJson,
    MissingField(&'static str),
    InvalidField(&'static str),
    TooManyRegions,
}

pub fn parse_overlay_pass_map_console_payload(
    payload: &str,
) -> Result<ipc::OverlayPassMapOutput, OverlayPassMapParseError> {
    let value: Value =
        serde_json::from_str(payload).map_err(|_| OverlayPassMapParseError::InvalidJson)?;
    let object = value
        .as_object()
        .ok_or(OverlayPassMapParseError::InvalidJson)?;

    let regions = object
        .get("regions")
        .ok_or(OverlayPassMapParseError::MissingField("regions"))?
        .as_array()
        .ok_or(OverlayPassMapParseError::InvalidField("regions"))?;
    if regions.len() > OVERLAY_PASS_MAP_MAX_REGIONS {
        return Err(OverlayPassMapParseError::TooManyRegions);
    }

    Ok(ipc::OverlayPassMapOutput {
        version: read_version(object.get("version"))?,
        viewport_width: read_positive_i32(object.get("viewportWidth"), "viewportWidth")?,
        viewport_height: read_positive_i32(object.get("viewportHeight"), "viewportHeight")?,
        device_scale_factor: read_positive_f32(
            object.get("deviceScaleFactor"),
            "deviceScaleFactor",
        )?,
        enabled: read_bool(object.get("enabled"), "enabled")?,
        regions: regions
            .iter()
            .map(read_region)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn read_region(value: &Value) -> Result<ipc::OverlayPassRegionOutput, OverlayPassMapParseError> {
    let object = value
        .as_object()
        .ok_or(OverlayPassMapParseError::InvalidField("regions"))?;

    Ok(ipc::OverlayPassRegionOutput {
        id: read_u32(object.get("id"), "id")?,
        shape: 1,
        disabled: object
            .get("disabled")
            .map(|value| {
                value
                    .as_bool()
                    .ok_or(OverlayPassMapParseError::InvalidField("disabled"))
            })
            .transpose()?
            .unwrap_or(false),
        x: read_finite_f32(object.get("x"), "x")?,
        y: read_finite_f32(object.get("y"), "y")?,
        width: read_positive_f32(object.get("width"), "width")?,
        height: read_positive_f32(object.get("height"), "height")?,
    })
}

fn read_version(value: Option<&Value>) -> Result<u64, OverlayPassMapParseError> {
    let value = value.ok_or(OverlayPassMapParseError::MissingField("version"))?;
    if let Some(number) = value.as_u64() {
        return Ok(number);
    }
    if let Some(text) = value.as_str() {
        return text
            .parse::<u64>()
            .map_err(|_| OverlayPassMapParseError::InvalidField("version"));
    }
    Err(OverlayPassMapParseError::InvalidField("version"))
}

fn read_bool(value: Option<&Value>, field: &'static str) -> Result<bool, OverlayPassMapParseError> {
    value
        .ok_or(OverlayPassMapParseError::MissingField(field))?
        .as_bool()
        .ok_or(OverlayPassMapParseError::InvalidField(field))
}

fn read_u32(value: Option<&Value>, field: &'static str) -> Result<u32, OverlayPassMapParseError> {
    let number = value
        .ok_or(OverlayPassMapParseError::MissingField(field))?
        .as_u64()
        .ok_or(OverlayPassMapParseError::InvalidField(field))?;
    u32::try_from(number).map_err(|_| OverlayPassMapParseError::InvalidField(field))
}

fn read_positive_i32(
    value: Option<&Value>,
    field: &'static str,
) -> Result<i32, OverlayPassMapParseError> {
    let number = value
        .ok_or(OverlayPassMapParseError::MissingField(field))?
        .as_i64()
        .ok_or(OverlayPassMapParseError::InvalidField(field))?;
    let number =
        i32::try_from(number).map_err(|_| OverlayPassMapParseError::InvalidField(field))?;
    if number > 0 {
        Ok(number)
    } else {
        Err(OverlayPassMapParseError::InvalidField(field))
    }
}

fn read_positive_f32(
    value: Option<&Value>,
    field: &'static str,
) -> Result<f32, OverlayPassMapParseError> {
    let number = read_finite_f32(value, field)?;
    if number > 0.0 {
        Ok(number)
    } else {
        Err(OverlayPassMapParseError::InvalidField(field))
    }
}

fn read_finite_f32(
    value: Option<&Value>,
    field: &'static str,
) -> Result<f32, OverlayPassMapParseError> {
    let number = value
        .ok_or(OverlayPassMapParseError::MissingField(field))?
        .as_f64()
        .ok_or(OverlayPassMapParseError::InvalidField(field))?;
    let number = number as f32;
    if number.is_finite() {
        Ok(number)
    } else {
        Err(OverlayPassMapParseError::InvalidField(field))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_overlay_pass_map_console_payload, OVERLAY_PASS_MAP_CONSOLE_PREFIX,
        OVERLAY_PASS_MAP_MAX_REGIONS,
    };

    #[test]
    fn parses_enabled_overlay_pass_map_with_numeric_version() {
        let payload = r#"{
            "version": 42,
            "viewportWidth": 1280,
            "viewportHeight": 720,
            "deviceScaleFactor": 1.25,
            "enabled": true,
            "regions": [
                { "id": 7, "x": 10.5, "y": 20.25, "width": 300.0, "height": 120.5 },
                { "id": 8, "x": 0.0, "y": 1.0, "width": 20.0, "height": 30.0, "disabled": true }
            ]
        }"#;

        let output = parse_overlay_pass_map_console_payload(payload).unwrap();

        assert_eq!(42, output.version);
        assert_eq!(1280, output.viewport_width);
        assert_eq!(720, output.viewport_height);
        assert_eq!(1.25, output.device_scale_factor);
        assert!(output.enabled);
        assert_eq!(2, output.regions.len());
        assert_eq!(7, output.regions[0].id);
        assert_eq!(1, output.regions[0].shape);
        assert!(!output.regions[0].disabled);
        assert_eq!(10.5, output.regions[0].x);
        assert_eq!(20.25, output.regions[0].y);
        assert_eq!(300.0, output.regions[0].width);
        assert_eq!(120.5, output.regions[0].height);
        assert!(output.regions[1].disabled);
    }

    #[test]
    fn parses_version_string() {
        let output = parse_overlay_pass_map_console_payload(
            r#"{"version":"18446744073709551615","viewportWidth":1,"viewportHeight":2,"deviceScaleFactor":1,"enabled":false,"regions":[]}"#,
        )
        .unwrap();

        assert_eq!(u64::MAX, output.version);
        assert!(!output.enabled);
    }

    #[test]
    fn exposes_console_prefix() {
        assert_eq!(
            "__LICHORA_OVERLAY_PASS_MAP__:",
            OVERLAY_PASS_MAP_CONSOLE_PREFIX
        );
    }

    #[test]
    fn rejects_invalid_required_fields_and_coordinates() {
        for payload in [
            r#"{"version":1,"viewportWidth":0,"viewportHeight":1,"deviceScaleFactor":1,"enabled":true,"regions":[]}"#,
            r#"{"version":1,"viewportWidth":1,"viewportHeight":0,"deviceScaleFactor":1,"enabled":true,"regions":[]}"#,
            r#"{"version":1,"viewportWidth":1,"viewportHeight":1,"deviceScaleFactor":0,"enabled":true,"regions":[]}"#,
            r#"{"version":1,"viewportWidth":1,"viewportHeight":1,"deviceScaleFactor":1,"enabled":true,"regions":[{"id":1,"x":0,"y":0,"width":0,"height":1}]}"#,
            r#"{"version":1,"viewportWidth":1,"viewportHeight":1,"deviceScaleFactor":1,"enabled":true,"regions":[{"id":1,"x":0,"y":0,"width":1,"height":-1}]}"#,
            r#"{"version":"not-a-number","viewportWidth":1,"viewportHeight":1,"deviceScaleFactor":1,"enabled":true,"regions":[]}"#,
            r#"{"version":1,"viewportWidth":1,"viewportHeight":1,"deviceScaleFactor":1,"enabled":true}"#,
        ] {
            assert!(
                parse_overlay_pass_map_console_payload(payload).is_err(),
                "payload should be rejected: {payload}"
            );
        }
    }

    #[test]
    fn rejects_more_than_max_regions() {
        let regions = (0..=OVERLAY_PASS_MAP_MAX_REGIONS)
            .map(|id| format!(r#"{{"id":{id},"x":0,"y":0,"width":1,"height":1}}"#))
            .collect::<Vec<_>>()
            .join(",");
        let payload = format!(
            r#"{{"version":1,"viewportWidth":1,"viewportHeight":1,"deviceScaleFactor":1,"enabled":true,"regions":[{regions}]}}"#
        );

        assert!(parse_overlay_pass_map_console_payload(&payload).is_err());
    }
}
