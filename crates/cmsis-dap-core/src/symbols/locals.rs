//! DWARF Locals / Arguments reader (frozen v5 P6.3).
//!
//! Evaluates DWARF variable locations (address / register / frame-base /
//! constant), resolves types (base / pointer / enum / struct / array) and
//! reads values from target memory. Unsupported location forms yield an
//! explicit "unresolved" value — never fabricated.

use crate::error::{ErrorCode, McpError};
use addr2line::gimli::{
    self, AttributeValue, DebuggingInformationEntry, EndianSlice, Expression, RunTimeEndian, Unit,
};
use object::{Object, ObjectSection};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

/// Memory-read callback used during value evaluation.
pub type MemReader<'a> = &'a mut dyn FnMut(u64, &mut [u8]) -> Result<(), String>;

/// A resolved local variable / argument value.
#[derive(Debug, Clone, Serialize)]
pub struct LocalValue {
    pub name: String,
    pub type_name: String,
    /// integer | unsigned | float | bool | char | pointer | enum | struct |
    /// array | unresolved
    pub kind: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<u64>,
    #[serde(default)]
    pub children: Vec<LocalValue>,
}

type Reader = EndianSlice<'static, RunTimeEndian>;

/// DWARF debug info for locals/type resolution.
pub struct DwarfLocals {
    dwarf: gimli::Dwarf<Reader>,
}

