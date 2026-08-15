use crate::backend::{AccessWidth, Backend, ConnectOptions, CoreRegister, ProbeInfo, Protocol, TargetInfo};
use crate::error::{ErrorCode, McpError};
use std::collections::HashMap;

pub struct MockBackend {
    memory: HashMap<u64, u64>,
    connected: bool,
}

impl MockBackend {
    pub fn new() -> Self {
        Self {
            memory: HashMap::new(),
            connected: false,
        }
    }
}

fn width_bytes(width: AccessWidth) -> u64 {
    match width {
        AccessWidth::U8 => 1,
        AccessWidth::U16 => 2,
        AccessWidth::U32 => 4,
        AccessWidth::U64 => 8,
    }
}

fn not_connected() -> McpError {
    McpError::new(ErrorCode::NotConnected, "no active session")
}

fn not_implemented() -> McpError {
    McpError::new(ErrorCode::UnsupportedFeature, "not implemented in mock backend")
}

impl Backend for MockBackend {
    fn list_probes(&self) -> Result<Vec<ProbeInfo>, McpError> {
        Ok(vec![ProbeInfo {
            id: "mock".into(),
            vendor: "mock".into(),
            product: "mock".into(),
            serial: None,
        }])
    }

    fn connect(&mut self, _opts: &ConnectOptions) -> Result<TargetInfo, McpError> {
        self.connected = true;
        Ok(TargetInfo { core_type: "Cortex-M0".into(), ap_count: 1 })
    }

    fn disconnect(&mut self) -> Result<(), McpError> {
        self.connected = false;
        Ok(())
    }

    fn read_memory(&mut self, address: u64, width: AccessWidth, count: u32) -> Result<Vec<u64>, McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        let step = width_bytes(width);
        Ok((0..count)
            .map(|i| *self.memory.get(&(address + i as u64 * step)).unwrap_or(&0))
            .collect())
    }

    fn write_memory(&mut self, address: u64, width: AccessWidth, data: &[u64]) -> Result<(), McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        for (i, v) in data.iter().enumerate() {
            self.memory.insert(address + i as u64 * width_bytes(width), *v);
        }
        Ok(())
    }

    fn read_core_register(&mut self, _reg: &CoreRegister) -> Result<u64, McpError> {
        if !self.connected { Err(not_connected()) } else { Err(not_implemented()) }
    }

    fn write_core_register(&mut self, _reg: &CoreRegister, _value: u64) -> Result<(), McpError> {
        if !self.connected { Err(not_connected()) } else { Err(not_implemented()) }
    }

    fn halt(&mut self) -> Result<(), McpError> {
        if !self.connected { Err(not_connected()) } else { Err(not_implemented()) }
    }

    fn resume(&mut self) -> Result<(), McpError> {
        if !self.connected { Err(not_connected()) } else { Err(not_implemented()) }
    }

    fn step(&mut self) -> Result<(), McpError> {
        if !self.connected { Err(not_connected()) } else { Err(not_implemented()) }
    }

    fn set_breakpoint(&mut self, _address: u64) -> Result<(), McpError> {
        if !self.connected { Err(not_connected()) } else { Err(not_implemented()) }
    }

    fn clear_breakpoints(&mut self) -> Result<(), McpError> {
        if !self.connected { Err(not_connected()) } else { Err(not_implemented()) }
    }

    fn list_breakpoints(&mut self) -> Result<Vec<u64>, McpError> {
        if !self.connected { Err(not_connected()) } else { Err(not_implemented()) }
    }

    fn reset(&mut self) -> Result<(), McpError> {
        if !self.connected { Err(not_connected()) } else { Err(not_implemented()) }
    }

    fn read_dap(&mut self, _address: u32) -> Result<u32, McpError> {
        if !self.connected { Err(not_connected()) } else { Err(not_implemented()) }
    }

    fn write_dap(&mut self, _address: u32, _value: u32) -> Result<(), McpError> {
        if !self.connected { Err(not_connected()) } else { Err(not_implemented()) }
    }

    fn erase_flash(&mut self, _address: u64, _size: u64) -> Result<(), McpError> {
        if !self.connected { Err(not_connected()) } else { Err(not_implemented()) }
    }

    fn program_flash(&mut self, _address: u64, _data: &[u8]) -> Result<(), McpError> {
        if !self.connected { Err(not_connected()) } else { Err(not_implemented()) }
    }
}