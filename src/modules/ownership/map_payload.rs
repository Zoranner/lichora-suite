use serde_json::Value;

pub const INPUT_OWNERSHIP_MAP_CONSOLE_PREFIX: &str = "__LICHORA_INPUT_OWNERSHIP_MAP__:";
pub const INPUT_OWNERSHIP_MAP_MAX_REGIONS: usize = 256;

const INPUT_OWNER_WEB: u8 = 1;
const INPUT_OWNER_HOST: u8 = 2;
const INPUT_REGION_SHAPE_RECT: u8 = 1;
const INPUT_REGION_SHAPE_ROUNDED_RECT: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputOwnershipMapParseError {
    InvalidJson,
    MissingField(&'static str),
    InvalidField(&'static str),
    TooManyRegions,
}

pub fn parse_input_ownership_map_console_payload(
    payload: &str,
) -> Result<ipc::InputOwnershipMapOutput, InputOwnershipMapParseError> {
    let value: Value =
        serde_json::from_str(payload).map_err(|_| InputOwnershipMapParseError::InvalidJson)?;
    let object = value
        .as_object()
        .ok_or(InputOwnershipMapParseError::InvalidJson)?;

    let regions = object
        .get("regions")
        .ok_or(InputOwnershipMapParseError::MissingField("regions"))?
        .as_array()
        .ok_or(InputOwnershipMapParseError::InvalidField("regions"))?;
    if regions.len() > INPUT_OWNERSHIP_MAP_MAX_REGIONS {
        return Err(InputOwnershipMapParseError::TooManyRegions);
    }

    Ok(ipc::InputOwnershipMapOutput {
        version: read_version(object.get("version"))?,
        viewport_width: read_positive_i32(object.get("viewportWidth"), "viewportWidth")?,
        viewport_height: read_positive_i32(object.get("viewportHeight"), "viewportHeight")?,
        device_scale_factor: read_positive_f32(
            object.get("deviceScaleFactor"),
            "deviceScaleFactor",
        )?,
        enabled: read_bool(object.get("enabled"), "enabled")?,
        default_owner: read_owner(object.get("defaultOwner"), "defaultOwner")?,
        regions: regions
            .iter()
            .map(read_region)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn read_region(
    value: &Value,
) -> Result<ipc::InputOwnershipRegionOutput, InputOwnershipMapParseError> {
    let object = value
        .as_object()
        .ok_or(InputOwnershipMapParseError::InvalidField("regions"))?;

    Ok(ipc::InputOwnershipRegionOutput {
        id: read_u32(object.get("id"), "id")?,
        owner: read_owner(object.get("owner"), "owner")?,
        shape: read_shape(object.get("shape"), "shape")?,
        disabled: object
            .get("disabled")
            .map(|value| {
                value
                    .as_bool()
                    .ok_or(InputOwnershipMapParseError::InvalidField("disabled"))
            })
            .transpose()?
            .unwrap_or(false),
        x: read_finite_f32(object.get("x"), "x")?,
        y: read_finite_f32(object.get("y"), "y")?,
        width: read_positive_f32(object.get("width"), "width")?,
        height: read_positive_f32(object.get("height"), "height")?,
        radius: read_non_negative_f32(object.get("radius"), "radius")?,
    })
}

fn read_version(value: Option<&Value>) -> Result<u64, InputOwnershipMapParseError> {
    let value = value.ok_or(InputOwnershipMapParseError::MissingField("version"))?;
    if let Some(number) = value.as_u64() {
        return Ok(number);
    }
    if let Some(text) = value.as_str() {
        return text
            .parse::<u64>()
            .map_err(|_| InputOwnershipMapParseError::InvalidField("version"));
    }
    Err(InputOwnershipMapParseError::InvalidField("version"))
}

fn read_owner(
    value: Option<&Value>,
    field: &'static str,
) -> Result<u8, InputOwnershipMapParseError> {
    match value
        .ok_or(InputOwnershipMapParseError::MissingField(field))?
        .as_str()
        .ok_or(InputOwnershipMapParseError::InvalidField(field))?
    {
        "web" => Ok(INPUT_OWNER_WEB),
        "host" => Ok(INPUT_OWNER_HOST),
        _ => Err(InputOwnershipMapParseError::InvalidField(field)),
    }
}

fn read_shape(
    value: Option<&Value>,
    field: &'static str,
) -> Result<u8, InputOwnershipMapParseError> {
    match value
        .ok_or(InputOwnershipMapParseError::MissingField(field))?
        .as_str()
        .ok_or(InputOwnershipMapParseError::InvalidField(field))?
    {
        "rect" => Ok(INPUT_REGION_SHAPE_RECT),
        "roundedRect" => Ok(INPUT_REGION_SHAPE_ROUNDED_RECT),
        _ => Err(InputOwnershipMapParseError::InvalidField(field)),
    }
}

fn read_bool(
    value: Option<&Value>,
    field: &'static str,
) -> Result<bool, InputOwnershipMapParseError> {
    value
        .ok_or(InputOwnershipMapParseError::MissingField(field))?
        .as_bool()
        .ok_or(InputOwnershipMapParseError::InvalidField(field))
}

fn read_u32(
    value: Option<&Value>,
    field: &'static str,
) -> Result<u32, InputOwnershipMapParseError> {
    let number = value
        .ok_or(InputOwnershipMapParseError::MissingField(field))?
        .as_u64()
        .ok_or(InputOwnershipMapParseError::InvalidField(field))?;
    u32::try_from(number).map_err(|_| InputOwnershipMapParseError::InvalidField(field))
}

fn read_positive_i32(
    value: Option<&Value>,
    field: &'static str,
) -> Result<i32, InputOwnershipMapParseError> {
    let number = value
        .ok_or(InputOwnershipMapParseError::MissingField(field))?
        .as_i64()
        .ok_or(InputOwnershipMapParseError::InvalidField(field))?;
    let number =
        i32::try_from(number).map_err(|_| InputOwnershipMapParseError::InvalidField(field))?;
    if number > 0 {
        Ok(number)
    } else {
        Err(InputOwnershipMapParseError::InvalidField(field))
    }
}

fn read_positive_f32(
    value: Option<&Value>,
    field: &'static str,
) -> Result<f32, InputOwnershipMapParseError> {
    let number = read_finite_f32(value, field)?;
    if number > 0.0 {
        Ok(number)
    } else {
        Err(InputOwnershipMapParseError::InvalidField(field))
    }
}

fn read_non_negative_f32(
    value: Option<&Value>,
    field: &'static str,
) -> Result<f32, InputOwnershipMapParseError> {
    let number = read_finite_f32(value, field)?;
    if number >= 0.0 {
        Ok(number)
    } else {
        Err(InputOwnershipMapParseError::InvalidField(field))
    }
}

fn read_finite_f32(
    value: Option<&Value>,
    field: &'static str,
) -> Result<f32, InputOwnershipMapParseError> {
    let number = value
        .ok_or(InputOwnershipMapParseError::MissingField(field))?
        .as_f64()
        .ok_or(InputOwnershipMapParseError::InvalidField(field))?;
    let number = number as f32;
    if number.is_finite() {
        Ok(number)
    } else {
        Err(InputOwnershipMapParseError::InvalidField(field))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_input_ownership_map_console_payload, INPUT_OWNERSHIP_MAP_CONSOLE_PREFIX,
        INPUT_OWNERSHIP_MAP_MAX_REGIONS,
    };

    #[test]
    fn parses_enabled_input_ownership_map() {
        let output = parse_input_ownership_map_console_payload(
            r#"{"version":"42","viewportWidth":1280,"viewportHeight":720,"deviceScaleFactor":1.25,"enabled":true,"defaultOwner":"host","regions":[{"id":7,"owner":"web","shape":"roundedRect","x":10.5,"y":20.25,"width":300,"height":120.5,"radius":12,"disabled":false}]}"#,
        )
        .unwrap();

        assert_eq!(42, output.version);
        assert_eq!(1280, output.viewport_width);
        assert_eq!(720, output.viewport_height);
        assert_eq!(1.25, output.device_scale_factor);
        assert!(output.enabled);
        assert_eq!(2, output.default_owner);
        assert_eq!(1, output.regions.len());
        assert_eq!(7, output.regions[0].id);
        assert_eq!(1, output.regions[0].owner);
        assert_eq!(2, output.regions[0].shape);
        assert!(!output.regions[0].disabled);
        assert_eq!(12.0, output.regions[0].radius);
    }

    #[test]
    fn exposes_console_prefix() {
        assert_eq!(
            "__LICHORA_INPUT_OWNERSHIP_MAP__:",
            INPUT_OWNERSHIP_MAP_CONSOLE_PREFIX
        );
    }

    #[test]
    fn rejects_invalid_required_fields_and_too_many_regions() {
        for payload in [
            r#"{"version":1,"viewportWidth":0,"viewportHeight":1,"deviceScaleFactor":1,"enabled":true,"defaultOwner":"web","regions":[]}"#,
            r#"{"version":1,"viewportWidth":1,"viewportHeight":1,"deviceScaleFactor":0,"enabled":true,"defaultOwner":"web","regions":[]}"#,
            r#"{"version":1,"viewportWidth":1,"viewportHeight":1,"deviceScaleFactor":1,"enabled":true,"defaultOwner":"invalid","regions":[]}"#,
            r#"{"version":1,"viewportWidth":1,"viewportHeight":1,"deviceScaleFactor":1,"enabled":true,"defaultOwner":"web","regions":[{"id":1,"owner":"host","shape":"polygon","x":0,"y":0,"width":1,"height":1,"radius":0}]}"#,
            r#"{"version":1,"viewportWidth":1,"viewportHeight":1,"deviceScaleFactor":1,"enabled":true,"defaultOwner":"web","regions":[{"id":1,"owner":"host","shape":"rect","x":0,"y":0,"width":1,"height":1,"radius":-1}]}"#,
        ] {
            assert!(
                parse_input_ownership_map_console_payload(payload).is_err(),
                "payload should be rejected: {payload}"
            );
        }

        let regions = (0..=INPUT_OWNERSHIP_MAP_MAX_REGIONS)
            .map(|id| {
                format!(
                    r#"{{"id":{id},"owner":"host","shape":"rect","x":0,"y":0,"width":1,"height":1,"radius":0}}"#
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let payload = format!(
            r#"{{"version":1,"viewportWidth":1,"viewportHeight":1,"deviceScaleFactor":1,"enabled":true,"defaultOwner":"web","regions":[{regions}]}}"#
        );

        assert!(parse_input_ownership_map_console_payload(&payload).is_err());
    }
}
