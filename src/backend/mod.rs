use crate::error::McpError;
pub mod mock;
pub mod probe_rs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessWidth {
    U8,
    U16,
    U32,
    U64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProbeInfo {
    pub id: String,
    pub vendor: String,
    pub product: String,
    pub serial: Option<String>,
    pub product_id: Option<String>,
    pub interface: Option<u8>,
    pub is_hid: bool,
    pub protocols: Vec<String>,
    pub speed_khz: Option<u32>,
    pub target_voltage: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Swd,
    Jtag,
}

#[derive(Debug, Clone)]
pub struct ConnectOptions {
    pub probe_id: Option<String>,
    pub protocol: Protocol,
    pub speed_khz: Option<u32>,
    pub target: Option<String>,
    pub under_reset: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TargetInfo {
    pub core_type: String,
    pub core_count: usize,
    pub ap_count: usize,
    pub cpu_id: Option<u32>,
    pub dp_id: Option<u32>,
    pub memory_regions: Vec<MemoryRegionSummary>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MemoryRegionSummary {
    pub name: String,
    pub kind: String,
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreRegister {
    Name(String),
    Number(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetMode {
    Run,
    Halt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFileFormat {
    Elf,
    Axf,
    Bin,
    Hex,
}

impl ImageFileFormat {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "elf" => Some(Self::Elf),
            "axf" => Some(Self::Axf),
            "bin" | "binary" => Some(Self::Bin),
            "hex" | "ihex" | "intelhex" => Some(Self::Hex),
            _ => None,
        }
    }

    pub fn from_extension(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
        match ext.as_str() {
            "elf" => Some(Self::Elf),
            "axf" => Some(Self::Axf),
            "bin" => Some(Self::Bin),
            "hex" | "ihx" => Some(Self::Hex),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Elf => "elf",
            Self::Axf => "axf",
            Self::Bin => "bin",
            Self::Hex => "hex",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Bin,
    Hex,
}

impl ExportFormat {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "bin" => Some(Self::Bin),
            "hex" | "ihex" | "intelhex" => Some(Self::Hex),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bin => "bin",
            Self::Hex => "hex",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchAccess {
    Read,
    Write,
    ReadWrite,
}

impl WatchAccess {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "read" => Some(WatchAccess::Read),
            "write" => Some(WatchAccess::Write),
            "rw" | "readwrite" | "read_write" => Some(WatchAccess::ReadWrite),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Watchpoint {
    pub address: u64,
    pub access: WatchAccess,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CoreStatusInfo {
    pub state: String,
    pub halt_reason: Option<String>,
    pub pc: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MemoryVerifyReport {
    pub verified: bool,
    pub mismatches: Vec<MemoryMismatch>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MemoryMismatch {
    pub index: usize,
    pub address: u64,
    pub expected: u64,
    pub actual: u64,
}

/// A pure helper that maps a register name to a lookup strategy.
///
/// Special roles (pc/sp/fp/lr/psr/msp/psp/fpsr) and general registers
/// (r0-r15) are resolved through role/index-based APIs; everything else
/// falls back to a by-name scan of the architecture register file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterHint {
    ProgramCounter,
    StackPointer,
    FramePointer,
    ReturnAddress,
    ProcessorStatus,
    MainStackPointer,
    ProcessStackPointer,
    FpuStatus,
    GeneralIndex(usize),
    ByName,
}

pub fn register_hint(name: &str) -> RegisterHint {
    match name.to_ascii_lowercase().as_str() {
        "pc" => RegisterHint::ProgramCounter,
        "sp" => RegisterHint::StackPointer,
        "fp" => RegisterHint::FramePointer,
        "lr" | "ra" => RegisterHint::ReturnAddress,
        "psr" | "xpsr" => RegisterHint::ProcessorStatus,
        "msp" => RegisterHint::MainStackPointer,
        "psp" => RegisterHint::ProcessStackPointer,
        "fpsr" => RegisterHint::FpuStatus,
        other => match other.strip_prefix('r') {
            Some(digits) => match digits.parse::<usize>() {
                Ok(index) if index < 16 => RegisterHint::GeneralIndex(index),
                _ => RegisterHint::ByName,
            },
            None => RegisterHint::ByName,
        },
    }
}

pub trait Backend: Send {
    fn list_probes(&self) -> Result<Vec<ProbeInfo>, McpError>;
    fn connect(&mut self, opts: &ConnectOptions) -> Result<TargetInfo, McpError>;
    fn disconnect(&mut self) -> Result<(), McpError>;
    fn read_memory(
        &mut self,
        address: u64,
        width: AccessWidth,
        count: u32,
    ) -> Result<Vec<u64>, McpError>;
    fn write_memory(
        &mut self,
        address: u64,
        width: AccessWidth,
        data: &[u64],
    ) -> Result<(), McpError>;
    fn read_core_register(&mut self, reg: &CoreRegister) -> Result<u64, McpError>;
    fn write_core_register(&mut self, reg: &CoreRegister, value: u64) -> Result<(), McpError>;
    fn halt(&mut self) -> Result<(), McpError>;
    fn resume(&mut self) -> Result<(), McpError>;
    fn step(&mut self) -> Result<(), McpError>;
    fn set_breakpoint(&mut self, address: u64) -> Result<(), McpError>;
    fn clear_breakpoints(&mut self) -> Result<(), McpError>;
    fn list_breakpoints(&mut self) -> Result<Vec<u64>, McpError>;
    fn reset(&mut self, mode: ResetMode) -> Result<(), McpError>;
    fn read_dap(&mut self, address: u32) -> Result<u32, McpError>;
    fn write_dap(&mut self, address: u32, value: u32) -> Result<(), McpError>;
    fn erase_flash(&mut self, address: u64, size: u64) -> Result<(), McpError>;
    fn program_flash(&mut self, address: u64, data: &[u8], verify: bool) -> Result<(), McpError>;
    fn list_core_registers(&mut self) -> Result<Vec<String>, McpError>;
    fn get_core_status(&mut self) -> Result<CoreStatusInfo, McpError>;
    fn set_watchpoint(&mut self, address: u64, access: WatchAccess) -> Result<(), McpError>;
    fn clear_watchpoints(&mut self) -> Result<(), McpError>;
    fn list_watchpoints(&mut self) -> Result<Vec<Watchpoint>, McpError>;
    fn verify_memory(
        &mut self,
        address: u64,
        width: AccessWidth,
        data: &[u64],
    ) -> Result<MemoryVerifyReport, McpError>;
    fn program_file(
        &mut self,
        path: &Path,
        format: ImageFileFormat,
        address: u64,
        verify: bool,
    ) -> Result<u64, McpError>;
    fn export_memory(
        &mut self,
        path: &Path,
        format: ExportFormat,
        address: u64,
        size: u64,
    ) -> Result<u64, McpError>;
}
