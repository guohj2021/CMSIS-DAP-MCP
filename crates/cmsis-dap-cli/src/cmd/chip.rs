//! Keil FLM -> probe-rs target YAML generation.
//!
//! An FLM is an ARM ELF containing the vendor flash programming algorithm
//! (code segment) plus a `FlashDevice` descriptor (usually in a separate data
//! segment). This module extracts the algorithm bytes, the entry-point
//! symbols and the descriptor fields, so a usable target YAML can be produced
//! from just the FLM plus the Flash/SRAM address ranges.

use super::CliError;
use base64::Engine as _;
use cmsis_dap_core::error::{ErrorCode, McpError};
use object::{Object, ObjectSection, ObjectSegment, ObjectSymbol, SymbolKind};
use std::collections::BTreeMap;

/// Parsed `FlashDevice` descriptor (Keil FLM layout).
pub struct FlashDevice {
    pub name: String,
    pub dev_addr: u64,
    pub flash_size: u64,
    pub page_size: u32,
    pub erased_value: u8,
    pub program_poll_time: u32,
    pub erase_sector_timeout: u32,
    pub sectors: Vec<u64>,
}

/// Everything extracted from an FLM file.
pub struct FlmParse {
    /// Virtual address of the code segment inside the ELF.
    pub load_address: u64,
    /// Algorithm blob: code followed by the data section.
    pub instructions: Vec<u8>,
    /// Offset from the code start to the static data base (R9).
    pub data_section_offset: u64,
    pub pc_init: Option<u64>,
    pub pc_uninit: Option<u64>,
    pub pc_program_page: Option<u64>,
    pub pc_erase_sector: Option<u64>,
    pub pc_erase_all: Option<u64>,
    pub pc_verify: Option<u64>,
    pub pc_blank_check: Option<u64>,
    pub device: FlashDevice,
    pub symbols: BTreeMap<String, u64>,
    /// (virtual address, memory size, file data length) per segment.
    pub segments: Vec<(u64, u64, usize)>,
}

fn file_error(msg: impl Into<String>) -> CliError {
    CliError::Mcp(McpError::new(ErrorCode::FileError, msg))
}

