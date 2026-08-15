use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct EraseFlashParams {
    pub address: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ProgramFlashParams {
    pub address: u64,
    /// Raw bytes to program. Provide exactly one of data or path.
    pub data: Option<Vec<u8>>,
    /// Path of a firmware file to program (axf, elf, bin or hex). Provide exactly one of data or path.
    pub path: Option<String>,
    /// File format: elf, axf, bin, hex, ihex, intelhex or auto (default, inferred from the extension).
    pub format: Option<String>,
    /// Read back and verify the programmed data after writing.
    pub verify: Option<bool>,
}
