//! Symbol database abstraction (frozen v5 §9).
//!
//! Two layers: `SymbolDatabase` (symbol table with stable ids) and future
//! DWARF `DebugInfo` (P6.1). Loaders live under `loaders/` so ELF/AXF, MAP
//! and DWARF sources can be added without changing consumers.

mod dwarf;
mod elf;

pub use dwarf::DebugInfo;
pub use elf::load_elf;

use crate::error::{ErrorCode, McpError};

/// Classification of a symbol, derived from the ELF section kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Variable,
    Other,
}

/// One symbol with a stable id (names are not unique across modules).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Symbol {
    pub id: u64,
    pub name: String,
    pub address: u64,
    pub size: u64,
    pub kind: SymbolKind,
    /// ELF section name, when known.
    pub section: Option<String>,
    /// Source module (DWARF compile unit); None until P6.1.
    pub module: Option<String>,
}

/// In-memory symbol table with name + address indexes.
#[derive(Debug, Clone, Default)]
pub struct SymbolDatabase {
    symbols: Vec<Symbol>,
    by_name: std::collections::HashMap<String, Vec<usize>>,
}

impl SymbolDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of stored symbols.
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    /// Add a symbol (used by loaders).
    pub fn push(&mut self, symbol: Symbol) {
        let idx = self.symbols.len();
        self.by_name
            .entry(symbol.name.clone())
            .or_default()
            .push(idx);
        self.symbols.push(symbol);
    }

    /// Resolve a symbol by exact name (first match).
    pub fn resolve_name(&self, name: &str) -> Option<&Symbol> {
        self.by_name
            .get(name)
            .and_then(|v| v.first())
            .map(|i| &self.symbols[*i])
    }

    /// Resolve a symbol by stable id.
    pub fn resolve_id(&self, id: u64) -> Option<&Symbol> {
        self.symbols.iter().find(|s| s.id == id)
    }

    /// Find the nearest defined symbol at or below `address` (for
    /// `symbol + offset` display / breakpoint resolution).
    pub fn resolve_address(&self, address: u64) -> Option<(&Symbol, u64)> {
        self.symbols
            .iter()
            .filter(|s| s.address <= address && s.size > 0)
            .max_by_key(|s| s.address)
            .map(|s| (s, address - s.address))
    }

    /// Search with optional kind + substring pattern, paginated.
    /// Returns `(total, page)`.
    pub fn search(
        &self,
        kind: Option<SymbolKind>,
        pattern: Option<&str>,
        offset: usize,
        limit: usize,
    ) -> (usize, Vec<&Symbol>) {
        let pat = pattern.unwrap_or("").to_ascii_lowercase();
        let matches: Vec<&Symbol> = self
            .symbols
            .iter()
            .filter(|s| kind.map(|k| s.kind == k).unwrap_or(true))
            .filter(|s| pat.is_empty() || s.name.to_ascii_lowercase().contains(&pat))
            .collect();
        let total = matches.len();
        let page = matches
            .into_iter()
            .skip(offset)
            .take(if limit == 0 { usize::MAX } else { limit })
            .collect();
        (total, page)
    }

    /// Load a firmware ELF/AXF into a symbol database.
    pub fn load(path: &std::path::Path) -> Result<Self, McpError> {
        load_elf(path)
    }
}

pub(crate) fn not_found(msg: impl Into<String>) -> McpError {
    McpError::new(ErrorCode::FileError, msg.into())
}
