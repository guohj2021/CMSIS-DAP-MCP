//! ELF symbol lookup shared by `symbols`, `watch`, `rtt` and `evr`.

use crate::cmd::CliError;
use cmsis_dap_core::error::{ErrorCode, McpError};
use object::{Object, ObjectSymbol};
use std::collections::BTreeMap;
use std::path::Path;

fn file_error(msg: impl Into<String>) -> CliError {
    CliError::Mcp(McpError::new(ErrorCode::FileError, msg))
}

/// Load all defined symbols (name -> virtual address) from a firmware ELF.
pub fn load_symbols(path: &Path) -> Result<BTreeMap<String, u64>, CliError> {
    let bytes = std::fs::read(path)
        .map_err(|e| file_error(format!("failed to read ELF {}: {e}", path.display())))?;
    let file = object::File::parse(&bytes[..])
        .map_err(|e| file_error(format!("failed to parse ELF {}: {e}", path.display())))?;
    let mut symbols = BTreeMap::new();
    for symbol in file.symbols() {
        let Ok(name) = symbol.name() else {
            continue;
        };
        if name.is_empty() || !symbol.is_definition() || symbol.section_index().is_none() {
            continue;
        }
        symbols.entry(name.to_string()).or_insert(symbol.address());
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
