pub mod legacy_pass;
mod map_payload;

pub use legacy_pass::input_ownership_map_from_legacy_pass_map;
pub use map_payload::{
    parse_input_ownership_map_console_payload, INPUT_OWNERSHIP_MAP_CONSOLE_PREFIX,
    INPUT_OWNERSHIP_MAP_MAX_REGIONS,
};
