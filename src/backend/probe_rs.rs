use crate::backend::{AccessWidth, Backend, ConnectOptions, CoreRegister, ProbeInfo, Protocol, TargetInfo};
use crate::error::{ErrorCode, McpError};
use probe_rs::probe::{list::Lister, Probe, WireProtocol};
use probe_rs::{MemoryInterface, Permissions, Session};

pub struct ProbeRsBackend {
    session: Option<Session>,
    core_index: usize,
}

impl ProbeRsBackend {
    pub fn new() -> Self {
        Self { session: None, core_index: 0 }
    }

    fn core(&mut self) -> Result<probe_rs::Core<'_>, McpError> {
        self.session
            .as_mut()
            .ok_or_else(|| McpError::new(ErrorCode::NotConnected, "no active session"))?
            .core(self.core_index)
            .map_err(|e| McpError::new(ErrorCode::ConnectFailed, e.to_string()))
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

    fn connect(&mut self, opts: &ConnectOptions) -> Result<TargetInfo, McpError> {
        if self.session.is_some() {
            self.disconnect()?;
        }
        let lister = Lister::new();
        let probes = lister.list_all();
        let selected = match &opts.probe_id {
            Some(id) => probes
                .iter()
                .find(|p| {
                    p.serial_number.as_deref() == Some(id.as_str())
                        || p.identifier == *id
                })
                .ok_or_else(|| McpError::new(ErrorCode::ProbeNotFound, format!("no probe with id {id}")))?,
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
        let session = match &opts.target {
            Some(name) => probe
                .attach(name, Permissions::default())
                .map_err(|e| McpError::new(ErrorCode::ConnectFailed, e.to_string()))?,
            None => {
                probe
                    .attach_to_unspecified()
                    .map_err(|e| McpError::new(ErrorCode::ConnectFailed, e.to_string()))?;
                probe
                    .attach(probe_rs::config::TargetSelector::Auto, Permissions::default())
                    .map_err(|e| McpError::new(ErrorCode::ConnectFailed, e.to_string()))?
            }
        };
        let core_type = session
            .target()
            .cores
            .first()
            .map(|c| format!("{:?}", c.core_type))
            .unwrap_or_else(|| "unknown".into());
        let ap_count = session.target().memory_map.len();
        self.session = Some(session);
        Ok(TargetInfo { core_type, ap_count })
    }

    fn disconnect(&mut self) -> Result<(), McpError> {
        self.session.take();
        Ok(())
    }

    fn read_memory(&mut self, address: u64, width: AccessWidth, count: u32) -> Result<Vec<u64>, McpError> {
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

    fn write_memory(&mut self, address: u64, width: AccessWidth, data: &[u64]) -> Result<(), McpError> {
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
                let buf: Vec<u64> = data.iter().map(|v| *v).collect();
                core.write_64(address, &buf).map_err(map_err)?;
            }
        }
        Ok(())
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