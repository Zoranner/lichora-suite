use std::string::FromUtf8Error;

const CONTROL_MAGIC: [u8; 4] = *b"EBCC";
const CONTROL_VERSION_MAJOR: u8 = 1;
const CONTROL_VERSION_MINOR: u8 = 0;
const CONTROL_HEADER_SIZE: usize = 8;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlCommand {
    Shutdown,
    AddBrowser {
        browser_id: String,
        width: i32,
        height: i32,
        address: String,
    },
    RemoveBrowser {
        browser_id: String,
    },
    ResizeBrowser {
        browser_id: String,
        width: i32,
        height: i32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlDecodeError {
    BufferTooSmall { expected: usize, actual: usize },
    InvalidMagic([u8; 4]),
    UnsupportedVersion { major: u8, minor: u8 },
    UnknownKind(u16),
    InvalidUtf8 { field: &'static str },
    TrailingGarbage { expected_end: usize, actual: usize },
}

impl std::fmt::Display for ControlDecodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControlDecodeError::BufferTooSmall { expected, actual } => {
                write!(
                    formatter,
                    "control buffer too small: expected {expected}, actual {actual}"
                )
            }
            ControlDecodeError::InvalidMagic(magic) => {
                write!(formatter, "invalid control magic: {magic:?}")
            }
            ControlDecodeError::UnsupportedVersion { major, minor } => {
                write!(formatter, "unsupported control version: {major}.{minor}")
            }
            ControlDecodeError::UnknownKind(kind) => {
                write!(formatter, "unknown control command kind: {kind}")
            }
            ControlDecodeError::InvalidUtf8 { field } => {
                write!(formatter, "invalid utf-8 in control field: {field}")
            }
            ControlDecodeError::TrailingGarbage {
                expected_end,
                actual,
            } => {
                write!(
                    formatter,
                    "trailing control bytes: expected end {expected_end}, actual {actual}"
                )
            }
        }
    }
}

impl std::error::Error for ControlDecodeError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
enum ControlCommandKind {
    Shutdown = 1,
    AddBrowser = 2,
    RemoveBrowser = 3,
    ResizeBrowser = 4,
}

impl ControlCommand {
    pub fn encode(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        match self {
            ControlCommand::Shutdown => write_header(&mut buffer, ControlCommandKind::Shutdown),
            ControlCommand::AddBrowser {
                browser_id,
                width,
                height,
                address,
            } => {
                write_header(&mut buffer, ControlCommandKind::AddBrowser);
                write_string(&mut buffer, browser_id);
                write_i32(&mut buffer, *width);
                write_i32(&mut buffer, *height);
                write_string(&mut buffer, address);
            }
            ControlCommand::RemoveBrowser { browser_id } => {
                write_header(&mut buffer, ControlCommandKind::RemoveBrowser);
                write_string(&mut buffer, browser_id);
            }
            ControlCommand::ResizeBrowser {
                browser_id,
                width,
                height,
            } => {
                write_header(&mut buffer, ControlCommandKind::ResizeBrowser);
                write_string(&mut buffer, browser_id);
                write_i32(&mut buffer, *width);
                write_i32(&mut buffer, *height);
            }
        }
        buffer
    }

