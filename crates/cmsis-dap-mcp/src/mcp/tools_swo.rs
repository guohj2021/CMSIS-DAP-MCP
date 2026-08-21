use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct StartSwoParams {
    pub baud: u32,
    pub tpiu_clk: u32,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct StopSwoParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ReadSwoParams {
    pub max_bytes: Option<u32>,
}
