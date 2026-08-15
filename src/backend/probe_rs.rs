use crate::backend::{
    AccessWidth, Backend, ConnectOptions, CoreRegister, ProbeInfo, Protocol, TargetInfo,
};
use crate::error::{ErrorCode, McpError};
use probe_rs::probe::{list::Lister, WireProtocol};
use probe_rs::{MemoryInterface, Permissions, Session};
use std::time::Duration;

pub struct ProbeRsBackend {
    session: Option<Session>,
    core_index: usize,
    breakpoints: Vec<u64>,
    registry: Option<probe_rs::config::Registry>,
}

impl Default for ProbeRsBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ProbeRsBackend {
    pub fn new() -> Self {
        Self {
            session: None,
            core_index: 0,
            breakpoints: Vec::new(),
            registry: None,
        }
    }

    pub fn with_registry(registry: probe_rs::config::Registry) -> Self {
        Self {
            session: None,
            core_index: 0,
            breakpoints: Vec::new(),
            registry: Some(registry),
        }
    }

    fn core(&mut self) -> Result<probe_rs::Core<'_>, McpError> {
        self.session
            .as_mut()
            .ok_or_else(|| McpError::new(ErrorCode::NotConnected, "no active session"))?
            .core(self.core_index)
            .map_err(|e| McpError::new(ErrorCode::ConnectFailed, e.to_string()))
    }

    fn resolve_register(
        core: &probe_rs::Core<'_>,
        reg: &CoreRegister,
    ) -> Result<probe_rs::RegisterId, McpError> {
        match reg {
            CoreRegister::Number(n) => Ok(probe_rs::RegisterId(*n)),
            CoreRegister::Name(name) => {
                let found = match name.to_ascii_lowercase().as_str() {
                    "pc" => Some(core.program_counter()),
                    "sp" => Some(core.stack_pointer()),
                    "fp" => Some(core.frame_pointer()),
                    "lr" | "ra" => Some(core.return_address()),
                    "psr" => core.registers().psr(),
                    _ => core
                        .registers()
                        .all_registers()
                        .find(|r| r.name().eq_ignore_ascii_case(name)),
                };
                found.map(|r| r.id()).ok_or_else(|| {
                    McpError::new(
                        ErrorCode::InvalidArgument,
                        format!("unknown core register {name}"),
                    )
                })
            }
        }
    }
}

impl Backend for ProbeRsBackend {
    fn list_probes(&self) -> Result<Vec<ProbeInfo>, McpError> {
        let lister = Lister::new();
        let probes = lister.list_all();
        Ok(probes
            .into_iter()
            .map(|p| ProbeInfo {
                id: p
                    .serial_number
                    .clone()
                    .unwrap_or_else(|| p.identifier.clone()),
                vendor: format!("{:04x}", p.vendor_id),
                product: p.identifier,
                serial: p.serial_number,
            })
            .collect())
    }

    fn connect(&mut self, opts: &ConnectOptions) -> Result<TargetInfo, McpError> {
        if self.session.is_some() {
            self.disconnect()?;
        }
        let lister = Lister::new();
        let probes = lister.list_all();
        let selected = match &opts.probe_id {
            Some(id) => probes
                .iter()
                .find(|p| p.serial_number.as_deref() == Some(id.as_str()) || p.identifier == *id)
                .ok_or_else(|| {
                    McpError::new(ErrorCode::ProbeNotFound, format!("no probe with id {id}"))
                })?,
            None => probes
                .first()
                .ok_or_else(|| McpError::new(ErrorCode::ProbeNotFound, "no probe found"))?,
        };
        let mut probe = lister
            .open(selected)
            .map_err(|e| McpError::new(ErrorCode::ProbeNotFound, e.to_string()))?;
        probe
            .set_speed(opts.speed_khz.unwrap_or(1000))
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
        let wire_protocol = match opts.protocol {
            Protocol::Swd => WireProtocol::Swd,
            Protocol::Jtag => WireProtocol::Jtag,
        };
        probe
            .select_protocol(wire_protocol)
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
        let attach =
            |probe: probe_rs::probe::Probe, target: &str, registry: &probe_rs::config::Registry| {
                probe
                    .attach_with_registry(target, Permissions::default(), registry)
                    .map_err(|e| McpError::new(ErrorCode::ConnectFailed, e.to_string()))
            };
        let session = match (&self.registry, &opts.target) {
            (Some(registry), Some(name)) => attach(probe, name, registry)?,
            (Some(registry), None) => attach(probe, "Cortex-M0", registry)?,
            (None, Some(name)) => {
                let registry = probe_rs::config::Registry::from_builtin_families();
                attach(probe, name, &registry)?
            }
            (None, None) => {
                let registry = probe_rs::config::Registry::from_builtin_families();
                attach(probe, "Cortex-M0", &registry)?
            }
        };
        let core_type = session
            .target()
            .cores
            .first()
            .map(|c| format!("{:?}", c.core_type))
            .unwrap_or_else(|| "unknown".into());
        let ap_count = session.target().memory_map.len();
        self.breakpoints.clear();
        self.session = Some(session);
        Ok(TargetInfo {
            core_type,
            ap_count,
        })
    }

