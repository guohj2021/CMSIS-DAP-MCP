use crate::error::McpError;
pub mod mock;
pub mod probe_rs;

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
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TargetInfo {
    pub core_type: String,
    pub ap_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreRegister {
    Name(String),
    Number(u16),
}

pub trait Backend: Send {
    fn list_probes(&self) -> Result<Vec<ProbeInfo>, McpError>;
    fn connect(&mut self, opts: &ConnectOptions) -> Result<TargetInfo, McpError>;
    fn disconnect(&mut self) -> Result<(), McpError>;
    fn read_memory(&mut self, address: u64, width: AccessWidth, count: u32) -> Result<Vec<u64>, McpError>;
    fn write_memory(&mut self, address: u64, width: AccessWidth, data: &[u64]) -> Result<(), McpError>;
    fn read_core_register(&mut self, reg: &CoreRegister) -> Result<u64, McpError>;
    fn write_core_register(&mut self, reg: &CoreRegister, value: u64) -> Result<(), McpError>;
    fn halt(&mut self) -> Result<(), McpError>;
    fn resume(&mut self) -> Result<(), McpError>;
    fn step(&mut self) -> Result<(), McpError>;
    fn set_breakpoint(&mut self, address: u64) -> Result<(), McpError>;
    fn clear_breakpoints(&mut self) -> Result<(), McpError>;
    fn list_breakpoints(&mut self) -> Result<Vec<u64>, McpError>;
    fn reset(&mut self) -> Result<(), McpError>;
    fn read_dap(&mut self, address: u32) -> Result<u32, McpError>;
    fn write_dap(&mut self, address: u32, value: u32) -> Result<(), McpError>;
    fn erase_flash(&mut self, address: u64, size: u64) -> Result<(), McpError>;
    fn program_flash(&mut self, address: u64, data: &[u8]) -> Result<(), McpError>;
}