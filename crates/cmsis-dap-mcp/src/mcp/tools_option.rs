use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ReadOptionBytesParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WriteOptionBytesParams {
    pub bytes: Vec<OptionByteParam>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct OptionByteParam {
    pub name: String,
    pub address: u32,
    pub value: u32,
}
