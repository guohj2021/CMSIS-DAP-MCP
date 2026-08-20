use schemars::JsonSchema;
use serde::Deserialize;

fn default_core() -> String {
    "armv6m".to_string()
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DefineChipParams {
    /// Path to a Keil FLM flash algorithm file (ARM ELF).
    pub flm: String,
    /// Flash start address.
    pub flash_start: u64,
    /// Flash size in bytes.
    pub flash_size: u64,
    /// SRAM start address.
    pub sram_start: u64,
    /// SRAM size in bytes.
    pub sram_size: u64,
    /// Core type (default: armv6m).
    #[serde(default = "default_core")]
    pub core: String,
    /// Chip/variant name used with connect (default: FLM file stem).
    pub name: Option<String>,
}
