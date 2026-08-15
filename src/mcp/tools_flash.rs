use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct EraseFlashParams {
    pub address: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ProgramFlashParams {
    pub address: u64,
    pub data: Vec<u8>,
    /// Read back and verify the programmed data after writing.
    pub verify: Option<bool>,
}
