//! Caret console payload parsing.

pub fn parse_caret_console_payload(payload: &str) -> Option<(i16, i16, i16)> {
    let mut parts = payload.split(',');
    let x = parts.next()?.trim().parse::<i32>().ok()?;
    let y = parts.next()?.trim().parse::<i32>().ok()?;
    let height = parts.next()?.trim().parse::<i32>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((clamp_i16(x), clamp_i16(y), clamp_i16(height)))
}

fn clamp_i16(value: i32) -> i16 {
    value.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

#[cfg(test)]
mod tests {
    use super::parse_caret_console_payload;

    #[test]
    fn parses_console_payload_and_clamps_to_i16() {
        assert_eq!(
            Some((32767, -32768, 20)),
            parse_caret_console_payload("40000,-40000,20")
        );
        assert_eq!(None, parse_caret_console_payload("1,2"));
        assert_eq!(None, parse_caret_console_payload("1,2,3,4"));
    }
}
