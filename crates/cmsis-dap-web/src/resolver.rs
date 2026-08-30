//! DebugResolver (frozen v5 §5): symbol / address / register / peripheral
//! resolution shared by Watch, Breakpoint, Go To, Memory and Symbol Explorer.

use cmsis_dap_core::svd::SvdDatabase;
use cmsis_dap_core::symbols::{Symbol, SymbolDatabase};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SymbolAt {
    pub name: String,
    pub address: u64,
    pub offset: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddressInfo {
    pub address: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<SymbolAt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peripheral: Option<String>,
}

/// Resolve a symbol by name (first match).
pub fn resolve_symbol_name<'a>(db: &'a SymbolDatabase, name: &str) -> Option<&'a Symbol> {
    db.resolve_name(name)
}

/// Resolve a symbol by stable id.
pub fn resolve_symbol_id(db: &SymbolDatabase, id: u64) -> Option<&Symbol> {
    db.resolve_id(id)
}

/// Nearest symbol at or below `address`.
pub fn resolve_address(db: &SymbolDatabase, address: u64) -> Option<SymbolAt> {
    db.resolve_address(address).map(|(s, offset)| SymbolAt {
        name: s.name.clone(),
        address: s.address,
        offset,
    })
}

/// Describe an address: memory region + nearest symbol + owning peripheral.
pub fn describe_address(
    db: &SymbolDatabase,
    svd: Option<&SvdDatabase>,
    regions: &[(String, u64, u64)],
    address: u64,
) -> AddressInfo {
    let region = regions
        .iter()
        .find(|(_, start, end)| address >= *start && address < *end)
        .map(|(name, _, _)| name.clone());
    let symbol = resolve_address(db, address);
    let peripheral = svd.and_then(|s| {
        s.peripherals()
            .iter()
            .find(|p| address >= p.base && address < p.base + 0x1000)
            .map(|p| p.name.clone())
    });
    AddressInfo {
        address,
        region,
        symbol,
        peripheral,
    }
}
