use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ChipGenerateParams {
    /// Path to the Keil FLM flash algorithm file.
    pub flm: String,
    /// Flash start address.
    pub flash_start: u64,
    /// Flash size in bytes.
    pub flash_size: u64,
    /// SRAM start address (algorithm load target).
    pub sram_start: u64,
    /// SRAM size in bytes.
    pub sram_size: u64,
    /// Chip/variant name (default: FLM file stem).
    pub name: Option<String>,
    /// Core type (default: armv6m).
    pub core: Option<String>,
    /// When true, load the generated target into the session so
    /// subsequent connect calls can use it.
    pub load: Option<bool>,
}
