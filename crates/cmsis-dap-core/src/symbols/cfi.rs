//! Minimal `.debug_frame` CFI reader/interpreter (DWARF Call Frame Information).
//!
//! Supports the standard DW_CFA instruction set plus ARM Compiler's
//! `armcc`/`armcc+` CIE augmentation (4 extra bytes after the return-address
//! register), which upstream gimli rejects. Used by the real call-stack
//! unwinder (frozen v5 P6.2) — never fabricated frames.

use crate::error::{ErrorCode, McpError};
use serde::Serialize;
use std::collections::HashMap;

/// Register recovery rule for one register in a CFI row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RegRule {
    Undefined,
    SameValue,
    Offset(i64),
    ValOffset(i64),
    Register(u16),
    Expression,
}

/// A CFI row: how to recover the caller frame from the current PC.
#[derive(Debug, Clone, Default)]
pub struct Row {
    /// (register, offset) CFA rule; None = expression (unsupported).
    pub cfa: Option<(u16, i64)>,
    pub registers: HashMap<u16, RegRule>,
    /// Address range this row applies to (used during evaluation).
    pub start: u64,
    pub end: u64,
}

impl Row {
    pub fn register(&self, reg: u16) -> &RegRule {
        self.registers.get(&reg).unwrap_or(&RegRule::Undefined)
    }
}

struct Cie {
    code_align: u64,
    data_align: i64,
    instructions: Vec<u8>,
}

struct Fde {
    initial: u64,
    range: u64,
    cie: usize,
    instructions: Vec<u8>,
    cie_offset: usize,
}

/// A parsed `.debug_frame` section.
pub struct DebugFrameCfi {
    cies: Vec<Cie>,
    fdes: Vec<Fde>,
}

impl DebugFrameCfi {
    /// Parse a `.debug_frame` section (bytes + little/big endian).
    pub fn parse(data: &[u8], little_endian: bool) -> Result<Self, McpError> {
        let mut cies = Vec::new();
        let mut fdes = Vec::new();
        let mut off = 0usize;
        while off + 8 <= data.len() {
            let length = read_u32(data, off, little_endian) as usize;
            if length == 0 || length == 0xFFFF_FFFF {
                break;
            }
            let id = read_u32(data, off + 4, little_endian);
            let body_start = off + 8;
            let body_end = off + 4 + length;
            if body_end > data.len() {
                break;
            }
            if id == 0xFFFF_FFFF {
                // CIE (in .debug_frame the CIE id is 0xffffffff for v3+).
                let p = parse_cie(data, body_start, body_end, little_endian)?;
                cies.push(Cie {
                    code_align: p.code_align,
                    data_align: p.data_align,
                    instructions: p.instructions,
                });
            } else {
                // FDE: id is the CIE offset from the start of the section.
                let p = parse_fde(data, body_start, body_end, little_endian, id as usize)?;
                fdes.push(Fde {
                    initial: p.initial,
                    range: p.range,
                    cie: 0, // resolved after all CIE offsets known
                    instructions: p.instructions,
                    cie_offset: id as usize,
                });
            }
            off = body_end;
        }
        // Resolve FDE -> CIE by offset.
        let cie_offsets = compute_cie_offsets(data, little_endian);
        for fde in fdes.iter_mut() {
            if let Some(idx) = cie_offsets.get(&fde.cie_offset) {
                fde.cie = *idx;
            }
        }
        fdes.retain(|f| f.cie < cies.len());
        Ok(Self { cies, fdes })
    }

    /// Number of parsed FDEs.
    pub fn fde_count(&self) -> usize {
        self.fdes.len()
    }

    /// Find the CFI row for `address`, or None when there is no unwind info.
    pub fn row_for_address(&self, address: u64) -> Option<Row> {
        let fde = self
            .fdes
            .iter()
            .find(|f| address >= f.initial && address < f.initial + f.range)?;
        let cie = self.cies.get(fde.cie)?;
        let mut row = Row::default();
        evaluate(&cie.instructions, cie, 0, &mut row);
        row.start = fde.initial;
        row.end = fde.initial + fde.range;
        evaluate_fde(fde, cie, address, &mut row);
        Some(row)
    }
}

struct ParsedCie {
    code_align: u64,
    data_align: i64,
    instructions: Vec<u8>,
}

struct ParsedFde {
    initial: u64,
    range: u64,
    instructions: Vec<u8>,
}

fn read_u32(d: &[u8], o: usize, le: bool) -> u32 {
    if le {
        u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
    } else {
        u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
    }
}

fn read_u8(d: &[u8], o: usize) -> u8 {
    d[o]
}

fn uleb(d: &[u8], o: &mut usize) -> Result<u64, McpError> {
    let mut result = 0u64;
    let mut shift = 0u32;
    loop {
        if *o >= d.len() {
            return Err(McpError::new(ErrorCode::FileError, "CFI: truncated ULEB"));
        }
        let b = d[*o];
        *o += 1;
        result |= ((b & 0x7f) as u64) << shift;
        if b & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift > 63 {
            return Err(McpError::new(ErrorCode::FileError, "CFI: ULEB overflow"));
        }
    }
    Ok(result)
}

