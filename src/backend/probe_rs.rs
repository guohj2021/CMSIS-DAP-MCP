use crate::backend::{
    register_hint, AccessWidth, Backend, ConnectOptions, CoreRegister, CoreStatusInfo,
    ExportFormat, ImageFileFormat, MemoryMismatch, MemoryRegionSummary, MemoryVerifyReport,
    ProbeInfo, Protocol, RegisterHint, ResetMode, TargetInfo, WatchAccess, Watchpoint,
};
use crate::error::{ErrorCode, McpError};
use probe_rs::config::MemoryRegion;
use probe_rs::flashing::{
    build_loader, erase, erase_all, image_format, DownloadOptions, FlashProgress,
};
use probe_rs::probe::{list::Lister, WireProtocol};
use probe_rs::{CoreStatus, MemoryInterface, Permissions, RegisterRole, Session};
use std::collections::BTreeSet;
use std::time::Duration;

fn file_error<E: std::fmt::Display>(e: E) -> McpError {
    McpError::new(ErrorCode::FileError, e.to_string())
}

pub struct ProbeRsBackend {
    session: Option<Session>,
    core_index: usize,
    breakpoints: Vec<u64>,
    watchpoints: Vec<Watchpoint>,
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
            watchpoints: Vec::new(),
            registry: None,
        }
    }

    pub fn with_registry(registry: probe_rs::config::Registry) -> Self {
        Self {
            session: None,
            core_index: 0,
            breakpoints: Vec::new(),
            watchpoints: Vec::new(),
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
                let regs = core.registers();
                let found = match register_hint(name) {
                    RegisterHint::ProgramCounter => Some(core.program_counter()),
                    RegisterHint::StackPointer => Some(core.stack_pointer()),
                    RegisterHint::FramePointer => Some(core.frame_pointer()),
                    RegisterHint::ReturnAddress => Some(core.return_address()),
                    RegisterHint::ProcessorStatus => regs.psr(),
                    RegisterHint::MainStackPointer => regs.msp(),
                    RegisterHint::ProcessStackPointer => regs.psp(),
                    RegisterHint::FpuStatus => regs.fpsr(),
                    RegisterHint::GeneralIndex(index) => regs.get_core_register(index),
                    RegisterHint::ByName => regs
                        .all_registers()
                        .find(|r| r.name().eq_ignore_ascii_case(name))
                        .or_else(|| regs.other_by_name(name)),
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

    fn register_names(core: &probe_rs::Core<'_>) -> Vec<String> {
        let mut names = BTreeSet::new();
        for r in core.registers().all_registers() {
            let primary = r.name();
            if primary != "Unknown" {
                names.insert(primary.to_ascii_lowercase());
            }
            for role in r.roles {
                match role {
                    RegisterRole::ProgramCounter => {
                        names.insert("pc".to_string());
                    }
                    RegisterRole::StackPointer => {
                        names.insert("sp".to_string());
                    }
                    RegisterRole::MainStackPointer => {
                        names.insert("msp".to_string());
                    }
                    RegisterRole::ProcessStackPointer => {
                        names.insert("psp".to_string());
                    }
                    RegisterRole::ProcessorStatus => {
                        names.insert("xpsr".to_string());
                        names.insert("psr".to_string());
                    }
                    RegisterRole::ReturnAddress => {
                        names.insert("lr".to_string());
                        names.insert("ra".to_string());
                    }
                    RegisterRole::FramePointer => {
                        names.insert("fp".to_string());
                    }
                    RegisterRole::Other(n) => {
                        names.insert(n.to_ascii_lowercase());
                    }
                    RegisterRole::Core(n) => {
                        names.insert(n.to_ascii_lowercase());
                    }
                    RegisterRole::Argument(n) | RegisterRole::Return(n) => {
                        names.insert(n.to_ascii_lowercase());
                    }
                    RegisterRole::FloatingPoint | RegisterRole::FloatingPointStatus => {}
                }
            }
        }
        names.into_iter().collect()
    }
}

impl Backend for ProbeRsBackend {
    fn list_probes(&self) -> Result<Vec<ProbeInfo>, McpError> {
        let lister = Lister::new();
        let probes = lister.list_all();
        Ok(probes
            .into_iter()
            .map(|p| {
                let mut protocols = Vec::new();
                let mut speed_khz = None;
                let mut target_voltage = None;
                if let Ok(mut probe) = lister.open(&p) {
                    if probe.select_protocol(WireProtocol::Swd).is_ok() {
                        protocols.push("swd".into());
                    }
                    if probe.select_protocol(WireProtocol::Jtag).is_ok() {
                        protocols.push("jtag".into());
                    }
                    speed_khz = Some(probe.speed_khz());
                    target_voltage = probe.get_target_voltage().ok().flatten();
                }
                ProbeInfo {
                    id: p
                        .serial_number
                        .clone()
                        .unwrap_or_else(|| p.identifier.clone()),
                    vendor: format!("{:04x}", p.vendor_id),
                    product: p.identifier.clone(),
                    serial: p.serial_number.clone(),
                    product_id: Some(format!("{:04x}", p.product_id)),
                    interface: p.interface,
                    is_hid: p.is_hid_interface,
                    protocols,
                    speed_khz,
                    target_voltage,
                }
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
                let attach_result = if opts.under_reset {
                    probe.attach_under_reset_with_registry(target, Permissions::default(), registry)
                } else {
                    probe.attach_with_registry(target, Permissions::default(), registry)
                };
                attach_result.map_err(|e| McpError::new(ErrorCode::ConnectFailed, e.to_string()))
            };
        let mut session = match (&self.registry, &opts.target) {
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
        let core_count = session.target().cores.len();
        let ap_count = match session.get_arm_interface() {
            Ok(iface) => iface
                .access_ports(probe_rs::architecture::arm::dp::DpAddress::Default)
                .map(|aps| aps.len())
                .unwrap_or(0),
            Err(_) => 0,
        };
        let cpu_id = match session.core(0) {
            Ok(mut core) => {
                let mut v = [0u32];
                core.read_32(0xE000_ED00, &mut v).ok().map(|_| v[0])
            }
            Err(_) => None,
        };
        let dp_id = match session.get_arm_interface() {
            Ok(iface) => iface
                .read_raw_dp_register(
                    probe_rs::architecture::arm::dp::DpAddress::Default,
                    probe_rs::architecture::arm::dp::DpRegisterAddress {
                        address: 0,
                        bank: None,
                    },
                )
                .ok(),
            Err(_) => None,
        };
        let memory_regions = session
            .target()
            .memory_map
            .iter()
            .filter_map(|region| {
                if let Some(r) = region.as_nvm_region() {
                    Some(MemoryRegionSummary {
                        name: r.name.clone().unwrap_or_else(|| "NVM".into()),
                        kind: "nvm".into(),
                        start: r.range.start,
                        end: r.range.end,
                    })
                } else {
                    region.as_ram_region().map(|r| MemoryRegionSummary {
                        name: r.name.clone().unwrap_or_else(|| "RAM".into()),
                        kind: "ram".into(),
                        start: r.range.start,
                        end: r.range.end,
                    })
                }
            })
            .collect();
        self.breakpoints.clear();
        self.watchpoints.clear();
        self.session = Some(session);
        Ok(TargetInfo {
            core_type,
            core_count,
            ap_count,
            cpu_id,
            dp_id,
            memory_regions,
        })
    }

    fn disconnect(&mut self) -> Result<(), McpError> {
        self.session.take();
        self.breakpoints.clear();
        self.watchpoints.clear();
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

    fn reset(&mut self, mode: ResetMode) -> Result<(), McpError> {
        let mut core = self.core()?;
        let result = match mode {
            ResetMode::Run => core.reset().map(|_| ()),
            ResetMode::Halt => core.reset_and_halt(Duration::from_secs(1)).map(|_| ()),
        };
        result.map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))
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

    fn erase_flash(&mut self, address: u64, size: u64) -> Result<(), McpError> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| McpError::new(ErrorCode::NotConnected, "no active session"))?;
        if size == 0 {
            return Err(McpError::new(
                ErrorCode::InvalidArgument,
                "erase size must be greater than zero",
            ));
        }
        let end = address
            .checked_add(size)
            .ok_or_else(|| McpError::new(ErrorCode::InvalidArgument, "erase range overflows"))?;
        let mut progress = FlashProgress::new(|_| {});
        let covers_all = session
            .target()
            .memory_map
            .iter()
            .filter_map(MemoryRegion::as_nvm_region)
            .filter(|r| !r.is_alias)
            .all(|r| address <= r.range.start && r.range.end <= end);
        let result = if covers_all {
            erase_all(session, &mut progress, false)
        } else {
            erase(session, &mut progress, address, end, false)
        };
        result.map_err(|e| {
            McpError::new(ErrorCode::ProtocolError, format!("flash erase failed: {e}"))
        })
    }

    fn program_flash(&mut self, address: u64, data: &[u8], verify: bool) -> Result<(), McpError> {
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
        let mut options = DownloadOptions::default();
        options.verify = verify;
        loader.commit(session, options).map_err(|e| {
            McpError::new(
                ErrorCode::ProtocolError,
                format!("flash programming failed: {e}"),
            )
        })?;
        Ok(())
    }

    fn list_core_registers(&mut self) -> Result<Vec<String>, McpError> {
        let core = self.core()?;
        Ok(Self::register_names(&core))
    }

    fn get_core_status(&mut self) -> Result<CoreStatusInfo, McpError> {
        let mut core = self.core()?;
        let status = core
            .status()
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
        let (state, halt_reason, pc) = match status {
            CoreStatus::Running => ("running", None, None),
            CoreStatus::Halted(reason) => (
                "halted",
                Some(format!("{reason:?}")),
                core.read_core_reg::<u32>(core.program_counter().id())
                    .ok()
                    .map(|v| v as u64),
            ),
            CoreStatus::LockedUp => ("locked_up", None, None),
            CoreStatus::Sleeping => ("sleeping", None, None),
            CoreStatus::Unknown => ("unknown", None, None),
        };
        Ok(CoreStatusInfo {
            state: state.into(),
            halt_reason,
            pc,
        })
    }

    fn set_watchpoint(&mut self, address: u64, access: WatchAccess) -> Result<(), McpError> {
        const DEMCR: u64 = 0xE000_EDFC;
        const TRCENA: u32 = 1 << 24;
        const DWT_CTRL: u64 = 0xE000_1000;
        const DWT_COMP_BASE: u64 = 0xE000_1020;

        if let Some(wp) = self.watchpoints.iter_mut().find(|w| w.address == address) {
            wp.access = access;
            return Ok(());
        }
        let mut core = self.core()?;
        let mut demcr = [0u32];
        core.read_32(DEMCR, &mut demcr)
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
        core.write_32(DEMCR, &[demcr[0] | TRCENA])
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
        let mut ctrl = [0u32];
        core.read_32(DWT_CTRL, &mut ctrl)
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
        let numcomp = ((ctrl[0] >> 28) & 0xF) as usize;
        if numcomp == 0 {
            return Err(McpError::new(
                ErrorCode::UnsupportedFeature,
                "target has no DWT watchpoint comparators",
            ));
        }
        let slot = (0..numcomp).find(|n| {
            let function_addr = DWT_COMP_BASE + (*n as u64) * 0x10 + 0x8;
            let mut v = [0u32];
            core.read_32(function_addr, &mut v)
                .map(|_| v[0] & 0xF == 0)
                .unwrap_or(false)
        });
        let n = slot.ok_or_else(|| {
            McpError::new(
                ErrorCode::UnsupportedFeature,
                "no free DWT watchpoint comparator",
            )
        })?;
        let base = DWT_COMP_BASE + n as u64 * 0x10;
        core.write_32(base, &[address as u32])
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
        core.write_32(base + 0x4, &[0])
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
        let function = match access {
            WatchAccess::Read => 5,
            WatchAccess::Write => 4,
            WatchAccess::ReadWrite => 6,
        };
        let mut current = [0u32];
        core.read_32(base + 0x8, &mut current)
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
        core.write_32(base + 0x8, &[(current[0] & !0xF) | function])
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
        drop(core);
        self.watchpoints.push(Watchpoint { address, access });
        Ok(())
    }

    fn clear_watchpoints(&mut self) -> Result<(), McpError> {
        const DWT_CTRL: u64 = 0xE000_1000;
        const DWT_COMP_BASE: u64 = 0xE000_1020;
        let mut core = self.core()?;
        let mut ctrl = [0u32];
        core.read_32(DWT_CTRL, &mut ctrl)
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
        let numcomp = ((ctrl[0] >> 28) & 0xF) as usize;
        for n in 0..numcomp {
            core.write_32(DWT_COMP_BASE + n as u64 * 0x10 + 0x8, &[0])
                .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
        }
        drop(core);
        self.watchpoints.clear();
        Ok(())
    }

    fn list_watchpoints(&mut self) -> Result<Vec<Watchpoint>, McpError> {
        self.core()?;
        Ok(self.watchpoints.clone())
    }

    fn verify_memory(
        &mut self,
        address: u64,
        width: AccessWidth,
        data: &[u64],
    ) -> Result<MemoryVerifyReport, McpError> {
        let actual = self.read_memory(address, width, data.len() as u32)?;
        let width_bytes = match width {
            AccessWidth::U8 => 1u64,
            AccessWidth::U16 => 2,
            AccessWidth::U32 => 4,
            AccessWidth::U64 => 8,
        };
        let mismatches: Vec<MemoryMismatch> = data
            .iter()
            .enumerate()
            .zip(actual.iter())
            .filter(|((_, expected), actual)| expected != actual)
            .map(|((index, expected), actual)| MemoryMismatch {
                index,
                address: address + index as u64 * width_bytes,
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
        format: ImageFileFormat,
        address: u64,
        verify: bool,
    ) -> Result<u64, McpError> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| McpError::new(ErrorCode::NotConnected, "no active session"))?;
        let mut options = DownloadOptions::default();
        options.verify = verify;
        match format {
            ImageFileFormat::Bin => {
                let data = std::fs::read(path).map_err(file_error)?;
                let mut loader = probe_rs::flashing::FlashLoader::new(
                    session.target().memory_map.clone(),
                    session.target().source().clone(),
                );
                loader.add_data(address, &data).map_err(file_error)?;
                loader.commit(session, options).map_err(file_error)?;
                Ok(data.len() as u64)
            }
            ImageFileFormat::Elf | ImageFileFormat::Axf => {
                let factory =
                    image_format("elf").ok_or_else(|| file_error("elf format is unavailable"))?;
                let loader = build_loader(session, path, factory.create_loader(None), None)
                    .map_err(file_error)?;
                loader.commit(session, options).map_err(file_error)?;
                Ok(0)
            }
            ImageFileFormat::Hex => {
                let factory =
                    image_format("hex").ok_or_else(|| file_error("hex format is unavailable"))?;
                let loader = build_loader(session, path, factory.create_loader(None), None)
                    .map_err(file_error)?;
                loader.commit(session, options).map_err(file_error)?;
                Ok(0)
            }
        }
    }

    fn export_memory(
        &mut self,
        path: &std::path::Path,
        format: ExportFormat,
        address: u64,
        size: u64,
    ) -> Result<u64, McpError> {
        if size == 0 {
            return Err(McpError::new(
                ErrorCode::InvalidArgument,
                "export size must be greater than zero",
            ));
        }
        let mut bytes = Vec::with_capacity(size as usize);
        let mut remaining = size;
        let mut addr = address;
        while remaining > 0 {
            let chunk = remaining.min(4096) as u32;
            let values = self.read_memory(addr, AccessWidth::U8, chunk)?;
            bytes.extend(values.iter().map(|v| *v as u8));
            addr += chunk as u64;
            remaining -= chunk as u64;
        }
        match format {
            ExportFormat::Bin => std::fs::write(path, &bytes).map_err(file_error)?,
            ExportFormat::Hex => std::fs::write(path, crate::hex::encode_ihex(&bytes, address))
                .map_err(file_error)?,
        }
        Ok(size)
    }
}
