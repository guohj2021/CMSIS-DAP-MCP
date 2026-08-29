//! ELF/AXF symbol loader (object crate).

use super::{not_found, Symbol, SymbolDatabase, SymbolKind};
use crate::error::McpError;
use object::{Object, ObjectSection, ObjectSymbol};

/// Load all defined symbols from a firmware ELF/AXF.
pub fn load_elf(path: &std::path::Path) -> Result<SymbolDatabase, McpError> {
    let bytes = std::fs::read(path)
        .map_err(|e| not_found(format!("failed to read {}: {e}", path.display())))?;
    let file = object::File::parse(&bytes[..])
        .map_err(|e| not_found(format!("failed to parse {}: {e}", path.display())))?;
    let mut db = SymbolDatabase::new();
    let mut next_id = 1u64;

    let section_names: std::collections::HashMap<usize, String> = file
        .sections()
        .map(|s| (s.index().0, s.name().unwrap_or("").to_string()))
        .collect();

    for symbol in file.symbols() {
        let Ok(name) = symbol.name() else { continue };
        if name.is_empty() || !symbol.is_definition() {
            continue;
        }
        let section = symbol
            .section_index()
            .map(|i| section_names.get(&i.0).cloned().unwrap_or_default());
        // Prefer the ELF symbol kind (Text/Data) over section-name heuristics;
        // section names are kept for display only.
        let kind = match symbol.kind() {
            object::SymbolKind::Text => SymbolKind::Function,
            object::SymbolKind::Data => SymbolKind::Variable,
            _ => SymbolKind::Other,
        };
        db.push(Symbol {
            id: next_id,
            name: name.to_string(),
            address: symbol.address(),
            size: symbol.size(),
            kind,
            section,
            module: None,
        });
        next_id += 1;
    }
    Ok(db)
}