    pub fn decode(buffer: &[u8]) -> Result<Self, ControlDecodeError> {
        require_len(buffer, CONTROL_HEADER_SIZE)?;

        let mut magic = [0u8; 4];
        magic.copy_from_slice(&buffer[0..4]);
        if magic != CONTROL_MAGIC {
            return Err(ControlDecodeError::InvalidMagic(magic));
        }

        let major = buffer[4];
        let minor = buffer[5];
        if major != CONTROL_VERSION_MAJOR || minor != CONTROL_VERSION_MINOR {
            return Err(ControlDecodeError::UnsupportedVersion { major, minor });
        }

        let kind = read_u16(buffer, 6);
        let mut cursor = CONTROL_HEADER_SIZE;
        let command = match kind {
            kind if kind == ControlCommandKind::Shutdown as u16 => ControlCommand::Shutdown,
            kind if kind == ControlCommandKind::AddBrowser as u16 => {
                let browser_id = read_string(buffer, &mut cursor, "browser_id")?;
                let width = read_i32(buffer, &mut cursor)?;
                let height = read_i32(buffer, &mut cursor)?;
                let address = read_string(buffer, &mut cursor, "address")?;
                ControlCommand::AddBrowser {
                    browser_id,
                    width,
                    height,
                    address,
                }
            }
            kind if kind == ControlCommandKind::RemoveBrowser as u16 => {
                let browser_id = read_string(buffer, &mut cursor, "browser_id")?;
                ControlCommand::RemoveBrowser { browser_id }
            }
            kind if kind == ControlCommandKind::ResizeBrowser as u16 => {
                let browser_id = read_string(buffer, &mut cursor, "browser_id")?;
                let width = read_i32(buffer, &mut cursor)?;
                let height = read_i32(buffer, &mut cursor)?;
                ControlCommand::ResizeBrowser {
                    browser_id,
                    width,
                    height,
                }
            }
            unknown => return Err(ControlDecodeError::UnknownKind(unknown)),
        };

        if cursor != buffer.len() {
            return Err(ControlDecodeError::TrailingGarbage {
                expected_end: cursor,
                actual: buffer.len(),
            });
        }

        Ok(command)
    }
}

fn write_header(buffer: &mut Vec<u8>, kind: ControlCommandKind) {
    buffer.extend_from_slice(&CONTROL_MAGIC);
    buffer.push(CONTROL_VERSION_MAJOR);
    buffer.push(CONTROL_VERSION_MINOR);
    write_u16(buffer, kind as u16);
}

fn write_string(buffer: &mut Vec<u8>, value: &str) {
    write_u32(buffer, value.len() as u32);
    buffer.extend_from_slice(value.as_bytes());
}

fn write_u16(buffer: &mut Vec<u8>, value: u16) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(buffer: &mut Vec<u8>, value: u32) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn write_i32(buffer: &mut Vec<u8>, value: i32) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn require_len(buffer: &[u8], expected: usize) -> Result<(), ControlDecodeError> {
    if buffer.len() < expected {
        Err(ControlDecodeError::BufferTooSmall {
            expected,
            actual: buffer.len(),
        })
    } else {
        Ok(())
    }
}

fn read_string(
    buffer: &[u8],
    cursor: &mut usize,
    field: &'static str,
) -> Result<String, ControlDecodeError> {
    let byte_count = read_u32_at_cursor(buffer, cursor)? as usize;
    let end = cursor
        .checked_add(byte_count)
        .ok_or(ControlDecodeError::BufferTooSmall {
            expected: usize::MAX,
            actual: buffer.len(),
        })?;
    require_len(buffer, end)?;

    let bytes = buffer[*cursor..end].to_vec();
    *cursor = end;
    String::from_utf8(bytes).map_err(|_: FromUtf8Error| ControlDecodeError::InvalidUtf8 { field })
}

fn read_i32(buffer: &[u8], cursor: &mut usize) -> Result<i32, ControlDecodeError> {
    let value = i32::from_le_bytes(read_array::<4>(buffer, *cursor)?);
    *cursor += 4;
    Ok(value)
}

fn read_u32_at_cursor(buffer: &[u8], cursor: &mut usize) -> Result<u32, ControlDecodeError> {
    let value = u32::from_le_bytes(read_array::<4>(buffer, *cursor)?);
    *cursor += 4;
    Ok(value)
}

fn read_u16(buffer: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(read_array::<2>(buffer, offset).expect("validated header"))
}

fn read_array<const SIZE: usize>(
    buffer: &[u8],
    offset: usize,
) -> Result<[u8; SIZE], ControlDecodeError> {
    let end = offset
        .checked_add(SIZE)
        .ok_or(ControlDecodeError::BufferTooSmall {
            expected: usize::MAX,
            actual: buffer.len(),
        })?;
    require_len(buffer, end)?;
    Ok(buffer[offset..end].try_into().expect("slice length"))
}
