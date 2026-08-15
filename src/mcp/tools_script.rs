use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct RunScriptParams {
    /// Path of a script file (J-Link Commander / OpenOCD style). Provide exactly one of path or script.
    pub path: Option<String>,
    /// Inline script text. Provide exactly one of path or script.
    pub script: Option<String>,
}