    fn disconnect(&mut self) -> Result<(), McpError> {
        self.session.take();
        self.breakpoints.clear();
        Ok(())
    }

    fn read_memory(
        &mut self,
        address: u64,
        width: AccessWidth,
        count: u32,
    ) -> Result<Vec<u64>, McpError> {
        let mut core = self.core()?;
        let mut out = Vec::with_capacity(count as usize);
        let map_err = |e: probe_rs::Error| McpError::new(ErrorCode::MemoryFault, e.to_string());
        match width {
            AccessWidth::U8 => {
                let mut buf = vec![0u8; count as usize];
                core.read_8(address, &mut buf).map_err(map_err)?;
                out.extend(buf.iter().map(|v| *v as u64));
            }
            AccessWidth::U16 => {
                let mut buf = vec![0u16; count as usize];
                core.read_16(address, &mut buf).map_err(map_err)?;
                out.extend(buf.iter().map(|v| *v as u64));
            }
            AccessWidth::U32 => {
                let mut buf = vec![0u32; count as usize];
                core.read_32(address, &mut buf).map_err(map_err)?;
                out.extend(buf.iter().map(|v| *v as u64));
            }
            AccessWidth::U64 => {
                let mut buf = vec![0u64; count as usize];
                core.read_64(address, &mut buf).map_err(map_err)?;
                out.extend(buf);
            }
        }
        Ok(out)
    }

    fn write_memory(
        &mut self,
        address: u64,
        width: AccessWidth,
        data: &[u64],
    ) -> Result<(), McpError> {
        let mut core = self.core()?;
        let map_err = |e: probe_rs::Error| McpError::new(ErrorCode::MemoryFault, e.to_string());
        match width {
            AccessWidth::U8 => {
                let buf: Vec<u8> = data.iter().map(|v| *v as u8).collect();
                core.write_8(address, &buf).map_err(map_err)?;
            }
            AccessWidth::U16 => {
                let buf: Vec<u16> = data.iter().map(|v| *v as u16).collect();
                core.write_16(address, &buf).map_err(map_err)?;
            }
            AccessWidth::U32 => {
                let buf: Vec<u32> = data.iter().map(|v| *v as u32).collect();
                core.write_32(address, &buf).map_err(map_err)?;
            }
            AccessWidth::U64 => {
                let buf: Vec<u64> = data.to_vec();
                core.write_64(address, &buf).map_err(map_err)?;
            }
        }
        Ok(())
    }

