use crate::backend::{AccessWidth, Backend, ConnectOptions, CoreRegister, ProbeInfo, TargetInfo};
use crate::error::{ErrorCode, McpError};
use probe_rs::probe::list::Lister;

pub struct ProbeRsBackend;

impl ProbeRsBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Backend for ProbeRsBackend {
    fn list_probes(&self) -> Result<Vec<ProbeInfo>, McpError> {
        let lister = Lister::new();
        let probes = lister.list_all();
        Ok(probes
            .into_iter()
            .map(|p| ProbeInfo {
                id: p.serial_number.clone().unwrap_or_else(|| p.identifier.clone()),
                vendor: format!("{:04x}", p.vendor_id),
                product: p.identifier,
                serial: p.serial_number,
            })
            .collect())
    }

    fn connect(&mut self, _opts: &ConnectOptions) -> Result<TargetInfo, McpError> {
        Err(McpError::new(ErrorCode::UnsupportedFeature, "connect not implemented yet"))
    }

    fn disconnect(&mut self) -> Result<(), McpError> {
        Ok(())
    }

    fn read_memory(&mut self, _address: u64, _width: AccessWidth, _count: u32) -> Result<Vec<u64>, McpError> {
        Err(McpError::new(ErrorCode::UnsupportedFeature, "read_memory not implemented yet"))
    }

    fn write_memory(&mut self, _address: u64, _width: AccessWidth, _data: &[u64]) -> Result<(), McpError> {
        Err(McpError::new(ErrorCode::UnsupportedFeature, "write_memory not implemented yet"))
    }

    fn read_core_register(&mut self, _reg: &CoreRegister) -> Result<u64, McpError> {
        Err(McpError::new(ErrorCode::UnsupportedFeature, "not implemented yet"))
    }

    fn write_core_register(&mut self, _reg: &CoreRegister, _value: u64) -> Result<(), McpError> {
        Err(McpError::new(ErrorCode::UnsupportedFeature, "not implemented yet"))
    }

    fn halt(&mut self) -> Result<(), McpError> {
        Err(McpError::new(ErrorCode::UnsupportedFeature, "not implemented yet"))
    }

    fn resume(&mut self) -> Result<(), McpError> {
        Err(McpError::new(ErrorCode::UnsupportedFeature, "not implemented yet"))
    }

    fn step(&mut self) -> Result<(), McpError> {
        Err(McpError::new(ErrorCode::UnsupportedFeature, "not implemented yet"))
    }

    fn set_breakpoint(&mut self, _address: u64) -> Result<(), McpError> {
        Err(McpError::new(ErrorCode::UnsupportedFeature, "not implemented yet"))
    }

    fn clear_breakpoints(&mut self) -> Result<(), McpError> {
        Err(McpError::new(ErrorCode::UnsupportedFeature, "not implemented yet"))
    }

    fn list_breakpoints(&mut self) -> Result<Vec<u64>, McpError> {
        Err(McpError::new(ErrorCode::UnsupportedFeature, "not implemented yet"))
    }

    fn reset(&mut self) -> Result<(), McpError> {
        Err(McpError::new(ErrorCode::UnsupportedFeature, "not implemented yet"))
    }

    fn read_dap(&mut self, _address: u32) -> Result<u32, McpError> {
        Err(McpError::new(ErrorCode::UnsupportedFeature, "not implemented yet"))
    }

    fn write_dap(&mut self, _address: u32, _value: u32) -> Result<(), McpError> {
        Err(McpError::new(ErrorCode::UnsupportedFeature, "not implemented yet"))
    }

    fn erase_flash(&mut self, _address: u64, _size: u64) -> Result<(), McpError> {
        Err(McpError::new(ErrorCode::UnsupportedFeature, "not implemented yet"))
    }

    fn program_flash(&mut self, _address: u64, _data: &[u8]) -> Result<(), McpError> {
        Err(McpError::new(ErrorCode::UnsupportedFeature, "not implemented yet"))
    }
}