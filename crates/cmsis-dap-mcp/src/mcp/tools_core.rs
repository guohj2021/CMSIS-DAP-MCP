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
pub struct ResetParams {
    /// Reset mode: "run" (default, reset and continue) or "halt" (reset and halt).
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ListCoreRegistersParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetCoreStatusParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DumpCpuStateParams {
    /// Optional addresses to sample (word reads) alongside the registers.
    pub addresses: Option<Vec<u64>>,
    /// Number of words to dump from the top of MSP/PSP stacks (default 16).
    pub stack_words: Option<u32>,
    /// Restore the previous run state after the dump (default true).
    pub restore: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SetWatchpointParams {
    pub address: u64,
    /// Access type to watch: "read", "write" or "rw".
    pub access: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ClearWatchpointsParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ListWatchpointsParams {}
