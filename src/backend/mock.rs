use crate::backend::{
    AccessWidth, Backend, ConnectOptions, CoreRegister, CoreStatusInfo, MemoryMismatch,
    MemoryRegionSummary, MemoryVerifyReport, ProbeInfo, ResetMode, TargetInfo, WatchAccess,
    Watchpoint,
};
use crate::error::{ErrorCode, McpError};
use std::collections::HashMap;

pub struct MockBackend {
    memory: HashMap<u64, u64>,
    registers: HashMap<String, u64>,
    dap: HashMap<u32, u32>,
    connected: bool,
    halted: bool,
    breakpoints: Vec<u64>,
    watchpoints: Vec<Watchpoint>,
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MockBackend {
    pub fn new() -> Self {
        Self {
            memory: HashMap::new(),
            registers: HashMap::from([
                ("r0".into(), 0u64),
                ("r1".into(), 1u64),
                ("pc".into(), 0x0800_0100u64),
                ("sp".into(), 0x2000_2000u64),
                ("lr".into(), 0x0800_00FFu64),
                ("xpsr".into(), 0x0100_0000u64),
            ]),
            dap: HashMap::new(),
            connected: false,
            halted: false,
            breakpoints: Vec::new(),
            watchpoints: Vec::new(),
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
            product_id: Some("5051".into()),
            interface: Some(0),
            is_hid: true,
            protocols: vec!["swd".into(), "jtag".into()],
            speed_khz: Some(1000),
            target_voltage: Some(3.3),
        }])
    }

    fn connect(&mut self, _opts: &ConnectOptions) -> Result<TargetInfo, McpError> {
        self.connected = true;
        Ok(TargetInfo {
            core_type: "Cortex-M0".into(),
            core_count: 1,
            ap_count: 1,
            cpu_id: Some(0x410C_C601),
            dp_id: Some(0x0BB1_1477),
            memory_regions: vec![
                MemoryRegionSummary {
                    name: "FLASH".into(),
                    kind: "nvm".into(),
                    start: 0x0800_0000,
                    end: 0x0801_0000,
                },
                MemoryRegionSummary {
                    name: "RAM".into(),
                    kind: "ram".into(),
                    start: 0x2000_0000,
                    end: 0x2001_0000,
                },
            ],
        })
    }

    fn disconnect(&mut self) -> Result<(), McpError> {
        self.connected = false;
        Ok(())
    }

    fn read_memory(
        &mut self,
        address: u64,
        width: AccessWidth,
        count: u32,
    ) -> Result<Vec<u64>, McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        let step = width_bytes(width);
        Ok((0..count)
            .map(|i| *self.memory.get(&(address + i as u64 * step)).unwrap_or(&0))
            .collect())
    }

    fn write_memory(
        &mut self,
        address: u64,
        width: AccessWidth,
        data: &[u64],
    ) -> Result<(), McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        for (i, v) in data.iter().enumerate() {
            self.memory
                .insert(address + i as u64 * width_bytes(width), *v);
        }
        Ok(())
    }

    fn read_core_register(&mut self, reg: &CoreRegister) -> Result<u64, McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        match reg {
            CoreRegister::Name(n) => Ok(*self.registers.get(n).unwrap_or(&0)),
            CoreRegister::Number(_) => Err(McpError::new(
                ErrorCode::UnsupportedFeature,
                "number-based register access not supported by mock",
            )),
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
            CoreRegister::Number(_) => Err(McpError::new(
                ErrorCode::UnsupportedFeature,
                "number-based register access not supported by mock",
            )),
        }
    }

    fn halt(&mut self) -> Result<(), McpError> {
        if !self.connected {
            Err(not_connected())
        } else {
            self.halted = true;
            Ok(())
        }
    }

    fn resume(&mut self) -> Result<(), McpError> {
        if !self.connected {
            Err(not_connected())
        } else {
            self.halted = false;
            Ok(())
        }
    }

    fn step(&mut self) -> Result<(), McpError> {
        if !self.connected {
            Err(not_connected())
        } else {
            Ok(())
        }
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
        if !self.connected {
            Err(not_connected())
        } else {
            self.breakpoints.clear();
            Ok(())
        }
    }

    fn list_breakpoints(&mut self) -> Result<Vec<u64>, McpError> {
        if !self.connected {
            Err(not_connected())
        } else {
            Ok(self.breakpoints.clone())
        }
    }

    fn reset(&mut self, mode: ResetMode) -> Result<(), McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        self.breakpoints.clear();
        self.halted = mode == ResetMode::Halt;
        Ok(())
    }

    fn read_dap(&mut self, address: u32) -> Result<u32, McpError> {
        if !self.connected {
            Err(not_connected())
        } else {
            Ok(*self.dap.get(&address).unwrap_or(&0))
        }
    }

    fn write_dap(&mut self, address: u32, value: u32) -> Result<(), McpError> {
        if !self.connected {
            Err(not_connected())
        } else {
            self.dap.insert(address, value);
            Ok(())
        }
    }

    fn erase_flash(&mut self, address: u64, size: u64) -> Result<(), McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        if size == 0 {
            return Err(McpError::new(
                ErrorCode::InvalidArgument,
                "erase size must be greater than zero",
            ));
        }
        for i in 0..size {
            self.memory.insert(address + i, 0xFF);
        }
        Ok(())
    }

    fn program_flash(&mut self, address: u64, data: &[u8], _verify: bool) -> Result<(), McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        for (i, b) in data.iter().enumerate() {
            self.memory.insert(address + i as u64, *b as u64);
        }
        Ok(())
    }

    fn list_core_registers(&mut self) -> Result<Vec<String>, McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        let mut names: Vec<String> = (0..16).map(|i| format!("r{i}")).collect();
        names.extend([
            "pc".into(),
            "sp".into(),
            "lr".into(),
            "xpsr".into(),
            "msp".into(),
            "psp".into(),
            "primask".into(),
        ]);
        Ok(names)
    }

    fn get_core_status(&mut self) -> Result<CoreStatusInfo, McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        Ok(CoreStatusInfo {
            state: if self.halted {
                "halted".into()
            } else {
                "running".into()
            },
            halt_reason: if self.halted {
                Some("request".into())
            } else {
                None
            },
            pc: self.registers.get("pc").copied(),
        })
    }

    fn set_watchpoint(&mut self, address: u64, access: WatchAccess) -> Result<(), McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        if !self.watchpoints.iter().any(|w| w.address == address) {
            self.watchpoints.push(Watchpoint { address, access });
        }
        Ok(())
    }

    fn clear_watchpoints(&mut self) -> Result<(), McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        self.watchpoints.clear();
        Ok(())
    }

    fn list_watchpoints(&mut self) -> Result<Vec<Watchpoint>, McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        Ok(self.watchpoints.clone())
    }

    fn verify_memory(
        &mut self,
        address: u64,
        width: AccessWidth,
        data: &[u64],
    ) -> Result<MemoryVerifyReport, McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        let actual = self.read_memory(address, width, data.len() as u32)?;
        let mismatches: Vec<MemoryMismatch> = data
            .iter()
            .enumerate()
            .zip(actual.iter())
            .filter(|((_, expected), actual)| expected != actual)
            .map(|((index, expected), actual)| MemoryMismatch {
                index,
                address: address + index as u64 * width_bytes(width),
                expected: *expected,
                actual: *actual,
            })
            .collect();
        Ok(MemoryVerifyReport {
            verified: mismatches.is_empty(),
            mismatches,
        })
    }
}
