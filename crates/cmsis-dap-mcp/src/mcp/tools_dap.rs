use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ReadDapParams {
    pub address: u32,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WriteDapParams {
    pub address: u32,
    pub value: u32,
}
