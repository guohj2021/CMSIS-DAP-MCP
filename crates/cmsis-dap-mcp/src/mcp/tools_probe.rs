use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ListProbesParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetProbeInfoParams {
    pub probe_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ConnectParams {
    pub probe_id: Option<String>,
    pub protocol: Option<String>,
    pub speed_khz: Option<u32>,
    pub target: Option<String>,
    /// Connect while holding the target reset line (for locked or non-responsive targets).
    pub under_reset: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DisconnectParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetTargetInfoParams {}