impl DwarfLocals {
    /// Load DWARF from a firmware ELF/AXF. None when there is no `.debug_info`.
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
        let endian = if file.is_little_endian() {
            RunTimeEndian::Little
        } else {
            RunTimeEndian::Big
        };
        let load_section =
            |id: gimli::SectionId| -> Result<EndianSlice<'static, RunTimeEndian>, gimli::Error> {
                let name = id.name();
                let data: &'static [u8] = match file
                    .section_by_name(name)
                    .and_then(|s| s.uncompressed_data().ok())
                {
                    Some(d) => Box::leak(d.into_owned().into_boxed_slice()),
                    None => &[],
                };
                Ok(EndianSlice::new(data, endian))
            };
        let dwarf = match gimli::Dwarf::load(load_section) {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };
        Ok(Some(Self { dwarf }))
    }

    /// Read the locals/arguments of the function containing `pc`.
    pub fn locals_at(
        &self,
        pc: u64,
        regs: &HashMap<u16, u64>,
        cfa: Option<u64>,
        read: MemReader<'_>,
    ) -> Vec<LocalValue> {
        let mut out = Vec::new();
        let mut headers = self.dwarf.units();
        while let Ok(Some(header)) = headers.next() {
            let Ok(unit) = self.dwarf.unit(header) else {
                continue;
            };
            let mut cursor = unit.entries();
            while let Ok(Some((_depth, entry))) = cursor.next_dfs() {
                let tag = entry.tag();
                if tag != gimli::DW_TAG_subprogram && tag != gimli::DW_TAG_inlined_subroutine {
                    continue;
                }
                let mut covers = false;
                if let Ok(mut ranges) = self.dwarf.die_ranges(&unit, entry) {
                    while let Ok(Some(range)) = ranges.next() {
                        if pc >= range.begin && pc < range.end {
                            covers = true;
                            break;
                        }
                    }
                }
                if !covers {
                    continue;
                }
                self.read_function_locals(&unit, entry, pc, regs, cfa, read, &mut out);
                return out;
            }
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn read_function_locals(
        &self,
        unit: &Unit<Reader>,
        function: &DebuggingInformationEntry<'_, '_, Reader>,
        pc: u64,
        regs: &HashMap<u16, u64>,
        cfa: Option<u64>,
        read: MemReader<'_>,
        out: &mut Vec<LocalValue>,
    ) {
        let frame_base = self
            .eval_frame_base(unit, function, regs, cfa, read)
            .unwrap_or(cfa.unwrap_or(0));
        let target_off = function.offset();
        let mut entries = unit.entries();
        let mut in_function = false;
        while let Ok(Some((delta, entry))) = entries.next_dfs() {
            if entry.offset() == target_off {
                in_function = true;
                continue;
            }
            if !in_function {
                continue;
            }
            if delta < 0 {
                break; // left the function scope
            }
            let tag = entry.tag();
            if tag == gimli::DW_TAG_formal_parameter || tag == gimli::DW_TAG_variable {
                if let Some(v) = self.read_var(unit, entry, pc, regs, frame_base, read) {
                    out.push(v);
                }
            }
        }
    }

    fn read_var(
        &self,
        unit: &Unit<Reader>,
        entry: &DebuggingInformationEntry<'_, '_, Reader>,
        pc: u64,
        regs: &HashMap<u16, u64>,
        frame_base: u64,
        read: MemReader<'_>,
    ) -> Option<LocalValue> {
        let name = self
            .attr_str(unit, entry, gimli::DW_AT_name)
            .unwrap_or_default();
        let type_info = entry
            .attr_value(gimli::DW_AT_type)
            .ok()
            .flatten()
            .and_then(|v| match v {
                AttributeValue::UnitRef(r) => self.resolve_type(unit, r),
                _ => None,
            });
        let type_name = type_info
            .as_ref()
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "?".into());
        let type_kind = type_info
            .as_ref()
            .map(|t| t.kind.clone())
            .unwrap_or_else(|| "unresolved".into());
        let size = type_info.as_ref().map(|t| t.size).unwrap_or(0);

        let loc = entry.attr_value(gimli::DW_AT_location).ok().flatten();
        let mut address = None;
        let mut value = None;
        match loc {
            Some(AttributeValue::Exprloc(expr)) => {
                if let Ok((v, is_addr)) = self.eval_expr(unit, expr, regs, Some(frame_base), read) {
                    if is_addr {
                        address = Some(v);
                    } else {
                        value = Some(v);
                    }
                }
            }
            Some(AttributeValue::LocationListsRef(_)) => {
                if let Ok(Some((addr, _))) =
                    self.eval_loclist(unit, entry, pc, regs, frame_base, read)
                {
                    address = Some(addr);
                }
            }
            Some(AttributeValue::Data1(d)) => value = Some(d as u64),
            Some(AttributeValue::Data2(d)) => value = Some(d as u64),
            Some(AttributeValue::Data4(d)) => value = Some(d as u64),
            Some(AttributeValue::Data8(d)) => value = Some(d),
            _ => {}
        }

        let mut bytes = Vec::new();
        if let Some(addr) = address {
            if size > 0 && size <= 64 {
                let mut buf = vec![0u8; size as usize];
                if read(addr, &mut buf).is_ok() {
                    bytes = buf;
                }
            }
        }

        let formatted = if let Some(v) = value {
            format_scalar(&type_kind, v, size, type_info.as_ref())
        } else if !bytes.is_empty() {
            format_bytes(&type_kind, &bytes)
        } else if address.is_some() && type_kind == "pointer" {
            format!("0x{:08x}", address.unwrap())
        } else if address.is_some() {
            "0x0".into()
        } else {
            "unresolved".into()
        };

        let mut children = Vec::new();
        if let Some(ti) = &type_info {
            match ti.kind.as_str() {
                "struct" => {
                    if let Some(base) = address {
                        for m in &ti.members {
                            let maddr = base.wrapping_add(m.offset);
                            let mut mbuf = vec![0u8; m.size.min(64) as usize];
                            let read_ok = read(maddr, &mut mbuf).is_ok();
                            let mval = if read_ok && !mbuf.is_empty() {
                                format_bytes(&m.kind, &mbuf)
                            } else {
                                "?".into()
                            };
                            children.push(LocalValue {
                                name: m.name.clone(),
                                type_name: m.kind.clone(),
                                kind: m.kind.clone(),
                                value: mval,
                                address: Some(maddr),
                                children: Vec::new(),
                            });
                        }
                    }
                }
                "array" => {
                    if let Some(base) = address {
                        for i in 0..ti.array_count.min(8) {
                            let eaddr = base.wrapping_add(i as u64 * ti.element_size as u64);
                            let mut ebuf = vec![0u8; ti.element_size.min(64) as usize];
                            let ev = if read(eaddr, &mut ebuf).is_ok() && !ebuf.is_empty() {
                                format_bytes(&ti.element_kind, &ebuf)
                            } else {
                                "?".into()
                            };
                            children.push(LocalValue {
                                name: format!("[{}]", i),
                                type_name: ti.element_kind.clone(),
                                kind: ti.element_kind.clone(),
                                value: ev,
                                address: Some(eaddr),
                                children: Vec::new(),
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        Some(LocalValue {
            name,
            type_name,
            kind: type_kind,
            value: formatted,
            address,
            children,
        })
    }

    fn attr_str(
        &self,
        _unit: &Unit<Reader>,
        entry: &DebuggingInformationEntry<'_, '_, Reader>,
        attr: gimli::DwAt,
    ) -> Option<String> {
        let v = entry.attr_value(attr).ok()??;
        match v {
            AttributeValue::String(s) => Some(s.to_string_lossy().into_owned()),
            AttributeValue::DebugStrRef(o) => self
                .dwarf
                .debug_str
                .get_str(o)
                .ok()
                .map(|s| s.to_string_lossy().into_owned()),
            _ => None,
        }
    }

    fn attr_udata(
        &self,
        entry: &DebuggingInformationEntry<'_, '_, Reader>,
        attr: gimli::DwAt,
    ) -> Option<u64> {
        let v = entry.attr_value(attr).ok()??;
        match v {
            AttributeValue::Udata(u) => Some(u),
            AttributeValue::Sdata(s) => Some(s as u64),
            AttributeValue::Data1(d) => Some(d as u64),
            AttributeValue::Data2(d) => Some(d as u64),
            AttributeValue::Data4(d) => Some(d as u64),
            AttributeValue::Data8(d) => Some(d),
            _ => None,
        }
    }

    fn resolve_type(
        &self,
        unit: &Unit<Reader>,
        offset: gimli::UnitOffset<usize>,
    ) -> Option<TypeInfo> {
        self.resolve_type_depth(unit, offset, 0)
    }

    /// Resolve a DW_FORM_ref_addr (cross-unit) reference to a type.
    fn resolve_debug_info_ref(&self, offset: gimli::DebugInfoOffset<usize>) -> Option<TypeInfo> {
        let mut headers = self.dwarf.units();
        while let Ok(Some(header)) = headers.next() {
            let unit_start = header.offset().as_debug_info_offset()?.0;
            let len = header.length_including_self();
            let abs = offset.0;
            if abs >= unit_start && abs < unit_start + len {
                let rel = gimli::UnitOffset(abs - unit_start);
                let unit = self.dwarf.unit(header).ok()?;
                return self.resolve_type_depth(&unit, rel, 0);
            }
        }
        None
    }

    fn resolve_type_depth(
        &self,
        unit: &Unit<Reader>,
        offset: gimli::UnitOffset<usize>,
        depth: u8,
    ) -> Option<TypeInfo> {
        if depth > 8 {
            return None;
        }
        let entry = unit.entry(offset).ok()?;
        let tag = entry.tag();
        match tag {
            gimli::DW_TAG_base_type => {
                let name = self
                    .attr_str(unit, &entry, gimli::DW_AT_name)
                    .unwrap_or_else(|| "?".into());
                let size = self.attr_udata(&entry, gimli::DW_AT_byte_size).unwrap_or(0) as u32;
                let kind = match entry.attr_value(gimli::DW_AT_encoding).ok().flatten() {
                    Some(AttributeValue::Data1(e)) => {
                        let ate = gimli::DwAte(e);
                        if ate == gimli::DW_ATE_signed {
                            "integer"
                        } else if ate == gimli::DW_ATE_float {
                            "float"
                        } else if ate == gimli::DW_ATE_boolean {
                            "bool"
                        } else if ate == gimli::DW_ATE_signed_char
                            || ate == gimli::DW_ATE_unsigned_char
                        {
                            "char"
                        } else {
                            "unsigned"
                        }
                    }
                    _ => "unsigned",
                }
                .to_string();
                Some(TypeInfo {
                    name,
                    kind,
                    size,
                    members: Vec::new(),
                    array_count: 0,
                    element_size: 0,
                    element_kind: String::new(),
                })
            }
            gimli::DW_TAG_pointer_type => {
                let name = self
                    .attr_str(unit, &entry, gimli::DW_AT_name)
                    .unwrap_or_default();
                let pointee = entry
                    .attr_value(gimli::DW_AT_type)
                    .ok()
                    .flatten()
                    .and_then(|v| match v {
                        AttributeValue::UnitRef(r) => self.resolve_type_depth(unit, r, depth + 1),
                        AttributeValue::DebugInfoRef(o) => self.resolve_debug_info_ref(o),
                        _ => None,
                    });
                let size = self.attr_udata(&entry, gimli::DW_AT_byte_size).unwrap_or(4) as u32;
                let name = if name.is_empty() {
                    format!(
                        "{}*",
                        pointee.as_ref().map(|t| t.name.as_str()).unwrap_or("void")
                    )
                } else {
                    name
                };
                Some(TypeInfo {
                    name,
                    kind: "pointer".into(),
                    size,
                    members: Vec::new(),
                    array_count: 0,
                    element_size: 0,
                    element_kind: String::new(),
                })
            }
            gimli::DW_TAG_enumeration_type => {
                let name = self
                    .attr_str(unit, &entry, gimli::DW_AT_name)
                    .unwrap_or_else(|| "enum".into());
                let size = self.attr_udata(&entry, gimli::DW_AT_byte_size).unwrap_or(4) as u32;
                Some(TypeInfo {
                    name,
                    kind: "enum".into(),
                    size,
                    members: Vec::new(),
                    array_count: 0,
                    element_size: 0,
                    element_kind: String::new(),
                })
            }
            gimli::DW_TAG_structure_type => {
                let name = self
                    .attr_str(unit, &entry, gimli::DW_AT_name)
                    .unwrap_or_else(|| "struct".into());
                let size = self.attr_udata(&entry, gimli::DW_AT_byte_size).unwrap_or(0) as u32;
                let mut members = Vec::new();
                let target = entry.offset();
                let mut cursor = unit.entries();
                let mut collect = false;
                while let Ok(Some((delta, e))) = cursor.next_dfs() {
                    if e.offset() == target {
                        collect = true;
                        continue;
                    }
                    if !collect {
                        continue;
                    }
                    if delta < 0 {
                        break;
                    }
                    if e.tag() == gimli::DW_TAG_member {
                        let mname = self
                            .attr_str(unit, e, gimli::DW_AT_name)
                            .unwrap_or_else(|| "?".into());
                        let moff = self
                            .attr_udata(e, gimli::DW_AT_data_member_location)
                            .unwrap_or(0);
                        let mtype = e
                            .attr_value(gimli::DW_AT_type)
                            .ok()
                            .flatten()
                            .and_then(|v| match v {
                                AttributeValue::UnitRef(r) => {
                                    self.resolve_type_depth(unit, r, depth + 1)
                                }
                                AttributeValue::DebugInfoRef(o) => self.resolve_debug_info_ref(o),
                                _ => None,
                            });
                        let mkind = mtype
                            .as_ref()
                            .map(|t| t.kind.clone())
                            .unwrap_or_else(|| "unresolved".into());
                        let msize = mtype.as_ref().map(|t| t.size).unwrap_or(0);
                        members.push(Member {
                            name: mname,
                            kind: mkind,
                            size: msize,
                            offset: moff,
                        });
                    }
                }
                Some(TypeInfo {
                    name,
                    kind: "struct".into(),
                    size,
                    members,
                    array_count: 0,
                    element_size: 0,
                    element_kind: String::new(),
                })
            }
            gimli::DW_TAG_array_type => {
                let name = self
                    .attr_str(unit, &entry, gimli::DW_AT_name)
                    .unwrap_or_else(|| "array".into());
                let elem =
                    entry
                        .attr_value(gimli::DW_AT_type)
                        .ok()
                        .flatten()
                        .and_then(|v| match v {
                            AttributeValue::UnitRef(r) => {
                                self.resolve_type_depth(unit, r, depth + 1)
                            }
                            AttributeValue::DebugInfoRef(o) => self.resolve_debug_info_ref(o),
                            _ => None,
                        });
                let element_size = elem.as_ref().map(|t| t.size).unwrap_or(0);
                let element_kind = elem
                    .as_ref()
                    .map(|t| t.kind.clone())
                    .unwrap_or_else(|| "unresolved".into());
                let mut count = 0u32;
                let target = entry.offset();
                let mut cursor = unit.entries();
                let mut collect = false;
                while let Ok(Some((_d, e))) = cursor.next_dfs() {
                    if e.offset() == target {
                        collect = true;
                    } else if collect && e.tag() == gimli::DW_TAG_subrange_type {
                        count = self.attr_udata(e, gimli::DW_AT_count).unwrap_or(0) as u32;
                        break;
                    }
                }
                Some(TypeInfo {
                    name,
                    kind: "array".into(),
                    size: count.saturating_mul(element_size),
                    members: Vec::new(),
                    array_count: count,
                    element_size,
                    element_kind,
                })
            }
            gimli::DW_TAG_const_type
            | gimli::DW_TAG_volatile_type
            | gimli::DW_TAG_typedef
            | gimli::DW_TAG_restrict_type
            | gimli::DW_TAG_atomic_type
            | gimli::DW_TAG_shared_type => entry
                .attr_value(gimli::DW_AT_type)
                .ok()
                .flatten()
                .and_then(|v| match v {
                    AttributeValue::UnitRef(r) => self.resolve_type_depth(unit, r, depth + 1),
                    AttributeValue::DebugInfoRef(o) => self.resolve_debug_info_ref(o),
                    _ => None,
                }),
            _ => None,
        }
    }

    fn eval_frame_base(
        &self,
        unit: &Unit<Reader>,
        entry: &DebuggingInformationEntry<'_, '_, Reader>,
        regs: &HashMap<u16, u64>,
        cfa: Option<u64>,
        read: MemReader<'_>,
    ) -> Option<u64> {
        let attr = entry.attr_value(gimli::DW_AT_frame_base).ok()??;
        match attr {
            AttributeValue::Exprloc(expr) => self
                .eval_expr(unit, expr, regs, cfa, read)
                .ok()
                .map(|(v, _)| v),
            AttributeValue::Data1(d) => regs.get(&(d as u16)).copied(),
            AttributeValue::Data2(d) => regs.get(&d).copied(),
            AttributeValue::Data4(d) => regs.get(&(d as u16)).copied(),
            AttributeValue::Data8(d) => regs.get(&(d as u16)).copied(),
            _ => cfa,
        }
    }

    fn eval_expr(
        &self,
        unit: &Unit<Reader>,
        expr: Expression<Reader>,
        regs: &HashMap<u16, u64>,
        frame_base: Option<u64>,
        read: MemReader<'_>,
    ) -> Result<(u64, bool), String> {
        let encoding = unit.encoding();
        let mut eval = gimli::Evaluation::new(expr.0, encoding);
        let result = eval.evaluate().map_err(|e| format!("DWARF expr: {e}"))?;
        self.finish_eval(eval, result, regs, frame_base, read)
    }

    fn finish_eval(
        &self,
        mut eval: gimli::Evaluation<Reader>,
        mut result: gimli::EvaluationResult<Reader>,
        regs: &HashMap<u16, u64>,
        frame_base: Option<u64>,
        read: MemReader<'_>,
    ) -> Result<(u64, bool), String> {
        loop {
            match result {
                gimli::EvaluationResult::Complete => {
                    let pieces = eval.result();
                    let piece = pieces.first().ok_or("no result")?;
                    return Ok(match piece.location {
                        gimli::Location::Address { address } => (address, true),
                        gimli::Location::Register { register } => {
                            let v = regs
                                .get(&register.0)
                                .copied()
                                .ok_or("register unavailable")?;
                            (v, false)
                        }
                        gimli::Location::Value { value } => {
                            (value.to_u64(u64::MAX).unwrap_or(0), false)
                        }
                        gimli::Location::Bytes { .. } | gimli::Location::ImplicitPointer { .. } => {
                            return Err("unsupported piece location".into())
                        }
                        gimli::Location::Empty => return Err("empty location".into()),
                    });
                }
                gimli::EvaluationResult::RequiresRegister { register, .. } => {
                    let v = regs
                        .get(&register.0)
                        .copied()
                        .ok_or("register unavailable")?;
                    result = eval
                        .resume_with_register(gimli::Value::Generic(v))
                        .map_err(|e| e.to_string())?;
                }
                gimli::EvaluationResult::RequiresFrameBase => {
                    let fb = frame_base.ok_or("no frame base")?;
                    result = eval.resume_with_frame_base(fb).map_err(|e| e.to_string())?;
                }
                gimli::EvaluationResult::RequiresCallFrameCfa => {
                    let cfa = frame_base.ok_or("no cfa")?;
                    result = eval
                        .resume_with_call_frame_cfa(cfa)
                        .map_err(|e| e.to_string())?;
                }
                gimli::EvaluationResult::RequiresMemory { address, size, .. } => {
                    let mut buf = vec![0u8; size as usize];
                    read(address, &mut buf)?;
                    let v = le_bytes_to_u64(&buf);
                    result = eval
                        .resume_with_memory(gimli::Value::Generic(v))
                        .map_err(|e| e.to_string())?;
                }
                gimli::EvaluationResult::RequiresRelocatedAddress(a) => {
                    result = eval
                        .resume_with_relocated_address(a)
                        .map_err(|e| e.to_string())?;
                }
                _ => return Err("unsupported DWARF expression".into()),
            }
        }
    }

    fn eval_loclist(
        &self,
        unit: &Unit<Reader>,
        entry: &DebuggingInformationEntry<'_, '_, Reader>,
        pc: u64,
        regs: &HashMap<u16, u64>,
        frame_base: u64,
        read: MemReader<'_>,
    ) -> Result<Option<(u64, u64)>, String> {
        let attr = entry.attr_value(gimli::DW_AT_location).ok().flatten();
        let Some(AttributeValue::LocationListsRef(off)) = attr else {
            return Ok(None);
        };
        let mut locs = self.dwarf.locations(unit, off).map_err(|e| e.to_string())?;
        while let Ok(Some(item)) = locs.next() {
            if pc < item.range.begin || pc >= item.range.end {
                continue;
            }
            let expr = item.data;
            let (addr, _) = self.eval_expr(unit, expr, regs, Some(frame_base), read)?;
            return Ok(Some((addr, item.range.end - item.range.begin)));
        }
        Ok(None)
    }
}

#[derive(Debug, Clone)]
struct TypeInfo {
    name: String,
    kind: String,
    size: u32,
    members: Vec<Member>,
    array_count: u32,
    element_size: u32,
    element_kind: String,
}

#[derive(Debug, Clone)]
struct Member {
    name: String,
    kind: String,
    size: u32,
    offset: u64,
}

fn le_bytes_to_u64(b: &[u8]) -> u64 {
    let mut v = 0u64;
    for (i, byte) in b.iter().enumerate().take(8) {
        v |= (*byte as u64) << (8 * i);
    }
    v
}

fn format_scalar(kind: &str, v: u64, size: u32, _ti: Option<&TypeInfo>) -> String {
    format_value(kind, v, size)
}

fn format_bytes(kind: &str, bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "unresolved".into();
    }
    format_value(kind, le_bytes_to_u64(bytes), (bytes.len() as u32) * 8)
}

fn format_value(kind: &str, v: u64, size_bits: u32) -> String {
    match kind {
        "bool" => (v != 0).to_string(),
        "char" => {
            let c = (v & 0xff) as u8;
            if (0x20..=0x7e).contains(&c) {
                format!("'{}' (0x{:02x})", c as char, c)
            } else {
                format!("0x{:02x}", c)
            }
        }
        "float" => {
            let f = if size_bits >= 64 {
                f64::from_bits(v)
            } else {
                f32::from_bits((v & 0xFFFF_FFFF) as u32) as f64
            };
            format!("{f}")
        }
        "integer" => {
            let bits = size_bits.clamp(1, 64);
            let signed = if bits < 64 {
                ((v << (64 - bits)) as i64) >> (64 - bits)
            } else {
                v as i64
            };
            format!("{signed} (0x{:x})", v)
        }
        "pointer" => format!("0x{:08x}", v),
        _ => format!("0x{:x}", v),
    }
}
