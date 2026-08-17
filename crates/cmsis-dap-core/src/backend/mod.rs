use crate::error::{ErrorCode, McpError};
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

/// A channel of the target's RTT control block, as seen by the host.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RttChannelInfo {
    pub number: usize,
    pub name: Option<String>,
    pub buffer_size: usize,
}

/// Bytes read from one RTT up channel in a single poll.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RttRead {
    pub channel: usize,
    pub name: Option<String>,
    pub data: Vec<u8>,
}

/// One decoded CMSIS-View Event Recorder event.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EvrEvent {
    pub timestamp_ticks: u64,
    pub timestamp_secs: f64,
    /// Event context / data length (record `info` bits 16..18).
    pub context: u8,
    pub component: u16,
    pub message: u16,
    pub irq: bool,
    pub first: bool,
    pub last: bool,
    pub sequence: u8,
    pub val1: u32,
    pub val2: u32,
}

/// Summary of the Event Recorder state read from the target.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EvrStatus {
    pub state: u8,
    pub protocol_version: String,
    pub record_count: u32,
    pub records_written: u64,
    pub records_dumped: u64,
    pub ts_freq: u32,
    pub ts_source: u8,
    pub init_count: u32,
    pub signature: u32,
    pub event_buffer: u64,
    pub event_status: u64,
}

/// One named register (or fault status register) value in a CPU dump.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RegisterValue {
    pub name: String,
    pub value: u64,
}

/// One memory sample read during a CPU dump.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MemorySample {
    pub address: u64,
    pub value: u64,
}

/// A non-invasive snapshot of the target CPU.
///
/// Memory and fault-status registers are read without halting; core registers
/// require a short halt, after which the original run state is restored when
/// `restore` was requested.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CpuStateDump {
    /// "running" or "halted" as observed before the dump.
    pub state: String,
    pub halt_reason: Option<String>,
    pub pc: Option<u64>,
    pub registers: Vec<RegisterValue>,
    /// Cortex-M SCB fault status registers (CFSR/HFSR/DFSR/MMFAR/BFAR).
    pub fault: Vec<RegisterValue>,
    /// Top words of the main stack (MSP) when available.
    pub stack_msp: Vec<u64>,
    /// Top words of the process stack (PSP) when available.
    pub stack_psp: Vec<u64>,
    /// Samples for caller-requested addresses (word reads).
    pub memory: Vec<MemorySample>,
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

    /// Attach to the target's RTT control block.
    ///
    /// `address` is an optional explicit control block address (from the
    /// firmware ELF `_SEGGER_RTT` symbol or `--address`); when absent the
    /// backend scans the target RAM.
    fn attach_rtt(&mut self, _address: Option<u64>) -> Result<Vec<RttChannelInfo>, McpError> {
        Err(McpError::new(
            ErrorCode::UnsupportedFeature,
            "RTT is not supported by this backend",
        ))
    }

    /// Read available bytes from the given RTT up channels.
    fn read_rtt(
        &mut self,
        _channels: &[usize],
        _max_bytes: usize,
    ) -> Result<Vec<RttRead>, McpError> {
        Err(McpError::new(
            ErrorCode::UnsupportedFeature,
            "RTT is not supported by this backend",
        ))
    }

    /// Detach from the target's RTT control block.
    fn detach_rtt(&mut self) -> Result<(), McpError> {
        Ok(())
    }

    /// Attach to the CMSIS-View Event Recorder at `EventRecorderInfo`.
    fn attach_evr(&mut self, _info_address: u64) -> Result<EvrStatus, McpError> {
        Err(McpError::new(
            ErrorCode::UnsupportedFeature,
            "Event Recorder is not supported by this backend",
        ))
    }

    /// Read newly committed Event Recorder events (state is kept per backend).
    fn read_evr(&mut self) -> Result<Vec<EvrEvent>, McpError> {
        Err(McpError::new(
            ErrorCode::UnsupportedFeature,
            "Event Recorder is not supported by this backend",
        ))
    }

    /// Detach from the Event Recorder.
    fn detach_evr(&mut self) -> Result<(), McpError> {
        Ok(())
    }

    /// Take a non-invasive snapshot of the target CPU.
    ///
    /// Never resets the target. Core registers are read while halted (a short
    /// halt when the core was running, restored afterwards when `restore` is
    /// set); memory and fault-status registers are read without halting.
    fn dump_cpu_state(
        &mut self,
        _addresses: &[u64],
        _stack_words: usize,
        _restore: bool,
    ) -> Result<CpuStateDump, McpError> {
        Err(McpError::new(
            ErrorCode::UnsupportedFeature,
            "CPU state dump is not supported by this backend",
        ))
    }
}
