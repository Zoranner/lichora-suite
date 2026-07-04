use std::string::FromUtf8Error;

const INPUT_MAGIC: [u8; 4] = *b"EBIP";
const OUTPUT_MAGIC: [u8; 4] = *b"EBOP";
const PAYLOAD_VERSION_MAJOR: u8 = 1;
const PAYLOAD_VERSION_MINOR: u8 = 0;
const PAYLOAD_HEADER_SIZE: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum InputPayloadKind {
    MouseButton = 1,
    MouseWheel = 2,
    KeyboardKeyDown = 3,
    KeyboardKeyUp = 4,
    KeyboardChar = 5,
    ImeComposition = 6,
    ImeCommit = 7,
    ImeCancel = 8,
    ImeDeleteSurroundingText = 9,
    ScriptRequest = 10,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum OutputPayloadKind {
    Caret = 1,
    SurroundingText = 2,
    ScriptResult = 3,
    PageEvent = 4,
    OverlayPassMap = 5,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MouseButtonInput {
    pub x: i32,
    pub y: i32,
    pub button: u32,
    pub buttons: u32,
    pub pressed: bool,
    pub click_count: u8,
    pub modifiers: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MouseWheelInput {
    pub x: i32,
    pub y: i32,
    pub delta_x: i32,
    pub delta_y: i32,
    pub modifiers: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyboardKeyInput {
    pub key_code: u32,
    pub native_key_code: u32,
    pub modifiers: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImeCompositionInput {
    pub text: String,
    pub selection_start: i32,
    pub selection_end: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptRequestInput {
    pub request_id: u64,
    pub script: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputPayload {
    MouseButton(MouseButtonInput),
    MouseWheel(MouseWheelInput),
    KeyboardKeyDown(KeyboardKeyInput),
    KeyboardKeyUp(KeyboardKeyInput),
    KeyboardChar { code_point: u32, modifiers: u32 },
    ImeComposition(ImeCompositionInput),
    ImeCommit { text: String },
    ImeCancel,
    ImeDeleteSurroundingText { before: i32, after: i32 },
    ScriptRequest(ScriptRequestInput),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaretOutput {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub visible: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurroundingTextOutput {
    pub text: String,
    pub selection_start: i32,
    pub selection_end: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptResultOutput {
    pub request_id: u64,
    pub succeeded: bool,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageEventOutput {
    pub event_type: u32,
    pub url: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OverlayPassRegionOutput {
    pub id: u32,
    pub shape: u8,
    pub disabled: bool,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OverlayPassMapOutput {
    pub version: u64,
    pub viewport_width: i32,
    pub viewport_height: i32,
    pub device_scale_factor: f32,
    pub enabled: bool,
    pub regions: Vec<OverlayPassRegionOutput>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OutputPayload {
    Caret(CaretOutput),
    SurroundingText(SurroundingTextOutput),
    ScriptResult(ScriptResultOutput),
    PageEvent(PageEventOutput),
    OverlayPassMap(OverlayPassMapOutput),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputPayloadDecodeError {
    BufferTooSmall { expected: usize, actual: usize },
    InvalidMagic([u8; 4]),
    UnsupportedVersion { major: u8, minor: u8 },
    UnknownKind(u16),
    InvalidUtf8 { field: &'static str },
    TrailingGarbage { expected_end: usize, actual: usize },
}

impl std::fmt::Display for InputPayloadDecodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BufferTooSmall { expected, actual } => {
                write!(
                    formatter,
                    "input payload buffer too small: expected {expected}, actual {actual}"
                )
            }
            Self::InvalidMagic(magic) => {
                write!(formatter, "invalid input payload magic: {magic:?}")
            }
            Self::UnsupportedVersion { major, minor } => {
                write!(
                    formatter,
                    "unsupported input payload version: {major}.{minor}"
                )
            }
            Self::UnknownKind(kind) => write!(formatter, "unknown input payload kind: {kind}"),
            Self::InvalidUtf8 { field } => {
                write!(formatter, "invalid utf-8 in input payload field: {field}")
            }
            Self::TrailingGarbage {
                expected_end,
                actual,
            } => write!(
                formatter,
                "trailing input payload bytes: expected end {expected_end}, actual {actual}"
            ),
        }
    }
}

impl std::error::Error for InputPayloadDecodeError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputPayloadDecodeError {
    BufferTooSmall { expected: usize, actual: usize },
    InvalidMagic([u8; 4]),
    UnsupportedVersion { major: u8, minor: u8 },
    UnknownKind(u16),
    InvalidUtf8 { field: &'static str },
    TrailingGarbage { expected_end: usize, actual: usize },
}

impl std::fmt::Display for OutputPayloadDecodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BufferTooSmall { expected, actual } => {
                write!(
                    formatter,
                    "output payload buffer too small: expected {expected}, actual {actual}"
                )
            }
            Self::InvalidMagic(magic) => {
                write!(formatter, "invalid output payload magic: {magic:?}")
            }
            Self::UnsupportedVersion { major, minor } => {
                write!(
                    formatter,
                    "unsupported output payload version: {major}.{minor}"
                )
            }
            Self::UnknownKind(kind) => write!(formatter, "unknown output payload kind: {kind}"),
            Self::InvalidUtf8 { field } => {
                write!(formatter, "invalid utf-8 in output payload field: {field}")
            }
            Self::TrailingGarbage {
                expected_end,
                actual,
            } => write!(
                formatter,
                "trailing output payload bytes: expected end {expected_end}, actual {actual}"
            ),
        }
    }
}

impl std::error::Error for OutputPayloadDecodeError {}

impl InputPayload {
    pub fn kind(&self) -> InputPayloadKind {
        match self {
            Self::MouseButton(_) => InputPayloadKind::MouseButton,
            Self::MouseWheel(_) => InputPayloadKind::MouseWheel,
            Self::KeyboardKeyDown(_) => InputPayloadKind::KeyboardKeyDown,
            Self::KeyboardKeyUp(_) => InputPayloadKind::KeyboardKeyUp,
            Self::KeyboardChar { .. } => InputPayloadKind::KeyboardChar,
            Self::ImeComposition(_) => InputPayloadKind::ImeComposition,
            Self::ImeCommit { .. } => InputPayloadKind::ImeCommit,
            Self::ImeCancel => InputPayloadKind::ImeCancel,
            Self::ImeDeleteSurroundingText { .. } => InputPayloadKind::ImeDeleteSurroundingText,
            Self::ScriptRequest(_) => InputPayloadKind::ScriptRequest,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        write_header(&mut buffer, &INPUT_MAGIC, self.kind() as u16);
        match self {
            Self::MouseButton(input) => {
                write_i32(&mut buffer, input.x);
                write_i32(&mut buffer, input.y);
                write_u32(&mut buffer, input.button);
                write_u32(&mut buffer, input.buttons);
                write_u32(&mut buffer, input.modifiers);
                buffer.push(u8::from(input.pressed));
                buffer.push(input.click_count);
                buffer.extend_from_slice(&[0, 0]);
            }
            Self::MouseWheel(input) => {
                write_i32(&mut buffer, input.x);
                write_i32(&mut buffer, input.y);
                write_i32(&mut buffer, input.delta_x);
                write_i32(&mut buffer, input.delta_y);
                write_u32(&mut buffer, input.modifiers);
            }
            Self::KeyboardKeyDown(input) | Self::KeyboardKeyUp(input) => {
                write_keyboard_key(&mut buffer, input);
            }
            Self::KeyboardChar {
                code_point,
                modifiers,
            } => {
                write_u32(&mut buffer, *code_point);
                write_u32(&mut buffer, *modifiers);
            }
            Self::ImeComposition(input) => {
                write_string(&mut buffer, &input.text);
                write_i32(&mut buffer, input.selection_start);
                write_i32(&mut buffer, input.selection_end);
            }
            Self::ImeCommit { text } => write_string(&mut buffer, text),
            Self::ImeCancel => {}
            Self::ImeDeleteSurroundingText { before, after } => {
                write_i32(&mut buffer, *before);
                write_i32(&mut buffer, *after);
            }
            Self::ScriptRequest(input) => {
                write_u64(&mut buffer, input.request_id);
                write_string(&mut buffer, &input.script);
            }
        }
        buffer
    }

    pub fn decode(buffer: &[u8]) -> Result<Self, InputPayloadDecodeError> {
        let kind = read_input_header(buffer)?;
        let mut cursor = PAYLOAD_HEADER_SIZE;
        let payload = match kind {
            kind if kind == InputPayloadKind::MouseButton as u16 => {
                Self::MouseButton(MouseButtonInput {
                    x: read_i32(buffer, &mut cursor)?,
                    y: read_i32(buffer, &mut cursor)?,
                    button: read_u32(buffer, &mut cursor)?,
                    buttons: read_u32(buffer, &mut cursor)?,
                    modifiers: read_u32(buffer, &mut cursor)?,
                    pressed: read_u8(buffer, &mut cursor)? != 0,
                    click_count: read_u8(buffer, &mut cursor)?,
                })
                .with_padding(buffer, &mut cursor, 2)?
            }
            kind if kind == InputPayloadKind::MouseWheel as u16 => {
                Self::MouseWheel(MouseWheelInput {
                    x: read_i32(buffer, &mut cursor)?,
                    y: read_i32(buffer, &mut cursor)?,
                    delta_x: read_i32(buffer, &mut cursor)?,
                    delta_y: read_i32(buffer, &mut cursor)?,
                    modifiers: read_u32(buffer, &mut cursor)?,
                })
            }
            kind if kind == InputPayloadKind::KeyboardKeyDown as u16 => {
                Self::KeyboardKeyDown(read_keyboard_key(buffer, &mut cursor)?)
            }
            kind if kind == InputPayloadKind::KeyboardKeyUp as u16 => {
                Self::KeyboardKeyUp(read_keyboard_key(buffer, &mut cursor)?)
            }
            kind if kind == InputPayloadKind::KeyboardChar as u16 => Self::KeyboardChar {
                code_point: read_u32(buffer, &mut cursor)?,
                modifiers: read_u32(buffer, &mut cursor)?,
            },
            kind if kind == InputPayloadKind::ImeComposition as u16 => {
                Self::ImeComposition(ImeCompositionInput {
                    text: read_string(buffer, &mut cursor, "text")?,
                    selection_start: read_i32(buffer, &mut cursor)?,
                    selection_end: read_i32(buffer, &mut cursor)?,
                })
            }
            kind if kind == InputPayloadKind::ImeCommit as u16 => Self::ImeCommit {
                text: read_string(buffer, &mut cursor, "text")?,
            },
            kind if kind == InputPayloadKind::ImeCancel as u16 => Self::ImeCancel,
            kind if kind == InputPayloadKind::ImeDeleteSurroundingText as u16 => {
                Self::ImeDeleteSurroundingText {
                    before: read_i32(buffer, &mut cursor)?,
                    after: read_i32(buffer, &mut cursor)?,
                }
            }
            kind if kind == InputPayloadKind::ScriptRequest as u16 => {
                Self::ScriptRequest(ScriptRequestInput {
                    request_id: read_u64(buffer, &mut cursor)?,
                    script: read_string(buffer, &mut cursor, "script")?,
                })
            }
            unknown => return Err(InputPayloadDecodeError::UnknownKind(unknown)),
        };
        require_exact_end(buffer, cursor)?;
        Ok(payload)
    }
}

impl OutputPayload {
    pub fn kind(&self) -> OutputPayloadKind {
        match self {
            Self::Caret(_) => OutputPayloadKind::Caret,
            Self::SurroundingText(_) => OutputPayloadKind::SurroundingText,
            Self::ScriptResult(_) => OutputPayloadKind::ScriptResult,
            Self::PageEvent(_) => OutputPayloadKind::PageEvent,
            Self::OverlayPassMap(_) => OutputPayloadKind::OverlayPassMap,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        write_header(&mut buffer, &OUTPUT_MAGIC, self.kind() as u16);
        match self {
            Self::Caret(output) => {
                write_i32(&mut buffer, output.x);
                write_i32(&mut buffer, output.y);
                write_i32(&mut buffer, output.width);
                write_i32(&mut buffer, output.height);
                buffer.push(u8::from(output.visible));
                buffer.extend_from_slice(&[0, 0, 0]);
            }
            Self::SurroundingText(output) => {
                write_string(&mut buffer, &output.text);
                write_i32(&mut buffer, output.selection_start);
                write_i32(&mut buffer, output.selection_end);
            }
            Self::ScriptResult(output) => {
                write_u64(&mut buffer, output.request_id);
                buffer.push(u8::from(output.succeeded));
                buffer.extend_from_slice(&[0, 0, 0]);
                write_string(&mut buffer, &output.value);
            }
            Self::PageEvent(output) => {
                write_u32(&mut buffer, output.event_type);
                write_string(&mut buffer, &output.url);
                write_string(&mut buffer, &output.detail);
            }
            Self::OverlayPassMap(output) => {
                write_u64(&mut buffer, output.version);
                write_i32(&mut buffer, output.viewport_width);
                write_i32(&mut buffer, output.viewport_height);
                write_f32(&mut buffer, output.device_scale_factor);
                buffer.push(u8::from(output.enabled));
                buffer.extend_from_slice(&[0; 7]);
                write_u32(&mut buffer, output.regions.len() as u32);
                for region in &output.regions {
                    write_overlay_pass_region(&mut buffer, region);
                }
            }
        }
        buffer
    }

    pub fn decode(buffer: &[u8]) -> Result<Self, OutputPayloadDecodeError> {
        let kind = read_output_header(buffer)?;
        let mut cursor = PAYLOAD_HEADER_SIZE;
        let payload = match kind {
            kind if kind == OutputPayloadKind::Caret as u16 => Self::Caret(CaretOutput {
                x: read_output_i32(buffer, &mut cursor)?,
                y: read_output_i32(buffer, &mut cursor)?,
                width: read_output_i32(buffer, &mut cursor)?,
                height: read_output_i32(buffer, &mut cursor)?,
                visible: read_output_u8(buffer, &mut cursor)? != 0,
            })
            .with_output_padding(buffer, &mut cursor, 3)?,
            kind if kind == OutputPayloadKind::SurroundingText as u16 => {
                Self::SurroundingText(SurroundingTextOutput {
                    text: read_output_string(buffer, &mut cursor, "text")?,
                    selection_start: read_output_i32(buffer, &mut cursor)?,
                    selection_end: read_output_i32(buffer, &mut cursor)?,
                })
            }
            kind if kind == OutputPayloadKind::ScriptResult as u16 => {
                let request_id = read_output_u64(buffer, &mut cursor)?;
                let succeeded = read_output_u8(buffer, &mut cursor)? != 0;
                skip_output_padding(buffer, &mut cursor, 3)?;
                Self::ScriptResult(ScriptResultOutput {
                    request_id,
                    succeeded,
                    value: read_output_string(buffer, &mut cursor, "value")?,
                })
            }
            kind if kind == OutputPayloadKind::PageEvent as u16 => {
                Self::PageEvent(PageEventOutput {
                    event_type: read_output_u32(buffer, &mut cursor)?,
                    url: read_output_string(buffer, &mut cursor, "url")?,
                    detail: read_output_string(buffer, &mut cursor, "detail")?,
                })
            }
            kind if kind == OutputPayloadKind::OverlayPassMap as u16 => {
                let version = read_output_u64(buffer, &mut cursor)?;
                let viewport_width = read_output_i32(buffer, &mut cursor)?;
                let viewport_height = read_output_i32(buffer, &mut cursor)?;
                let device_scale_factor = read_output_f32(buffer, &mut cursor)?;
                let enabled = read_output_u8(buffer, &mut cursor)? != 0;
                skip_output_padding(buffer, &mut cursor, 7)?;
                let region_count = read_output_u32(buffer, &mut cursor)?;
                let mut regions = Vec::with_capacity(region_count as usize);
                for _ in 0..region_count {
                    regions.push(read_output_overlay_pass_region(buffer, &mut cursor)?);
                }
                Self::OverlayPassMap(OverlayPassMapOutput {
                    version,
                    viewport_width,
                    viewport_height,
                    device_scale_factor,
                    enabled,
                    regions,
                })
            }
            unknown => return Err(OutputPayloadDecodeError::UnknownKind(unknown)),
        };
        require_output_exact_end(buffer, cursor)?;
        Ok(payload)
    }
}

trait InputPayloadPadding {
    fn with_padding(
        self,
        buffer: &[u8],
        cursor: &mut usize,
        byte_count: usize,
    ) -> Result<Self, InputPayloadDecodeError>
    where
        Self: Sized;
}

impl InputPayloadPadding for InputPayload {
    fn with_padding(
        self,
        buffer: &[u8],
        cursor: &mut usize,
        byte_count: usize,
    ) -> Result<Self, InputPayloadDecodeError> {
        skip_padding(buffer, cursor, byte_count)?;
        Ok(self)
    }
}

trait OutputPayloadPadding {
    fn with_output_padding(
        self,
        buffer: &[u8],
        cursor: &mut usize,
        byte_count: usize,
    ) -> Result<Self, OutputPayloadDecodeError>
    where
        Self: Sized;
}

impl OutputPayloadPadding for OutputPayload {
    fn with_output_padding(
        self,
        buffer: &[u8],
        cursor: &mut usize,
        byte_count: usize,
    ) -> Result<Self, OutputPayloadDecodeError> {
        skip_output_padding(buffer, cursor, byte_count)?;
        Ok(self)
    }
}

fn write_header(buffer: &mut Vec<u8>, magic: &[u8; 4], kind: u16) {
    buffer.extend_from_slice(magic);
    buffer.push(PAYLOAD_VERSION_MAJOR);
    buffer.push(PAYLOAD_VERSION_MINOR);
    write_u16(buffer, kind);
}

fn write_keyboard_key(buffer: &mut Vec<u8>, input: &KeyboardKeyInput) {
    write_u32(buffer, input.key_code);
    write_u32(buffer, input.native_key_code);
    write_u32(buffer, input.modifiers);
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

fn write_f32(buffer: &mut Vec<u8>, value: f32) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(buffer: &mut Vec<u8>, value: u64) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn write_overlay_pass_region(buffer: &mut Vec<u8>, region: &OverlayPassRegionOutput) {
    write_u32(buffer, region.id);
    buffer.push(region.shape);
    buffer.push(u8::from(region.disabled));
    write_u16(buffer, 0);
    write_f32(buffer, region.x);
    write_f32(buffer, region.y);
    write_f32(buffer, region.width);
    write_f32(buffer, region.height);
}

fn read_input_header(buffer: &[u8]) -> Result<u16, InputPayloadDecodeError> {
    require_len(buffer, PAYLOAD_HEADER_SIZE)?;
    let magic = read_magic(buffer);
    if magic != INPUT_MAGIC {
        return Err(InputPayloadDecodeError::InvalidMagic(magic));
    }
    let major = buffer[4];
    let minor = buffer[5];
    if major != PAYLOAD_VERSION_MAJOR || minor != PAYLOAD_VERSION_MINOR {
        return Err(InputPayloadDecodeError::UnsupportedVersion { major, minor });
    }
    Ok(u16::from_le_bytes(
        buffer[6..8].try_into().expect("validated header"),
    ))
}

fn read_output_header(buffer: &[u8]) -> Result<u16, OutputPayloadDecodeError> {
    require_output_len(buffer, PAYLOAD_HEADER_SIZE)?;
    let magic = read_magic(buffer);
    if magic != OUTPUT_MAGIC {
        return Err(OutputPayloadDecodeError::InvalidMagic(magic));
    }
    let major = buffer[4];
    let minor = buffer[5];
    if major != PAYLOAD_VERSION_MAJOR || minor != PAYLOAD_VERSION_MINOR {
        return Err(OutputPayloadDecodeError::UnsupportedVersion { major, minor });
    }
    Ok(u16::from_le_bytes(
        buffer[6..8].try_into().expect("validated header"),
    ))
}

fn read_magic(buffer: &[u8]) -> [u8; 4] {
    buffer[0..4].try_into().expect("validated header")
}

fn read_keyboard_key(
    buffer: &[u8],
    cursor: &mut usize,
) -> Result<KeyboardKeyInput, InputPayloadDecodeError> {
    Ok(KeyboardKeyInput {
        key_code: read_u32(buffer, cursor)?,
        native_key_code: read_u32(buffer, cursor)?,
        modifiers: read_u32(buffer, cursor)?,
    })
}

fn read_string(
    buffer: &[u8],
    cursor: &mut usize,
    field: &'static str,
) -> Result<String, InputPayloadDecodeError> {
    let byte_count = read_u32(buffer, cursor)? as usize;
    let end = cursor
        .checked_add(byte_count)
        .ok_or(InputPayloadDecodeError::BufferTooSmall {
            expected: usize::MAX,
            actual: buffer.len(),
        })?;
    require_len(buffer, end)?;
    let bytes = buffer[*cursor..end].to_vec();
    *cursor = end;
    String::from_utf8(bytes)
        .map_err(|_: FromUtf8Error| InputPayloadDecodeError::InvalidUtf8 { field })
}

fn read_output_string(
    buffer: &[u8],
    cursor: &mut usize,
    field: &'static str,
) -> Result<String, OutputPayloadDecodeError> {
    let byte_count = read_output_u32(buffer, cursor)? as usize;
    let end = cursor
        .checked_add(byte_count)
        .ok_or(OutputPayloadDecodeError::BufferTooSmall {
            expected: usize::MAX,
            actual: buffer.len(),
        })?;
    require_output_len(buffer, end)?;
    let bytes = buffer[*cursor..end].to_vec();
    *cursor = end;
    String::from_utf8(bytes)
        .map_err(|_: FromUtf8Error| OutputPayloadDecodeError::InvalidUtf8 { field })
}

fn read_i32(buffer: &[u8], cursor: &mut usize) -> Result<i32, InputPayloadDecodeError> {
    let value = i32::from_le_bytes(read_array::<4>(buffer, *cursor)?);
    *cursor += 4;
    Ok(value)
}

fn read_u32(buffer: &[u8], cursor: &mut usize) -> Result<u32, InputPayloadDecodeError> {
    let value = u32::from_le_bytes(read_array::<4>(buffer, *cursor)?);
    *cursor += 4;
    Ok(value)
}

fn read_u64(buffer: &[u8], cursor: &mut usize) -> Result<u64, InputPayloadDecodeError> {
    let value = u64::from_le_bytes(read_array::<8>(buffer, *cursor)?);
    *cursor += 8;
    Ok(value)
}

fn read_u8(buffer: &[u8], cursor: &mut usize) -> Result<u8, InputPayloadDecodeError> {
    require_len(buffer, *cursor + 1)?;
    let value = buffer[*cursor];
    *cursor += 1;
    Ok(value)
}

fn read_output_i32(buffer: &[u8], cursor: &mut usize) -> Result<i32, OutputPayloadDecodeError> {
    let value = i32::from_le_bytes(read_output_array::<4>(buffer, *cursor)?);
    *cursor += 4;
    Ok(value)
}

fn read_output_f32(buffer: &[u8], cursor: &mut usize) -> Result<f32, OutputPayloadDecodeError> {
    let value = f32::from_le_bytes(read_output_array::<4>(buffer, *cursor)?);
    *cursor += 4;
    Ok(value)
}

fn read_output_u32(buffer: &[u8], cursor: &mut usize) -> Result<u32, OutputPayloadDecodeError> {
    let value = u32::from_le_bytes(read_output_array::<4>(buffer, *cursor)?);
    *cursor += 4;
    Ok(value)
}

fn read_output_u64(buffer: &[u8], cursor: &mut usize) -> Result<u64, OutputPayloadDecodeError> {
    let value = u64::from_le_bytes(read_output_array::<8>(buffer, *cursor)?);
    *cursor += 8;
    Ok(value)
}

fn read_output_u8(buffer: &[u8], cursor: &mut usize) -> Result<u8, OutputPayloadDecodeError> {
    require_output_len(buffer, *cursor + 1)?;
    let value = buffer[*cursor];
    *cursor += 1;
    Ok(value)
}

fn read_output_overlay_pass_region(
    buffer: &[u8],
    cursor: &mut usize,
) -> Result<OverlayPassRegionOutput, OutputPayloadDecodeError> {
    let id = read_output_u32(buffer, cursor)?;
    let shape = read_output_u8(buffer, cursor)?;
    let disabled = read_output_u8(buffer, cursor)? != 0;
    skip_output_padding(buffer, cursor, 2)?;
    Ok(OverlayPassRegionOutput {
        id,
        shape,
        disabled,
        x: read_output_f32(buffer, cursor)?,
        y: read_output_f32(buffer, cursor)?,
        width: read_output_f32(buffer, cursor)?,
        height: read_output_f32(buffer, cursor)?,
    })
}

fn skip_padding(
    buffer: &[u8],
    cursor: &mut usize,
    byte_count: usize,
) -> Result<(), InputPayloadDecodeError> {
    require_len(buffer, *cursor + byte_count)?;
    *cursor += byte_count;
    Ok(())
}

fn skip_output_padding(
    buffer: &[u8],
    cursor: &mut usize,
    byte_count: usize,
) -> Result<(), OutputPayloadDecodeError> {
    require_output_len(buffer, *cursor + byte_count)?;
    *cursor += byte_count;
    Ok(())
}

fn require_exact_end(buffer: &[u8], cursor: usize) -> Result<(), InputPayloadDecodeError> {
    if cursor == buffer.len() {
        Ok(())
    } else {
        Err(InputPayloadDecodeError::TrailingGarbage {
            expected_end: cursor,
            actual: buffer.len(),
        })
    }
}

fn require_output_exact_end(buffer: &[u8], cursor: usize) -> Result<(), OutputPayloadDecodeError> {
    if cursor == buffer.len() {
        Ok(())
    } else {
        Err(OutputPayloadDecodeError::TrailingGarbage {
            expected_end: cursor,
            actual: buffer.len(),
        })
    }
}

fn require_len(buffer: &[u8], expected: usize) -> Result<(), InputPayloadDecodeError> {
    if buffer.len() < expected {
        Err(InputPayloadDecodeError::BufferTooSmall {
            expected,
            actual: buffer.len(),
        })
    } else {
        Ok(())
    }
}

fn require_output_len(buffer: &[u8], expected: usize) -> Result<(), OutputPayloadDecodeError> {
    if buffer.len() < expected {
        Err(OutputPayloadDecodeError::BufferTooSmall {
            expected,
            actual: buffer.len(),
        })
    } else {
        Ok(())
    }
}

fn read_array<const SIZE: usize>(
    buffer: &[u8],
    offset: usize,
) -> Result<[u8; SIZE], InputPayloadDecodeError> {
    let end = offset
        .checked_add(SIZE)
        .ok_or(InputPayloadDecodeError::BufferTooSmall {
            expected: usize::MAX,
            actual: buffer.len(),
        })?;
    require_len(buffer, end)?;
    Ok(buffer[offset..end].try_into().expect("slice length"))
}

fn read_output_array<const SIZE: usize>(
    buffer: &[u8],
    offset: usize,
) -> Result<[u8; SIZE], OutputPayloadDecodeError> {
    let end = offset
        .checked_add(SIZE)
        .ok_or(OutputPayloadDecodeError::BufferTooSmall {
            expected: usize::MAX,
            actual: buffer.len(),
        })?;
    require_output_len(buffer, end)?;
    Ok(buffer[offset..end].try_into().expect("slice length"))
}
