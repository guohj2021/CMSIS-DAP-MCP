//! DWARF CFI call-stack unwinder (frozen v5 P6.2).
//!
//! Reads `.debug_frame` via the crate's own CFI interpreter (handles ARM
//! Compiler's `armcc` augmentation) and unwinds the Cortex-M stack from the
//! current register set. This is a real unwinder — never fabricated frames.
//! Frames that have no CFI simply stop the walk (honest partial stacks).

use crate::error::{ErrorCode, McpError};
use crate::symbols::cfi::{DebugFrameCfi, RegRule};
use crate::symbols::dwarf::{DebugInfo, SourceLocation};
use object::{Object, ObjectSection};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

/// One unwound call-stack frame, including the recovered register set so the
/// UI can evaluate locals / registers in that frame's context (v5 §9).
#[derive(Debug, Clone, Serialize)]
pub struct UnwindFrame {
    pub pc: u64,
    pub sp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceLocation>,
    /// DWARF register number -> value recovered for this frame.
    #[serde(default)]
    pub registers: HashMap<u16, u64>,
}

/// A real DWARF CFI unwinder loaded from a firmware ELF/AXF.
pub struct CfiUnwinder {
    cfi: DebugFrameCfi,
    debug_info: Option<DebugInfo>,
}

impl CfiUnwinder {
    /// Load `.debug_frame` + DWARF names. Returns None when the firmware has
    /// no usable CFI (callers report "not available", never fake).
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
        let Some(section) = file.section_by_name(".debug_frame") else {
            return Ok(None);
        };
        let Ok(data) = section.uncompressed_data() else {
            return Ok(None);
        };
        let data = data.into_owned();
        let cfi = match DebugFrameCfi::parse(&data, file.is_little_endian()) {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };
        if cfi.fde_count() == 0 {
            return Ok(None);
        }
        let debug_info = DebugInfo::load(path).ok().flatten();
        Ok(Some(Self { cfi, debug_info }))
    }

    /// Unwind from the given register set (keyed by DWARF register number).
    /// `read32` reads 32-bit words from target memory (the stack).
    /// `max_frames` caps the walk.
    pub fn unwind(
        &self,
        regs: &HashMap<u16, u64>,
        read32: &mut dyn FnMut(u64) -> Result<u32, String>,
        max_frames: usize,
    ) -> Vec<UnwindFrame> {
        let mut frames = Vec::new();
        let mut current: HashMap<u16, u64> = regs.clone();

        // Frame #0: the current PC where execution stopped.
        let pc0 = current.get(&15).copied().unwrap_or(0) & !1;
        if pc0 != 0 {
            let mut f = self.make_frame(pc0, current.get(&13).copied().unwrap_or(0));
            f.registers = current.clone();
            frames.push(f);
        }

        let mut guard = 0usize;
        while guard < max_frames {
            guard += 1;
            let pc = current.get(&15).copied().unwrap_or(0) & !1;
            if pc == 0 {
                break;
            }
            let Some(row) = self.cfi.row_for_address(pc) else {
                break; // no unwind info at this PC -> stop (honest)
            };
            let Some((cfa_reg, cfa_off)) = row.cfa else {
                break; // expression CFA not supported -> stop
            };
            let base = current.get(&cfa_reg).copied().unwrap_or(0);
            let cfa = base.wrapping_add_signed(cfa_off);

            let mut next: HashMap<u16, u64> = HashMap::new();
            next.insert(13, cfa); // SP = CFA
                                  // Callee-saved registers + return address + PC.
            for regno in [4u16, 5, 6, 7, 8, 9, 10, 11, 14, 15] {
                let value = match row.register(regno) {
                    RegRule::Offset(off) => {
                        let addr = cfa.wrapping_add_signed(*off);
                        read32(addr).ok().map(|v| v as u64)
                    }
                    RegRule::ValOffset(off) => Some(cfa.wrapping_add_signed(*off)),
                    RegRule::Register(r) => current.get(r).copied(),
                    RegRule::SameValue => current.get(&regno).copied(),
                    _ => None,
                };
                if let Some(v) = value {
                    next.insert(regno, v);
                }
            }

            // ARM: the return address is saved in LR (reg 14), not PC.
            let new_pc = next
                .get(&15)
                .copied()
                .or_else(|| next.get(&14).copied())
                .unwrap_or(0);
            let new_pc_aligned = new_pc & !1;
            if new_pc_aligned == 0 || new_pc_aligned == pc {
                break; // bottom of stack / loop guard
            }
            next.insert(15, new_pc);
            let sp = next.get(&13).copied().unwrap_or(cfa);
            let mut f = self.make_frame(new_pc_aligned, sp);
            f.registers = next.clone();
            frames.push(f);
            current = next;
        }
        frames
    }

    /// Compute the canonical frame address (CFA) for `pc` from the current
    /// registers, used by DWARF locals evaluation (`DW_OP_call_frame_cfa`).
    pub fn cfa_for(&self, pc: u64, regs: &HashMap<u16, u64>) -> Option<u64> {
        let row = self.cfi.row_for_address(pc & !1)?;
        let (reg, off) = row.cfa?;
        let base = regs.get(&reg).copied().unwrap_or(0);
        Some(base.wrapping_add_signed(off))
    }

    fn make_frame(&self, pc: u64, sp: u64) -> UnwindFrame {
        let (function, source) = match self.debug_info.as_ref().and_then(|di| di.find_location(pc))
        {
            Some(sl) => (
                Some(sl.function.clone().unwrap_or_default()).filter(|f| !f.is_empty()),
                Some(sl),
            ),
            None => (None, None),
        };
        UnwindFrame {
            pc,
            sp,
            function,
            source,
            registers: HashMap::new(),
        }
    }
}
