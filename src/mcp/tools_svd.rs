use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct LoadSvdParams {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ListPeripheralsParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ReadPeripheralParams {
    pub peripheral: String,
    pub register: String,
    pub field: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WritePeripheralParams {
    pub peripheral: String,
    pub register: String,
    pub field: Option<String>,
    pub value: u64,
}
