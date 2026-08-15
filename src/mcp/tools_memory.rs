use crate::backend::AccessWidth;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ReadMemoryParams {
    pub address: u64,
    pub width: String,
    #[schemars(default = "default_count")]
    pub count: u32,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WriteMemoryParams {
    pub address: u64,
    pub width: String,
    pub values: Vec<u64>,
}

fn default_count() -> u32 {
    1
}

pub fn parse_width(s: &str) -> Option<AccessWidth> {
    match s {
        "u8" => Some(AccessWidth::U8),
        "u16" => Some(AccessWidth::U16),
        "u32" => Some(AccessWidth::U32),
        "u64" => Some(AccessWidth::U64),
        _ => None,
    }
}