fn sleb(d: &[u8], o: &mut usize) -> Result<i64, McpError> {
    let mut result = 0i64;
    let mut shift = 0u32;
    loop {
        if *o >= d.len() {
            return Err(McpError::new(ErrorCode::FileError, "CFI: truncated SLEB"));
        }
        let b = d[*o];
        *o += 1;
        result |= ((b & 0x7f) as i64) << shift;
        shift += 7;
        if b & 0x80 == 0 {
            if shift < 64 && (b & 0x40) != 0 {
                result |= -1i64 << shift;
            }
            break;
        }
        if shift >= 64 {
            return Err(McpError::new(ErrorCode::FileError, "CFI: SLEB overflow"));
        }
    }
    Ok(result)
}

fn parse_cie(d: &[u8], start: usize, end: usize, _le: bool) -> Result<ParsedCie, McpError> {
    let mut o = start;
    let version = read_u8(d, o);
    o += 1;
    // augmentation string
    let aug_start = o;
    while o < end && d[o] != 0 {
        o += 1;
    }
    if o >= end {
        return Err(McpError::new(
            ErrorCode::FileError,
            "CFI: CIE augmentation not terminated",
        ));
    }
    let aug = String::from_utf8_lossy(&d[aug_start..o]).to_string();
    o += 1;
    if version >= 4 {
        // address_size + segment_selector_size
        o += 2;
    }
    let code_align = uleb(d, &mut o)?;
    let data_align = sleb(d, &mut o)?;
    let _ret_addr_reg = uleb(d, &mut o)? as u16;
    // ARM Compiler augmentation: 4 extra bytes after the return-address register.
    if aug.starts_with("armcc") {
        o += 4;
    }
    Ok(ParsedCie {
        code_align,
        data_align,
        instructions: d[o..end.min(o + 4096)].to_vec(),
    })
}

fn parse_fde(
    d: &[u8],
    start: usize,
    end: usize,
    le: bool,
    _cie_offset: usize,
) -> Result<ParsedFde, McpError> {
    let mut o = start;
    let initial = read_u32(d, o, le) as u64;
    o += 4;
    let range = read_u32(d, o, le) as u64;
    o += 4;
    Ok(ParsedFde {
        initial,
        range,
        instructions: d[o..end.min(o + 8192)].to_vec(),
    })
}

/// Compute the section offsets of each CIE (to resolve FDE -> CIE pointers).
fn compute_cie_offsets(d: &[u8], le: bool) -> HashMap<usize, usize> {
    let mut map = HashMap::new();
    let mut idx = 0usize;
    let mut off = 0usize;
    while off + 8 <= d.len() {
        let length = read_u32(d, off, le) as usize;
        if length == 0 || length == 0xFFFF_FFFF {
            break;
        }
        let id = read_u32(d, off + 4, le);
        if id == 0xFFFF_FFFF {
            map.insert(off, idx);
            idx += 1;
        }
        off += 4 + length;
    }
    map
}

fn evaluate(ins: &[u8], cie: &Cie, _start: u64, row: &mut Row) {
    let mut o = 0usize;
    while o < ins.len() {
        let op = ins[o];
        o += 1;
        match op {
            0x00 => {} // nop
            0x01 => {
                o = o.saturating_add(4); // set_loc
            }
            0x02..=0x04 => {
                o += 1 << (op - 0x02); // advance_loc1/2/4
            }
            0x05 => {
                let _reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                let off = uleb(ins, &mut o).unwrap_or(0) as i64;
                row.registers
                    .insert(_reg, RegRule::Offset(off * cie.data_align));
            }
            0x06 => {
                let _reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                row.registers.remove(&_reg);
            }
            0x07 => {
                let reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                row.registers.insert(reg, RegRule::Undefined);
            }
            0x08 => {
                let reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                row.registers.insert(reg, RegRule::SameValue);
            }
            0x09 => {
                let reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                let reg2 = uleb(ins, &mut o).unwrap_or(0) as u16;
                row.registers.insert(reg, RegRule::Register(reg2));
            }
            0x0a | 0x0b => {} // remember/restore state (ignored for simple rows)
            0x0c => {
                let reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                let off = uleb(ins, &mut o).unwrap_or(0) as i64;
                row.cfa = Some((reg, off));
            }
            0x0d => {
                if let Some((reg, off)) = row.cfa {
                    let new_reg = uleb(ins, &mut o).unwrap_or(reg as u64) as u16;
                    row.cfa = Some((new_reg, off));
                }
            }
            0x0e => {
                if let Some((reg, _)) = row.cfa {
                    let off = uleb(ins, &mut o).unwrap_or(0) as i64;
                    row.cfa = Some((reg, off));
                }
            }
            0x0f | 0x10 | 0x16 => {
                // expression-based rules: mark unsupported, skip the block
                if op != 0x0f {
                    let _reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                }
                let len = uleb(ins, &mut o).unwrap_or(0) as usize;
                o = (o + len).min(ins.len());
                row.cfa = row.cfa.take().or(Some((13, 0)));
            }
            0x11 => {
                let reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                let off = sleb(ins, &mut o).unwrap_or(0);
                row.registers
                    .insert(reg, RegRule::Offset(off * cie.data_align));
            }
            0x12 => {
                let reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                let off = sleb(ins, &mut o).unwrap_or(0);
                row.cfa = Some((reg, off * cie.data_align));
            }
            0x13 => {
                if let Some((reg, _)) = row.cfa {
                    let off = sleb(ins, &mut o).unwrap_or(0);
                    row.cfa = Some((reg, off * cie.data_align));
                }
            }
            0x14 => {
                let reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                let off = uleb(ins, &mut o).unwrap_or(0) as i64;
                row.registers
                    .insert(reg, RegRule::ValOffset(off * cie.data_align));
            }
            0x15 => {
                let reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                let off = sleb(ins, &mut o).unwrap_or(0);
                row.registers
                    .insert(reg, RegRule::ValOffset(off * cie.data_align));
            }
            _ => {
                // primary opcodes in the high bits
                match op & 0xc0 {
                    0x40 => {} // advance_loc
                    0x80 => {
                        let reg = (op & 0x3f) as u16;
                        let off = uleb(ins, &mut o).unwrap_or(0) as i64;
                        row.registers
                            .insert(reg, RegRule::Offset(off * cie.data_align));
                    }
                    0xc0 => {
                        let reg = (op & 0x3f) as u16;
                        row.registers.remove(&reg);
                    }
                    _ => break,
                }
            }
        }
    }
}

