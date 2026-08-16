use cmsis_dap_core::backend::AccessWidth;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ReadMemoryParams {
    pub address: u64,
    pub width: String,
    #[schemars(default = "default_count")]
    pub count: u32,
    /// Export mode: path of the file to write (bin or hex). When set, count is the number of bytes to read.
    pub path: Option<String>,
    /// Export format: "bin" (default) or "hex".
    pub format: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WriteMemoryParams {
    pub address: u64,
    pub width: String,
    pub values: Vec<u64>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct VerifyMemoryParams {
    pub address: u64,
    pub width: String,
    pub data: Vec<u64>,
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
