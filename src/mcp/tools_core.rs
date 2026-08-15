use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ReadCoreRegisterParams {
    pub name: Option<String>,
    pub number: Option<u16>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WriteCoreRegisterParams {
    pub name: Option<String>,
    pub number: Option<u16>,
    pub value: u64,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct HaltParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ResumeParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct StepParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SetBreakpointParams {
    pub address: u64,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ClearBreakpointsParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ListBreakpointsParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ResetParams {}