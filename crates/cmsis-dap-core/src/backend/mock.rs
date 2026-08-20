use crate::backend::{
    AccessWidth, Backend, ConnectOptions, CoreRegister, CoreStatusInfo, EvrEvent, EvrStatus,
    ExportFormat, ImageFileFormat, MemoryMismatch, MemoryRegionSummary, MemoryVerifyReport,
    ProbeInfo, ResetMode, RttChannelInfo, RttRead, TargetInfo, WatchAccess, Watchpoint,
};
use crate::error::{ErrorCode, McpError};
use crate::evr;
use std::collections::HashMap;

pub struct MockBackend {
    memory: HashMap<u64, u64>,
    registers: HashMap<String, u64>,
    dap: HashMap<u32, u32>,
    with_flash: bool,
    connected: bool,
    halted: bool,
    breakpoints: Vec<u64>,
    watchpoints: Vec<Watchpoint>,
    rtt_attached: bool,
    rtt_names: Vec<Option<String>>,
    rtt_pending: Vec<Vec<u8>>,
    evr_attached: bool,
    evr_records: Vec<[u8; evr::RECORD_SIZE]>,
    evr_record_count: u32,
    evr_ts_freq: u32,
    evr_ts_overflow: u32,
    evr_last_index: u32,
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
            with_flash: true,
            connected: false,
            halted: false,
            breakpoints: Vec::new(),
            watchpoints: Vec::new(),
            rtt_attached: false,
            rtt_names: Vec::new(),
            rtt_pending: Vec::new(),
            evr_attached: false,
            evr_records: Vec::new(),
            evr_record_count: 8,
            evr_ts_freq: 1_000_000,
            evr_ts_overflow: 0,
            evr_last_index: 0,
        }
    }

    /// A mock backend whose target has no flash memory region (used to verify
    /// that flash operations fail loudly instead of silently doing nothing).
    pub fn without_flash() -> Self {
        Self {
            with_flash: false,
            ..Self::new()
        }
    }

    /// A mock backend with RTT up channels and pending bytes per channel.
    pub fn with_rtt(channels: &[(Option<&str>, &[u8])]) -> Self {
        let mut backend = Self::new();
        backend.rtt_names = channels
            .iter()
            .map(|(name, _)| name.map(|s| s.to_string()))
            .collect();
        backend.rtt_pending = channels.iter().map(|(_, data)| data.to_vec()).collect();
        backend
    }

    /// A mock backend whose Event Recorder already contains `records`.
    pub fn with_evr(record_count: u32, ts_freq: u32, records: Vec<[u8; evr::RECORD_SIZE]>) -> Self {
        let mut backend = Self::new();
        backend.evr_record_count = record_count.max(1);
        backend.evr_ts_freq = ts_freq;
        backend.evr_records = records;
        backend
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
            memory_regions: {
                let mut regions = Vec::new();
                if self.with_flash {
                    regions.push(MemoryRegionSummary {
                        name: "FLASH".into(),
                        kind: "nvm".into(),
                        start: 0x0800_0000,
                        end: 0x0801_0000,
                    });
                }
                regions.push(MemoryRegionSummary {
                    name: "RAM".into(),
                    kind: "ram".into(),
                    start: 0x2000_0000,
                    end: 0x2001_0000,
                });
                regions
            },
        })
    }

    fn disconnect(&mut self) -> Result<(), McpError> {
        self.connected = false;
        self.rtt_attached = false;
        self.evr_attached = false;
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
        Ok(match width {
            AccessWidth::U8 => (0..count)
                .map(|i| {
                    let a = address + i as u64;
                    if let Some(v) = self.memory.get(&a) {
                        *v & 0xFF
                    } else {
                        self.memory
                            .get(&(a & !3))
                            .map(|w| (*w >> ((a & 3) * 8)) & 0xFF)
                            .unwrap_or(0)
                    }
                })
                .collect(),
            AccessWidth::U16 => (0..count)
                .map(|i| {
                    let a = address + i as u64 * 2;
                    if let Some(v) = self.memory.get(&a) {
                        *v & 0xFFFF
                    } else {
                        self.memory
                            .get(&(a & !1))
                            .map(|w| (*w >> ((a & 1) * 16)) & 0xFFFF)
                            .unwrap_or(0)
                    }
                })
                .collect(),
            AccessWidth::U32 => (0..count)
                .map(|i| *self.memory.get(&(address + i as u64 * step)).unwrap_or(&0))
                .collect(),
            AccessWidth::U64 => (0..count)
                .map(|i| *self.memory.get(&(address + i as u64 * step)).unwrap_or(&0))
                .collect(),
        })
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
        if size == u64::MAX {
            self.memory.clear();
            return Ok(());
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

    fn program_file(
        &mut self,
        path: &std::path::Path,
        _format: ImageFileFormat,
        address: u64,
        _verify: bool,
    ) -> Result<u64, McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        let data =
            std::fs::read(path).map_err(|e| McpError::new(ErrorCode::FileError, e.to_string()))?;
        for (i, b) in data.iter().enumerate() {
            self.memory.insert(address + i as u64, *b as u64);
        }
        Ok(data.len() as u64)
    }

    fn export_memory(
        &mut self,
        path: &std::path::Path,
        format: ExportFormat,
        address: u64,
        size: u64,
    ) -> Result<u64, McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        if size == 0 {
            return Err(McpError::new(
                ErrorCode::InvalidArgument,
                "export size must be greater than zero",
            ));
        }
        let values = self.read_memory(address, AccessWidth::U8, size as u32)?;
        let bytes: Vec<u8> = values.iter().map(|v| *v as u8).collect();
        match format {
            ExportFormat::Bin => std::fs::write(path, &bytes)
                .map_err(|e| McpError::new(ErrorCode::FileError, e.to_string()))?,
            ExportFormat::Hex => std::fs::write(path, crate::hex::encode_ihex(&bytes, address))
                .map_err(|e| McpError::new(ErrorCode::FileError, e.to_string()))?,
        }
        Ok(size)
    }

    fn define_target(&mut self, _yaml: &str) -> Result<(), McpError> {
        // The mock backend does not resolve a real probe-rs target; accept the
        // definition so callers can exercise the define_chip tool path.
        Ok(())
    }

    fn attach_rtt(&mut self, _address: Option<u64>) -> Result<Vec<RttChannelInfo>, McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        self.rtt_attached = true;
        Ok((0..self.rtt_names.len())
            .map(|number| RttChannelInfo {
                number,
                name: self.rtt_names[number].clone(),
                buffer_size: 1024,
            })
            .collect())
    }

    fn read_rtt(&mut self, channels: &[usize], max_bytes: usize) -> Result<Vec<RttRead>, McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        if !self.rtt_attached {
            return Err(McpError::new(
                ErrorCode::NotConnected,
                "RTT is not attached; run 'rtt info' or 'rtt monitor' first",
            ));
        }
        let mut out = Vec::new();
        for &channel in channels {
            let Some(pending) = self.rtt_pending.get_mut(channel) else {
                continue;
            };
            if pending.is_empty() {
                continue;
            }
            let take = pending.len().min(max_bytes.max(1));
            let data = pending.drain(..take).collect::<Vec<_>>();
            out.push(RttRead {
                channel,
                name: self.rtt_names.get(channel).cloned().flatten(),
                data,
            });
        }
        Ok(out)
    }

    fn detach_rtt(&mut self) -> Result<(), McpError> {
        self.rtt_attached = false;
        Ok(())
    }

    fn attach_evr(&mut self, _info_address: u64) -> Result<EvrStatus, McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        self.evr_attached = true;
        self.evr_last_index = 0;
        Ok(EvrStatus {
            state: 1,
            protocol_version: "1.1".into(),
            record_count: self.evr_record_count,
            records_written: self.evr_records.len() as u64,
            records_dumped: 0,
            ts_freq: self.evr_ts_freq,
            ts_source: 0,
            init_count: 1,
            signature: 0x4556_5254,
            event_buffer: 0x2000_0000,
            event_status: 0x2000_0100,
        })
    }

    fn read_evr(&mut self) -> Result<Vec<EvrEvent>, McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        if !self.evr_attached {
            return Err(McpError::new(
                ErrorCode::NotConnected,
                "Event Recorder is not attached; run 'evr info' or 'evr monitor' first",
            ));
        }
        let mut events = Vec::new();
        while (self.evr_last_index as usize) < self.evr_records.len() {
            let index = self.evr_last_index;
            let Some(record) = self.evr_records.get(index as usize) else {
                break;
            };
            match evr::decode_record(record, self.evr_ts_overflow, self.evr_ts_freq) {
                Some(event) => {
                    self.evr_last_index = index.wrapping_add(1);
                    events.push(event);
                }
                None => break,
            }
        }
        Ok(events)
    }

    fn detach_evr(&mut self) -> Result<(), McpError> {
        self.evr_attached = false;
        Ok(())
    }

    fn dump_cpu_state(
        &mut self,
        addresses: &[u64],
        stack_words: usize,
        restore: bool,
    ) -> Result<crate::backend::CpuStateDump, McpError> {
        if !self.connected {
            return Err(not_connected());
        }
        let was_running = !self.halted;
        let should_restore = was_running && restore;
        if was_running {
            self.halted = true;
        }

        let mut registers = Vec::new();
        for name in self.list_core_registers()? {
            let value = *self.registers.get(&name).unwrap_or(&0);
            registers.push(crate::backend::RegisterValue { name, value });
        }
        let pc = registers.iter().find(|r| r.name == "pc").map(|r| r.value);

        const FAULT_REGS: [(&str, u64); 5] = [
            ("CFSR", 0xE000_ED28),
            ("HFSR", 0xE000_ED2C),
            ("DFSR", 0xE000_ED30),
            ("MMFAR", 0xE000_ED34),
            ("BFAR", 0xE000_ED38),
        ];
        let mut fault = Vec::new();
        for (name, address) in FAULT_REGS {
            let values = self.read_memory(address, AccessWidth::U32, 1)?;
            fault.push(crate::backend::RegisterValue {
                name: name.to_string(),
                value: values[0],
            });
        }

        let sp_of = |name: &str| registers.iter().find(|r| r.name == name).map(|r| r.value);
        let mut stack_msp = Vec::new();
        if let Some(sp) = sp_of("msp") {
            for i in 0..stack_words.min(1024) {
                let values = self.read_memory(sp + (i as u64) * 4, AccessWidth::U32, 1)?;
                stack_msp.push(values[0]);
            }
        }
        let mut stack_psp = Vec::new();
        if let Some(sp) = sp_of("psp") {
            for i in 0..stack_words.min(1024) {
                let values = self.read_memory(sp + (i as u64) * 4, AccessWidth::U32, 1)?;
                stack_psp.push(values[0]);
            }
        }

        let mut memory = Vec::new();
        for &address in addresses {
            let values = self.read_memory(address, AccessWidth::U32, 1)?;
            memory.push(crate::backend::MemorySample {
                address,
                value: values[0],
            });
        }

        if should_restore {
            self.halted = false;
        }
        Ok(crate::backend::CpuStateDump {
            state: if was_running { "running" } else { "halted" }.into(),
            halt_reason: if was_running {
                None
            } else {
                Some("request".into())
            },
            pc,
            registers,
            fault,
            stack_msp,
            stack_psp,
            memory,
        })
    }
}
