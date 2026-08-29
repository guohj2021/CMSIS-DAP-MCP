//! DWARF source-location resolver (frozen v5 P6.1, layer 2 of the symbol
//! architecture). Kept separate from SymbolDatabase; falls back to None when
//! the firmware has no DWARF.

use crate::error::{ErrorCode, McpError};
use object::{Object, ObjectSection};
use std::path::Path;

/// A resolved source location for an address.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: u64,
    pub column: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
}

/// DWARF debug-info context loaded from a firmware ELF/AXF.
pub struct DebugInfo {
    ctx:
        addr2line::Context<addr2line::gimli::EndianSlice<'static, addr2line::gimli::RunTimeEndian>>,
}

impl DebugInfo {
    /// Load DWARF from a firmware ELF/AXF. Returns None when the file has no
    /// usable debug info (callers show "not available", never fake).
    pub fn load(path: &Path) -> Result<Option<Self>, McpError> {
        let bytes = std::fs::read(path).map_err(|e| {
            McpError::new(
                ErrorCode::FileError,
                format!("failed to read {}: {e}", path.display()),
            )
        })?;
        let file = match object::File::parse(&bytes[..]) {
            Ok(f) => f,
            Err(e) => {
                return Err(McpError::new(
                    ErrorCode::FileError,
                    format!("failed to parse: {e}"),
                ))
            }
        };
        if file.section_by_name(".debug_info").is_none() {
            return Ok(None);
        }
        use addr2line::gimli;
        let endian = if file.is_little_endian() {
            gimli::RunTimeEndian::Little
        } else {
            gimli::RunTimeEndian::Big
        };
        let load_section = |id: gimli::SectionId| -> Result<
            gimli::EndianSlice<'static, gimli::RunTimeEndian>,
            gimli::Error,
        > {
            let name = id.name();
            let data: &'static [u8] = match file
                .section_by_name(name)
                .and_then(|s| s.uncompressed_data().ok())
            {
                Some(d) => Box::leak(d.into_owned().into_boxed_slice()),
                None => &[],
            };
            Ok(gimli::EndianSlice::new(data, endian))
        };
        let dwarf = match gimli::Dwarf::load(load_section) {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };
        let ctx = match addr2line::Context::from_dwarf(dwarf) {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };
        Ok(Some(Self { ctx }))
    }

    /// Resolve an address to a source location + function name.
    pub fn find_location(&self, address: u64) -> Option<SourceLocation> {
        // Non-split DWARF (Keil/IAR/GCC default): skip_all_loads is safe.
        let mut frames = match self.ctx.find_frames(address).skip_all_loads() {
            Ok(f) => f,
            Err(_) => return None,
        };
        let frame = match frames.next() {
            Ok(Some(f)) => f,
            _ => return None,
        };
        let function = frame.function.map(|f| {
            f.demangle()
                .unwrap_or_else(|_| f.raw_name().unwrap_or_default())
                .to_string()
        });
        let loc = frame.location?;
        Some(SourceLocation {
            file: loc.file.unwrap_or("<unknown>").to_string(),
            line: loc.line.unwrap_or(0) as u64,
            column: loc.column.map(|c| c as u64),
            function,
        })
    }
}