    fn read_core_register(&mut self, reg: &CoreRegister) -> Result<u64, McpError> {
        let mut core = self.core()?;
        let id = Self::resolve_register(&core, reg)?;
        core.read_core_reg::<u32>(id)
            .map(|v| v as u64)
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))
    }

    fn write_core_register(&mut self, reg: &CoreRegister, value: u64) -> Result<(), McpError> {
        let mut core = self.core()?;
        let id = Self::resolve_register(&core, reg)?;
        core.write_core_reg(id, value as u32)
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))
    }

    fn halt(&mut self) -> Result<(), McpError> {
        self.core()?
            .halt(Duration::from_secs(1))
            .map(|_| ())
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))
    }

    fn resume(&mut self) -> Result<(), McpError> {
        self.core()?
            .run()
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))
    }

    fn step(&mut self) -> Result<(), McpError> {
        self.core()?
            .step()
            .map(|_| ())
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))
    }

    fn set_breakpoint(&mut self, address: u64) -> Result<(), McpError> {
        let mut core = self.core()?;
        core.set_hw_breakpoint(address)
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
        drop(core);
        if !self.breakpoints.contains(&address) {
            self.breakpoints.push(address);
            self.breakpoints.sort_unstable();
        }
        Ok(())
    }

    fn clear_breakpoints(&mut self) -> Result<(), McpError> {
        let addresses = self.breakpoints.clone();
        let mut core = self.core()?;
        for address in &addresses {
            core.clear_hw_breakpoint(*address)
                .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
        }
        drop(core);
        self.breakpoints.clear();
        Ok(())
    }

    fn list_breakpoints(&mut self) -> Result<Vec<u64>, McpError> {
        self.core()?;
        Ok(self.breakpoints.clone())
    }

    fn reset(&mut self) -> Result<(), McpError> {
        self.core()?
            .reset()
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))
    }

    fn read_dap(&mut self, address: u32) -> Result<u32, McpError> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| McpError::new(ErrorCode::NotConnected, "no active session"))?;
        let iface = session
            .get_arm_interface()
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
        let value = if (address >> 24) & 0xFF != 0 {
            let apsel = ((address >> 24) & 0xFF) as u8;
            let ap_addr = (address & 0xFF) as u64;
            iface
                .read_raw_ap_register(
                    &probe_rs::architecture::arm::FullyQualifiedApAddress::v1_with_default_dp(
                        apsel,
                    ),
                    ap_addr,
                )
                .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?
        } else {
            let dp_addr = probe_rs::architecture::arm::dp::DpRegisterAddress {
                address: (address & 0x0F) as u8,
                bank: Some(((address >> 4) & 0x0F) as u8),
            };
            iface
                .read_raw_dp_register(probe_rs::architecture::arm::dp::DpAddress::Default, dp_addr)
                .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?
        };
        Ok(value)
    }

    fn write_dap(&mut self, address: u32, value: u32) -> Result<(), McpError> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| McpError::new(ErrorCode::NotConnected, "no active session"))?;
        let iface = session
            .get_arm_interface()
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
        if (address >> 24) & 0xFF != 0 {
            let apsel = ((address >> 24) & 0xFF) as u8;
            let ap_addr = (address & 0xFF) as u64;
            iface
                .write_raw_ap_register(
                    &probe_rs::architecture::arm::FullyQualifiedApAddress::v1_with_default_dp(
                        apsel,
                    ),
                    ap_addr,
                    value,
                )
                .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?
        } else {
            let dp_addr = probe_rs::architecture::arm::dp::DpRegisterAddress {
                address: (address & 0x0F) as u8,
                bank: Some(((address >> 4) & 0x0F) as u8),
            };
            iface
                .write_raw_dp_register(
                    probe_rs::architecture::arm::dp::DpAddress::Default,
                    dp_addr,
                    value,
                )
                .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?
        }
        Ok(())
    }

    fn erase_flash(&mut self, _address: u64, _size: u64) -> Result<(), McpError> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| McpError::new(ErrorCode::NotConnected, "no active session"))?;
        let mut progress = probe_rs::flashing::FlashProgress::new(|_| {});
        probe_rs::flashing::erase_all(session, &mut progress, false).map_err(|e| {
            McpError::new(ErrorCode::ProtocolError, format!("flash erase failed: {e}"))
        })?;
        Ok(())
    }

    fn program_flash(&mut self, address: u64, data: &[u8]) -> Result<(), McpError> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| McpError::new(ErrorCode::NotConnected, "no active session"))?;
        let mut loader = probe_rs::flashing::FlashLoader::new(
            session.target().memory_map.clone(),
            session.target().source().clone(),
        );
        loader.add_data(address, data).map_err(|e| {
            McpError::new(ErrorCode::ProtocolError, format!("flash data invalid: {e}"))
        })?;
        loader
            .commit(session, probe_rs::flashing::DownloadOptions::default())
            .map_err(|e| {
                McpError::new(
                    ErrorCode::ProtocolError,
                    format!("flash programming failed: {e}"),
                )
            })?;
        Ok(())
    }
}