fn u32_at(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

/// Parse an FLM (ARM ELF) into the flash algorithm description.
pub fn parse_flm(bytes: &[u8]) -> Result<FlmParse, CliError> {
    let file = object::File::parse(bytes)
        .map_err(|e| file_error(format!("failed to parse FLM ELF: {e}")))?;

    // Collect loadable segments; fall back to allocated sections for ELF
    // files that carry the algorithm without program headers.
    let mut segments: Vec<(u64, u64, Vec<u8>)> = Vec::new(); // (address, size, data)
    for seg in file.segments() {
        if let Ok(data) = seg.data() {
            segments.push((seg.address(), seg.size(), data.to_vec()));
        }
    }
    if segments.is_empty() {
        for sec in file.sections() {
            if let Ok(data) = sec.data() {
                if !data.is_empty() {
                    segments.push((sec.address(), sec.size(), data.to_vec()));
                }
            }
        }
    }
    let segments_summary: Vec<(u64, u64, usize)> =
        segments.iter().map(|(a, s, d)| (*a, *s, d.len())).collect();

    let mut symbols = BTreeMap::new();
    let mut data_symbols: BTreeMap<String, u64> = BTreeMap::new();
    for sym in file.symbols() {
        let Ok(name) = sym.name() else {
            continue;
        };
        match sym.kind() {
            SymbolKind::Text => {
                symbols.insert(name.to_string(), sym.address());
            }
            SymbolKind::Data => {
                symbols.insert(name.to_string(), sym.address());
                data_symbols.insert(name.to_string(), sym.address());
            }
            _ => {}
        }
    }

    // The code segment is the one containing an entry-point symbol. FLMs
    // typically carry the code and the FlashDevice descriptor in separate
    // loadable segments.
    let entry_names = ["Init", "ProgramPage", "EraseSector", "EraseChip", "UnInit"];
    let code_idx = segments
        .iter()
        .position(|(addr, size, _)| {
            entry_names.iter().any(|n| {
                symbols
                    .get(*n)
                    .map(|a| {
                        let a2 = a & !1;
                        a2 >= *addr && a2 < addr + size
                    })
                    .unwrap_or(false)
            })
        })
        .or_else(|| {
            segments
                .iter()
                .enumerate()
                .max_by_key(|(_, (_, _, d))| d.len())
                .map(|(i, _)| i)
        })
        .ok_or_else(|| file_error("no loadable segment found in FLM"))?;
    let (code_addr, _, code_data) = &segments[code_idx];
    let load_address = *code_addr;

    // Entry-point offsets are relative to the load address. ARM symbols may
    // carry the Thumb bit in the low bit; keep it in the emitted offset.
    let rel = |addr: u64| -> u64 {
        let even = addr & !1u64;
        let thumb = addr & 1u64;
        even.wrapping_sub(load_address) | thumb
    };
    let pc_init = symbols.get("Init").map(|a| rel(*a));
    let pc_uninit = symbols.get("UnInit").map(|a| rel(*a));
    let pc_program_page = symbols.get("ProgramPage").map(|a| rel(*a));
    let pc_erase_sector = symbols.get("EraseSector").map(|a| rel(*a));
    let pc_erase_all = symbols
        .get("EraseAll")
        .or_else(|| symbols.get("EraseChip"))
        .map(|a| rel(*a));
    let pc_verify = symbols.get("Verify").map(|a| rel(*a));
    let pc_blank_check = symbols.get("BlankCheck").map(|a| rel(*a));

    let fd_addr = *symbols
        .get("FlashDevice")
        .ok_or_else(|| file_error("FLM has no FlashDevice symbol"))?;
    let fd_idx = segments
        .iter()
        .position(|(addr, size, _)| {
            let a = fd_addr & !1;
            a >= *addr && a < addr + size
        })
        .unwrap_or(code_idx);
    let (fd_seg_addr, _, fd_seg_data) = &segments[fd_idx];
    let fd_off = (fd_addr & !1)
        .checked_sub(*fd_seg_addr)
        .ok_or_else(|| file_error("FlashDevice symbol outside its segment"))?;
    let d = fd_seg_data
        .get(fd_off as usize..)
        .ok_or_else(|| file_error("FlashDevice descriptor outside segment data"))?;
    if d.len() < 0xA4 {
        return Err(file_error("FlashDevice descriptor too short"));
    }
    let name = String::from_utf8_lossy(&d[2..130])
        .trim_matches('\0')
        .trim()
        .to_string();
    // Empirically validated layout used by these flash algorithms:
    // Vers u16, DevName[128], DevType u16, DevAdr u32, FlashSize u32,
    // PageSize u32, reserved u32, ErasedVal u32, ProgPollTime u32,
    // EraseSectorTimeout u32, Sectors u32[] (0/0xFFFFFFFF terminated).
    let dev_addr = u32_at(d, 0x84) as u64;
    let flash_size = u32_at(d, 0x88) as u64;
    let page_size = u32_at(d, 0x8C);
    let erased_value = u32_at(d, 0x94) as u8;
    let program_poll_time = u32_at(d, 0x98);
    let erase_sector_timeout = u32_at(d, 0x9C);
    let mut sectors = Vec::new();
    let mut off = 0xA0usize;
    while off + 4 <= d.len() && sectors.len() < 512 {
        let v = u32_at(d, off) as u64;
        off += 4;
        if v == 0xFFFF_FFFF || v == 0 {
            break;
        }
        sectors.push(v);
    }

    // The static data base (R9) is the start of the data segment relative to
    // the code segment; when code and data share one segment, fall back to the
    // lowest data symbol inside the code segment, then to the descriptor.
    let data_section_offset = if fd_idx != code_idx {
        fd_seg_addr.wrapping_sub(load_address)
    } else {
        data_symbols
            .values()
            .map(|a| *a & !1)
            .filter(|a| *a >= load_address && *a < load_address + code_data.len() as u64)
            .map(|a| a.wrapping_sub(load_address))
            .min()
            .unwrap_or(fd_off)
    };

    // The algorithm blob is code followed by the data section (in virtual
    // address order), loaded as one block into RAM.
    let mut instructions = code_data.clone();
    for (i, (addr, _, data)) in segments.iter().enumerate() {
        if i != code_idx && *addr >= load_address {
            instructions.extend_from_slice(data);
        }
    }

    Ok(FlmParse {
        load_address,
        instructions,
        data_section_offset,
        pc_init,
        pc_uninit,
        pc_program_page,
        pc_erase_sector,
        pc_erase_all,
        pc_verify,
        pc_blank_check,
        device: FlashDevice {
            name,
            dev_addr,
            flash_size,
            page_size,
            erased_value,
            program_poll_time,
            erase_sector_timeout,
            sectors,
        },
        symbols,
        segments: segments_summary,
    })
}

fn hex(v: u64) -> String {
    format!("0x{v:x}")
}

/// Render a probe-rs target YAML from the parsed FLM and user-provided ranges.
///
/// probe-rs prepends an internal header to the algorithm, so the yaml
/// `load_address` is the SRAM start plus the header size (0x20); the code is
/// loaded at `load_address` and `data_section_offset` is relative to it.
pub fn generate_yaml(
    p: &FlmParse,
    name: &str,
    flash_start: u64,
    flash_end: u64,
    sram_start: u64,
    sram_end: u64,
    core: &str,
) -> Result<String, CliError> {
    const HEADER_SIZE: u64 = 0x20;
    let load_address = sram_start
        .checked_add(HEADER_SIZE)
        .ok_or_else(|| CliError::InvalidArgument("sram range overflow".into()))?;
    if load_address
        .checked_add(p.instructions.len() as u64)
        .map(|end| end > sram_end)
        .unwrap_or(true)
    {
        return Err(CliError::InvalidArgument(
            "SRAM region too small for the flash algorithm; increase --sram-size".into(),
        ));
    }

    let algo = format!("{}_flash", name.to_ascii_lowercase());
    let b64 = base64::engine::general_purpose::STANDARD.encode(&p.instructions);
    let sector = p
        .device
        .sectors
        .first()
        .copied()
        .unwrap_or(p.device.page_size as u64);
    let program_page_timeout = if p.device.program_poll_time > 0 {
        p.device.program_poll_time
    } else {
        100
    };
    let erase_sector_timeout = if p.device.erase_sector_timeout > 0 {
        p.device.erase_sector_timeout
    } else {
        6000
    };

    let mut out = String::new();
    out.push_str(&format!("name: {name}\n"));
    out.push_str("generated_from_pack: false\n");
    out.push_str("variants:\n");
    out.push_str(&format!("- name: {name}\n"));
    out.push_str("  cores:\n");
    out.push_str(&format!(
        "  - name: main\n    type: {core}\n    core_access_options: !Arm\n      ap: !v1 0\n"
    ));
    out.push_str("  memory_map:\n");
    out.push_str(&format!(
        "  - !Nvm\n    name: FLASH\n    range:\n      start: {}\n      end: {}\n    cores:\n    - main\n    access:\n      read: true\n      write: true\n      execute: true\n      boot: true\n",
        hex(flash_start),
        hex(flash_end)
    ));
    out.push_str(&format!(
        "  - !Ram\n    name: SRAM\n    range:\n      start: {}\n      end: {}\n    cores:\n    - main\n",
        hex(sram_start),
        hex(sram_end)
    ));
    out.push_str(&format!("  flash_algorithms:\n  - {algo}\n"));
    out.push_str("flash_algorithms:\n");
    out.push_str(&format!("- name: {algo}\n"));
    out.push_str(&format!("  description: {name} flash\n"));
    out.push_str("  default: true\n");
    out.push_str(&format!("  instructions: \"{b64}\"\n"));
    out.push_str(&format!("  load_address: {}\n", hex(load_address)));
    if let Some(v) = p.pc_init {
        out.push_str(&format!("  pc_init: {}\n", hex(v)));
    }
    if let Some(v) = p.pc_uninit {
        out.push_str(&format!("  pc_uninit: {}\n", hex(v)));
    }
    if let Some(v) = p.pc_program_page {
        out.push_str(&format!("  pc_program_page: {}\n", hex(v)));
    }
    if let Some(v) = p.pc_erase_sector {
        out.push_str(&format!("  pc_erase_sector: {}\n", hex(v)));
    }
    if let Some(v) = p.pc_erase_all {
        out.push_str(&format!("  pc_erase_all: {}\n", hex(v)));
    }
    if let Some(v) = p.pc_verify {
        out.push_str(&format!("  pc_verify: {}\n", hex(v)));
    }
    if let Some(v) = p.pc_blank_check {
        out.push_str(&format!("  pc_blank_check: {}\n", hex(v)));
    }
    out.push_str(&format!(
        "  data_section_offset: {}\n",
        hex(p.data_section_offset)
    ));
    out.push_str("  flash_properties:\n");
    out.push_str(&format!(
        "    address_range:\n      start: {}\n      end: {}\n",
        hex(flash_start),
        hex(flash_end)
    ));
    out.push_str(&format!(
        "    page_size: {}\n",
        hex(p.device.page_size as u64)
    ));
    out.push_str(&format!(
        "    erased_byte_value: {}\n",
        hex(p.device.erased_value as u64)
    ));
    out.push_str(&format!(
        "    program_page_timeout: {program_page_timeout}\n"
    ));
    out.push_str(&format!(
        "    erase_sector_timeout: {erase_sector_timeout}\n"
    ));
    out.push_str("    sectors:\n");
    out.push_str(&format!(
        "    - size: {}\n      address: 0x0\n",
        hex(sector)
    ));
    out.push_str("  transfer_encoding: raw\n");
    out.push_str("  cores:\n  - main\n");
    Ok(out)
}
