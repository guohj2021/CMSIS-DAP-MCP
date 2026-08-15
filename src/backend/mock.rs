use crate::backend::{AccessWidth, Backend, ConnectOptions, CoreRegister, ProbeInfo, Protocol, TargetInfo};
use crate::error::{ErrorCode, McpError};
use std::collections::HashMap;

pub struct MockBackend {
    memory: HashMap<u64, u64>,
    registers: HashMap<String, u64>,
    dap: HashMap<u32, u32>,
    connected: bool,
    halted: bool,
    breakpoints: Vec<u64>,
}

impl MockBackend {
    pub fn new() -> Self {
        Self {
            memory: HashMap::new(),
            registers: HashMap::new(),
            dap: HashMap::new(),
            connected: false,
            halted: false,
            breakpoints: Vec::new(),
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

    fn read_core_register(&mut self, reg: &CoreRegister) -> Result<u64, McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        match reg {
            CoreRegister::Name(n) => Ok(*self.registers.get(n).unwrap_or(&0)),
            CoreRegister::Number(_) => Err(McpError::new(ErrorCode::UnsupportedFeature, "number-based register access not supported by mock")),
        }
    }

    fn write_core_register(&mut self, reg: &CoreRegister, value: u64) -> Result<(), McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        match reg {
            CoreRegister::Name(n) => {
                self.registers.insert(n.clone(), value);
                Ok(())
            }
            CoreRegister::Number(_) => Err(McpError::new(ErrorCode::UnsupportedFeature, "number-based register access not supported by mock")),
        }
    }

    fn halt(&mut self) -> Result<(), McpError> {
        if !self.connected { Err(not_connected()) } else { self.halted = true; Ok(()) }
    }

    fn resume(&mut self) -> Result<(), McpError> {
        if !self.connected { Err(not_connected()) } else { self.halted = false; Ok(()) }
    }

    fn step(&mut self) -> Result<(), McpError> {
        if !self.connected { Err(not_connected()) } else { Ok(()) }
    }

    fn set_breakpoint(&mut self, address: u64) -> Result<(), McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        if !self.breakpoints.contains(&address) {
            self.breakpoints.push(address);
            self.breakpoints.sort_unstable();
        }
        Ok(())
    }

    fn clear_breakpoints(&mut self) -> Result<(), McpError> {
        if !self.connected { Err(not_connected()) } else { self.breakpoints.clear(); Ok(()) }
    }

    fn list_breakpoints(&mut self) -> Result<Vec<u64>, McpError> {
        if !self.connected { Err(not_connected()) } else { Ok(self.breakpoints.clone()) }
    }

    fn reset(&mut self) -> Result<(), McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        self.breakpoints.clear();
        self.halted = false;
        Ok(())
    }

    fn read_dap(&mut self, address: u32) -> Result<u32, McpError> {
        if !self.connected { Err(not_connected()) } else { Ok(*self.dap.get(&address).unwrap_or(&0)) }
    }

    fn write_dap(&mut self, address: u32, value: u32) -> Result<(), McpError> {
        if !self.connected { Err(not_connected()) } else { self.dap.insert(address, value); Ok(()) }
    }

    fn erase_flash(&mut self, _address: u64, _size: u64) -> Result<(), McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        Err(McpError::new(ErrorCode::UnsupportedFeature, "flash not implemented in mock backend"))
    }

    fn program_flash(&mut self, _address: u64, _data: &[u8]) -> Result<(), McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        Err(McpError::new(ErrorCode::UnsupportedFeature, "flash not implemented in mock backend"))
    }
}