fn evaluate_fde(fde: &Fde, cie: &Cie, address: u64, row: &mut Row) {
    let ins = &fde.instructions;
    let mut o = 0usize;
    let mut loc = fde.initial;
    while o < ins.len() {
        let op = ins[o];
        o += 1;
        match op & 0xc0 {
            0x40 => {
                loc += (op & 0x3f) as u64 * cie.code_align;
                if loc > address {
                    break;
                }
            }
            0x80 => {
                let reg = (op & 0x3f) as u16;
                let off = uleb(ins, &mut o).unwrap_or(0) as i64;
                row.registers
                    .insert(reg, RegRule::Offset(off * cie.data_align));
            }
            0xc0 => {
                let reg = (op & 0x3f) as u16;
                row.registers.remove(&reg);
            }
            _ => match op {
                0x00 => {}
                0x01 => {
                    loc = read_u32(ins, o, true) as u64;
                    o += 4;
                }
                0x02 => {
                    loc += ins[o] as u64 * cie.code_align;
                    o += 1;
                }
                0x03 => {
                    loc += read_u32(ins, o, true) as u64 * cie.code_align;
                    o += 4;
                }
                0x04 => {
                    loc += read_u32(ins, o, true) as u64 * cie.code_align;
                    o += 4;
                }
                0x05 => {
                    let _reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                    let off = uleb(ins, &mut o).unwrap_or(0) as i64;
                    row.registers
                        .insert(_reg, RegRule::Offset(off * cie.data_align));
                }
                0x06 => {
                    let _reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                    row.registers.remove(&_reg);
                }
                0x07 => {
                    let reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                    row.registers.insert(reg, RegRule::Undefined);
                }
                0x08 => {
                    let reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                    row.registers.insert(reg, RegRule::SameValue);
                }
                0x09 => {
                    let reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                    let reg2 = uleb(ins, &mut o).unwrap_or(0) as u16;
                    row.registers.insert(reg, RegRule::Register(reg2));
                }
                0x0c => {
                    let reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                    let off = uleb(ins, &mut o).unwrap_or(0) as i64;
                    row.cfa = Some((reg, off));
                }
                0x0d => {
                    if let Some((reg, off)) = row.cfa {
                        let new_reg = uleb(ins, &mut o).unwrap_or(reg as u64) as u16;
                        row.cfa = Some((new_reg, off));
                    }
                }
                0x0e => {
                    if let Some((reg, _)) = row.cfa {
                        let off = uleb(ins, &mut o).unwrap_or(0) as i64;
                        row.cfa = Some((reg, off));
                    }
                }
                0x11 => {
                    let reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                    let off = sleb(ins, &mut o).unwrap_or(0);
                    row.registers
                        .insert(reg, RegRule::Offset(off * cie.data_align));
                }
                0x12 => {
                    let reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                    let off = sleb(ins, &mut o).unwrap_or(0);
                    row.cfa = Some((reg, off * cie.data_align));
                }
                0x14 => {
                    let reg = uleb(ins, &mut o).unwrap_or(0) as u16;
                    let off = uleb(ins, &mut o).unwrap_or(0) as i64;
                    row.registers
                        .insert(reg, RegRule::ValOffset(off * cie.data_align));
                }
                _ => break,
            },
        }
        if loc > address {
            break;
        }
    }
}
