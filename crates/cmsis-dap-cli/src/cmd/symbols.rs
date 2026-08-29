//! ELF symbol lookup shared by `symbols`, `watch`, `rtt` and `evr`.
//!
//! Thin wrapper over `cmsis_dap_core::symbols::SymbolDatabase` (kept here so
//! existing callers in this crate keep the same `BTreeMap`-based API).

use crate::cmd::CliError;
use cmsis_dap_core::error::{ErrorCode, McpError};
use cmsis_dap_core::symbols::SymbolDatabase;
use std::collections::BTreeMap;
use std::path::Path;

fn file_error(msg: impl Into<String>) -> CliError {
    CliError::Mcp(McpError::new(ErrorCode::FileError, msg.into()))
}

/// Load all defined symbols (name -> virtual address) from a firmware ELF.
pub fn load_symbols(path: &Path) -> Result<BTreeMap<String, u64>, CliError> {
    let db = SymbolDatabase::load(path).map_err(|e| file_error(e.to_string()))?;
    let mut symbols = BTreeMap::new();
    for symbol in db.symbols() {
        symbols
            .entry(symbol.name.clone())
            .or_insert(symbol.address);
    }
    Ok(symbols)
}

/// Resolve a symbol name to its address, or `None` when absent.
pub fn resolve(symbols: &BTreeMap<String, u64>, name: &str) -> Option<u64> {
    symbols.get(name).copied()
}

/// Resolve an ELF symbol for RTT / Event Recorder address discovery.
pub fn resolve_from_elf(path: Option<&Path>, symbol: &str) -> Result<Option<u64>, CliError> {
    let Some(path) = path else {
        return Ok(None);
    };
    let symbols = load_symbols(path)?;
    Ok(resolve(&symbols, symbol))
}