pub mod tools_chip;
pub mod tools_config;
pub mod tools_core;
pub mod tools_dap;
pub mod tools_flash;
pub mod tools_memory;
pub mod tools_option;
pub mod tools_probe;
pub mod tools_script;
pub mod tools_svd;
pub mod tools_swo;

use crate::runtime::ServerRuntime;
use cmsis_dap_core::backend::{
    CoreRegister, ExportFormat, ImageFileFormat, OptionByte, Protocol, ResetMode, WatchAccess,
};
use cmsis_dap_core::error::ErrorCode;
use cmsis_dap_core::security::SecurityPolicy;
use cmsis_dap_core::session::SessionManager;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_router, ServerHandler};
use std::sync::Arc;

pub use tools_chip::DefineChipParams;
pub use tools_config::{GetConfigParams, ReloadConfigParams, UpdateConfigParams};
pub use tools_core::{
    ClearBreakpointsParams, ClearWatchpointsParams, DumpCpuStateParams, GetCoreStatusParams,
    HaltParams, ListBreakpointsParams, ListCoreRegistersParams, ListWatchpointsParams,
    ReadCoreRegisterParams, ResetParams, ResumeParams, SetBreakpointParams, SetWatchpointParams,
    StepParams, WriteCoreRegisterParams,
};
pub use tools_dap::{ReadDapParams, WriteDapParams};
pub use tools_flash::{EraseFlashParams, ProgramFlashParams};
pub use tools_memory::{ReadMemoryParams, VerifyMemoryParams, WriteMemoryParams};
pub use tools_option::{ReadOptionBytesParams, WriteOptionBytesParams};
pub use tools_probe::{
    ConnectParams, DisconnectParams, GetProbeInfoParams, GetTargetInfoParams, ListProbesParams,
};
pub use tools_script::RunScriptParams;
pub use tools_svd::{
    ListPeripheralsParams, LoadSvdParams, ReadPeripheralParams, WritePeripheralParams,
};
pub use tools_swo::{ReadSwoParams, StartSwoParams, StopSwoParams};

pub const SERVER_INSTRUCTIONS: &str = "CMSIS-DAP MCP server for Cortex-M targets. \
Security tiers: ReadOnly tools (list_probes, get_probe_info, get_target_info, read_memory, \
read_core_register, list_core_registers, list_breakpoints, get_core_status, read_dap, \
list_watchpoints, list_peripherals, read_peripheral, verify_memory) are always available; Write \
tools (connect, disconnect, write_memory, write_core_register, halt, resume, step, set_breakpoint, \
clear_breakpoints, reset, write_dap, set_watchpoint, clear_watchpoints, load_svd, \
write_peripheral) are marked write and governed by the MCP client approval policy; Destructive \
tools (erase_flash, program_flash) are disabled unless the server was started with \
--allow-destructive OR runtime configuration enabled them. Runtime configuration: the server can \
be started with NO flags (a to-be-configured state where all read/write tools work and destructive \
tools are gated). To enable destructive tools or the TCP/GDB servers at runtime, call \
update_config (allow_destructive, tcp_port, gdb_port) or reload_config (if --config-file was given \
at startup; the file is also watched and re-applied automatically on edit). get_config reports the \
current settings. All config changes take effect immediately without a restart. dump_cpu_state takes \
a non-invasive CPU snapshot (never resets; it briefly halts to read registers and restores the \
previous run state afterwards; memory and fault-status registers are read without halting). \
Workflow: call list_probes, then connect (protocol swd or jtag, optionally under_reset) with the \
probe id, then use memory/core tools; reset accepts mode run or halt; program_flash accepts raw \
data or a firmware file (axf/elf/bin/hex) with verify for read-back checking; read_memory can \
export a range as bin or hex when given a path; run_script executes J-Link Commander / OpenOCD \
style scripts (destructive script commands require allow_destructive). For named peripheral access, \
first call load_svd with a path to an SVD file provided by the user; chip-specific files are never \
bundled. All tools return structured JSON content. Logs go to stderr only.";

pub fn error_result(code: ErrorCode, message: String) -> CallToolResult {
    CallToolResult::structured_error(serde_json::json!({
        "code": format!("{code:?}"),
        "message": message,
    }))
}

fn register_params(
    name: Option<String>,
    number: Option<u16>,
) -> Result<CoreRegister, CallToolResult> {
    match (name, number) {
        (Some(n), None) => Ok(CoreRegister::Name(n)),
        (None, Some(n)) => Ok(CoreRegister::Number(n)),
        _ => Err(error_result(
            ErrorCode::InvalidArgument,
            "provide exactly one of name or number".into(),
        )),
    }
}

pub struct CmsisDapMcp {
    pub runtime: Arc<ServerRuntime>,
}

impl CmsisDapMcp {
    pub fn new(session: SessionManager, allow_destructive: bool) -> Self {
        Self {
            runtime: ServerRuntime::new(session, allow_destructive),
        }
    }

    /// Build a server around a runtime shared with other endpoints
    /// (e.g. the remote TCP server and the config file watcher).
    pub fn from_shared(runtime: Arc<ServerRuntime>) -> Self {
        Self { runtime }
    }

    /// Check the destructive gate against the *current* runtime config. Unlike
    /// a cached `SecurityPolicy`, this reflects changes made via `update_config`
    /// or a reloaded config file without a restart.
    fn require_destructive(&self) -> Result<(), CallToolResult> {
        if self.runtime.config.read().unwrap().allow_destructive {
            Ok(())
        } else {
            Err(error_result(
                ErrorCode::DestructiveDisabled,
                "destructive tools are disabled. Enable them at runtime by calling \
                 update_config with allow_destructive=true, by editing the watched \
                 --config-file, or by starting the server with --allow-destructive."
                    .into(),
            ))
        }
    }
}

#[tool_router]
impl CmsisDapMcp {
    #[tool(
        description = "Read memory from the connected target. width is one of u8/u16/u32/u64; count is the number of elements.",
        annotations(
            title = "Read memory",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn read_memory(
        &self,
        Parameters(params): Parameters<ReadMemoryParams>,
    ) -> CallToolResult {
        if let Some(path) = &params.path {
            let format = match params.format.as_deref() {
                None | Some("bin") => ExportFormat::Bin,
                Some("hex") | Some("ihex") | Some("intelhex") => ExportFormat::Hex,
                Some(other) => {
                    return error_result(
                        ErrorCode::InvalidArgument,
                        format!("export format must be bin or hex, got {other}"),
                    )
                }
            };
            if params.count == 0 {
                return error_result(
                    ErrorCode::InvalidArgument,
                    "export count must be greater than zero".into(),
                );
            }
            match self
                .runtime
                .session
                .lock()
                .unwrap()
                .backend()
                .export_memory(
                    std::path::Path::new(path),
                    format,
                    params.address,
                    params.count as u64,
                ) {
                Ok(bytes) => CallToolResult::structured(serde_json::json!({
                    "exported": true,
                    "path": path,
                    "format": format.as_str(),
                    "address": params.address,
                    "bytes": bytes,
                })),
                Err(e) => error_result(e.code, e.message),
            }
        } else {
            let width = match tools_memory::parse_width(&params.width) {
                Some(w) => w,
                None => {
                    return error_result(
                        ErrorCode::InvalidArgument,
                        "width must be u8/u16/u32/u64".into(),
                    )
                }
            };
            match self.runtime.session.lock().unwrap().backend().read_memory(
                params.address,
                width,
                params.count,
            ) {
                Ok(values) => CallToolResult::structured(serde_json::json!({
                    "address": params.address,
                    "width": params.width,
                    "count": params.count,
                    "values": values,
                })),
                Err(e) => error_result(e.code, e.message),
            }
        }
    }

    #[tool(
        description = "Write memory on the connected target. width is one of u8/u16/u32/u64; values are the elements to write.",
        annotations(
            title = "Write memory",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn write_memory(
        &self,
        Parameters(params): Parameters<WriteMemoryParams>,
    ) -> CallToolResult {
        let width = match tools_memory::parse_width(&params.width) {
            Some(w) => w,
            None => {
                return error_result(
                    ErrorCode::InvalidArgument,
                    "width must be u8/u16/u32/u64".into(),
                )
            }
        };
        match self.runtime.session.lock().unwrap().backend().write_memory(
            params.address,
            width,
            &params.values,
        ) {
            Ok(()) => CallToolResult::structured(serde_json::json!({
                "address": params.address,
                "width": params.width,
                "written": params.values.len(),
            })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Read a core register by name (e.g. r0, sp, pc, xpsr) or by architecture-specific number. Provide exactly one of name or number.",
        annotations(
            title = "Read core register",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn read_core_register(
        &self,
        Parameters(params): Parameters<ReadCoreRegisterParams>,
    ) -> CallToolResult {
        let reg = match register_params(params.name, params.number) {
            Ok(r) => r,
            Err(e) => return e,
        };
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .read_core_register(&reg)
        {
            Ok(value) => CallToolResult::structured(
                serde_json::json!({ "register": format!("{reg:?}"), "value": value }),
            ),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Write a core register by name or number. Provide exactly one of name or number.",
        annotations(
            title = "Write core register",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn write_core_register(
        &self,
        Parameters(params): Parameters<WriteCoreRegisterParams>,
    ) -> CallToolResult {
        let reg = match register_params(params.name, params.number) {
            Ok(r) => r,
            Err(e) => return e,
        };
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .write_core_register(&reg, params.value)
        {
            Ok(()) => CallToolResult::structured(
                serde_json::json!({ "register": format!("{reg:?}"), "value": params.value, "written": true }),
            ),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Halt the connected core.",
        annotations(
            title = "Halt",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    pub async fn halt(&self, Parameters(_): Parameters<HaltParams>) -> CallToolResult {
        match self.runtime.session.lock().unwrap().backend().halt() {
            Ok(()) => CallToolResult::structured(serde_json::json!({ "halted": true })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Resume execution of the connected core.",
        annotations(
            title = "Resume",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    pub async fn resume(&self, Parameters(_): Parameters<ResumeParams>) -> CallToolResult {
        match self.runtime.session.lock().unwrap().backend().resume() {
            Ok(()) => CallToolResult::structured(serde_json::json!({ "running": true })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Single-step the connected core.",
        annotations(
            title = "Step",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    pub async fn step(&self, Parameters(_): Parameters<StepParams>) -> CallToolResult {
        match self.runtime.session.lock().unwrap().backend().step() {
            Ok(()) => CallToolResult::structured(serde_json::json!({ "stepped": true })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Set a hardware breakpoint at the given address.",
        annotations(
            title = "Set breakpoint",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn set_breakpoint(
        &self,
        Parameters(params): Parameters<SetBreakpointParams>,
    ) -> CallToolResult {
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .set_breakpoint(params.address)
        {
            Ok(()) => CallToolResult::structured(
                serde_json::json!({ "address": params.address, "set": true }),
            ),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Clear all hardware breakpoints.",
        annotations(
            title = "Clear breakpoints",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn clear_breakpoints(
        &self,
        Parameters(_): Parameters<ClearBreakpointsParams>,
    ) -> CallToolResult {
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .clear_breakpoints()
        {
            Ok(()) => CallToolResult::structured(serde_json::json!({ "cleared": true })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "List currently set hardware breakpoints.",
        annotations(
            title = "List breakpoints",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn list_breakpoints(
        &self,
        Parameters(_): Parameters<ListBreakpointsParams>,
    ) -> CallToolResult {
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .list_breakpoints()
        {
            Ok(addresses) => {
                CallToolResult::structured(serde_json::json!({ "breakpoints": addresses }))
            }
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Reset the connected target. Can interrupt running firmware.",
        annotations(
            title = "Reset",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    pub async fn reset(&self, Parameters(params): Parameters<ResetParams>) -> CallToolResult {
        let mode = match params.mode.as_deref() {
            None | Some("run") => ResetMode::Run,
            Some("halt") => ResetMode::Halt,
            Some(other) => {
                return error_result(
                    ErrorCode::InvalidArgument,
                    format!("mode must be run or halt, got {other}"),
                )
            }
        };
        match self.runtime.session.lock().unwrap().backend().reset(mode) {
            Ok(()) => CallToolResult::structured(serde_json::json!({
                "reset": true,
                "mode": if mode == ResetMode::Run { "run" } else { "halt" },
            })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Read a raw DP or AP register. For AP access include APSEL in bits 24-31 (e.g. 0x010000FC); otherwise bits 0-7 are the DP register address.",
        annotations(
            title = "Read DAP register",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn read_dap(&self, Parameters(params): Parameters<ReadDapParams>) -> CallToolResult {
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .read_dap(params.address)
        {
            Ok(value) => CallToolResult::structured(
                serde_json::json!({ "address": params.address, "value": value }),
            ),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Write a raw DP or AP register. For AP access include APSEL in bits 24-31 (e.g. 0x010000FC); otherwise bits 0-7 are the DP register address.",
        annotations(
            title = "Write DAP register",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn write_dap(
        &self,
        Parameters(params): Parameters<WriteDapParams>,
    ) -> CallToolResult {
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .write_dap(params.address, params.value)
        {
            Ok(()) => CallToolResult::structured(
                serde_json::json!({ "address": params.address, "value": params.value, "written": true }),
            ),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Load an SVD file (user-provided path) for named peripheral access.",
        annotations(
            title = "Load SVD",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    pub async fn load_svd(&self, Parameters(params): Parameters<LoadSvdParams>) -> CallToolResult {
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .load_svd(std::path::Path::new(&params.path))
        {
            Ok(summary) => CallToolResult::structured(serde_json::json!({
                "name": summary.name,
                "peripherals": summary.peripherals,
            })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "List peripherals from the loaded SVD.",
        annotations(
            title = "List peripherals",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn list_peripherals(
        &self,
        Parameters(_): Parameters<ListPeripheralsParams>,
    ) -> CallToolResult {
        match self.runtime.session.lock().unwrap().svd() {
            Ok(db) => CallToolResult::structured(
                serde_json::json!({ "peripherals": db.list_peripherals() }),
            ),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Read a peripheral register (or one bit field of it) by name from the loaded SVD.",
        annotations(
            title = "Read peripheral register",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn read_peripheral(
        &self,
        Parameters(params): Parameters<ReadPeripheralParams>,
    ) -> CallToolResult {
        let mut session = self.runtime.session.lock().unwrap();
        let db = match session.svd() {
            Ok(db) => db.clone(),
            Err(e) => return error_result(e.code, e.message),
        };
        let (addr, field) = match db.resolve(
            &params.peripheral,
            &params.register,
            params.field.as_deref(),
        ) {
            Ok(v) => v,
            Err(e) => return error_result(e.code, e.message),
        };
        match session
            .backend()
            .read_memory(addr, cmsis_dap_core::backend::AccessWidth::U32, 1)
        {
            Ok(values) => {
                let raw = values[0];
                let value = match field {
                    Some((mask, shift)) => (raw & mask as u64) >> shift,
                    None => raw,
                };
                CallToolResult::structured(serde_json::json!({
                    "peripheral": params.peripheral,
                    "register": params.register,
                    "address": addr,
                    "value": value,
                }))
            }
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Write a peripheral register (or one bit field of it) by name from the loaded SVD. Field writes are read-modify-write.",
        annotations(
            title = "Write peripheral register",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn write_peripheral(
        &self,
        Parameters(params): Parameters<WritePeripheralParams>,
    ) -> CallToolResult {
        let mut session = self.runtime.session.lock().unwrap();
        let db = match session.svd() {
            Ok(db) => db.clone(),
            Err(e) => return error_result(e.code, e.message),
        };
        let (addr, field) = match db.resolve(
            &params.peripheral,
            &params.register,
            params.field.as_deref(),
        ) {
            Ok(v) => v,
            Err(e) => return error_result(e.code, e.message),
        };
        let result = match field {
            Some((mask, shift)) => {
                let current = match session.backend().read_memory(
                    addr,
                    cmsis_dap_core::backend::AccessWidth::U32,
                    1,
                ) {
                    Ok(values) => values[0],
                    Err(e) => return error_result(e.code, e.message),
                };
                let updated =
                    (current & !((mask as u64) << shift)) | ((params.value & mask as u64) << shift);
                session.backend().write_memory(
                    addr,
                    cmsis_dap_core::backend::AccessWidth::U32,
                    &[updated],
                )
            }
            None => session.backend().write_memory(
                addr,
                cmsis_dap_core::backend::AccessWidth::U32,
                &[params.value],
            ),
        };
        match result {
            Ok(()) => CallToolResult::structured(serde_json::json!({
                "peripheral": params.peripheral,
                "register": params.register,
                "address": addr,
                "written": true,
            })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Erase flash memory on the target. Destructive: requires --allow-destructive. The current backend performs a full-chip erase.",
        annotations(
            title = "Erase flash",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    pub async fn erase_flash(
        &self,
        Parameters(params): Parameters<EraseFlashParams>,
    ) -> CallToolResult {
        if let Err(e) = self.require_destructive() {
            return e;
        }
        if let Err(e) = self.runtime.session.lock().unwrap().require_flash_defined() {
            return error_result(e.code, e.message);
        }
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .erase_flash(params.address, params.size)
        {
            Ok(()) => CallToolResult::structured(
                serde_json::json!({ "erased": true, "address": params.address, "size": params.size }),
            ),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Program binary data into flash memory. Destructive: requires --allow-destructive. Requires a target with a flash algorithm loaded via connect(target=...).",
        annotations(
            title = "Program flash",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    pub async fn program_flash(
        &self,
        Parameters(params): Parameters<ProgramFlashParams>,
    ) -> CallToolResult {
        if let Err(e) = self.require_destructive() {
            return e;
        }
        if let Err(e) = self.runtime.session.lock().unwrap().require_flash_defined() {
            return error_result(e.code, e.message);
        }
        match (&params.data, &params.path) {
            (Some(_), Some(_)) => error_result(
                ErrorCode::InvalidArgument,
                "provide exactly one of data or path".into(),
            ),
            (None, None) => error_result(ErrorCode::InvalidArgument, "provide data or path".into()),
            (Some(data), None) => {
                match self
                    .runtime
                    .session
                    .lock()
                    .unwrap()
                    .backend()
                    .program_flash(params.address, data, params.verify.unwrap_or(false))
                {
                    Ok(()) => CallToolResult::structured(serde_json::json!({
                        "programmed": true,
                        "address": params.address,
                        "bytes": data.len(),
                        "verify": params.verify.unwrap_or(false),
                    })),
                    Err(e) => error_result(e.code, e.message),
                }
            }
            (None, Some(path)) => {
                let format = match params.format.as_deref() {
                    None => match ImageFileFormat::from_extension(std::path::Path::new(path)) {
                        Some(f) => f,
                        None => {
                            return error_result(
                                ErrorCode::InvalidArgument,
                                format!("cannot infer file format from extension of {path}"),
                            )
                        }
                    },
                    Some(name) => match ImageFileFormat::parse(name) {
                        Some(f) => f,
                        None => {
                            return error_result(
                                ErrorCode::InvalidArgument,
                                format!("unsupported file format {name}"),
                            )
                        }
                    },
                };
                match self.runtime.session.lock().unwrap().backend().program_file(
                    std::path::Path::new(path),
                    format,
                    params.address,
                    params.verify.unwrap_or(false),
                ) {
                    Ok(bytes) => CallToolResult::structured(serde_json::json!({
                        "programmed": true,
                        "path": path,
                        "format": format.as_str(),
                        "address": params.address,
                        "bytes": bytes,
                        "verify": params.verify.unwrap_or(false),
                    })),
                    Err(e) => error_result(e.code, e.message),
                }
            }
        }
    }

    #[tool(
        description = "Run a debug script using a J-Link Commander / OpenOCD style command subset. Provide exactly one of path or script. Destructive commands (erase, loadbin, loadfile, flash write_image, and w8/w16/w32 writes to NVM) require --allow-destructive.",
        annotations(
            title = "Run script",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    pub async fn run_script(
        &self,
        Parameters(params): Parameters<RunScriptParams>,
    ) -> CallToolResult {
        let text = match (&params.path, &params.script) {
            (Some(_), Some(_)) => {
                return error_result(
                    ErrorCode::InvalidArgument,
                    "provide exactly one of path or script".into(),
                )
            }
            (None, None) => {
                return error_result(ErrorCode::InvalidArgument, "provide path or script".into())
            }
            (Some(path), None) => match std::fs::read_to_string(path) {
                Ok(text) => text,
                Err(e) => {
                    return error_result(ErrorCode::FileError, e.to_string());
                }
            },
            (None, Some(script)) => script.clone(),
        };
        let mut session = self.runtime.session.lock().unwrap();
        let policy = SecurityPolicy {
            allow_destructive: self.runtime.config.read().unwrap().allow_destructive,
        };
        match cmsis_dap_core::script::run(&mut session, &policy, &text) {
            Ok(report) => {
                CallToolResult::structured(serde_json::to_value(&report).unwrap_or_default())
            }
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "List the core registers available on the connected target.",
        annotations(
            title = "List core registers",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn list_core_registers(
        &self,
        Parameters(_): Parameters<ListCoreRegistersParams>,
    ) -> CallToolResult {
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .list_core_registers()
        {
            Ok(registers) => {
                CallToolResult::structured(serde_json::json!({ "registers": registers }))
            }
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Get the execution status of the connected core (running, halted, sleeping, locked up or unknown), the halt reason, and the program counter when halted.",
        annotations(
            title = "Get core status",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn get_core_status(
        &self,
        Parameters(_): Parameters<GetCoreStatusParams>,
    ) -> CallToolResult {
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .get_core_status()
        {
            Ok(status) => CallToolResult::structured(serde_json::json!({
                "state": status.state,
                "halt_reason": status.halt_reason,
                "pc": status.pc,
            })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Take a non-invasive CPU state snapshot: registers, Cortex-M fault status registers, MSP/PSP stack words and optional memory samples. Never resets; briefly halts to read core registers and restores the previous run state afterwards (set restore=false to leave the core halted).",
        annotations(
            title = "Dump CPU state",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn dump_cpu_state(
        &self,
        Parameters(params): Parameters<DumpCpuStateParams>,
    ) -> CallToolResult {
        let addresses = params.addresses.unwrap_or_default();
        let stack_words = params.stack_words.unwrap_or(16) as usize;
        let restore = params.restore.unwrap_or(true);
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .dump_cpu_state(&addresses, stack_words, restore)
        {
            Ok(dump) => match serde_json::to_value(dump) {
                Ok(value) => CallToolResult::structured(value),
                Err(e) => error_result(ErrorCode::InternalError, e.to_string()),
            },
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Set a data watchpoint on the given address. access is read, write or rw.",
        annotations(
            title = "Set watchpoint",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn set_watchpoint(
        &self,
        Parameters(params): Parameters<SetWatchpointParams>,
    ) -> CallToolResult {
        let access = match WatchAccess::parse(&params.access) {
            Some(access) => access,
            None => {
                return error_result(
                    ErrorCode::InvalidArgument,
                    "access must be read, write or rw".into(),
                )
            }
        };
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .set_watchpoint(params.address, access)
        {
            Ok(()) => CallToolResult::structured(serde_json::json!({
                "address": params.address,
                "access": access,
                "set": true,
            })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Clear all data watchpoints.",
        annotations(
            title = "Clear watchpoints",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn clear_watchpoints(
        &self,
        Parameters(_): Parameters<ClearWatchpointsParams>,
    ) -> CallToolResult {
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .clear_watchpoints()
        {
            Ok(()) => CallToolResult::structured(serde_json::json!({ "cleared": true })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "List currently set data watchpoints.",
        annotations(
            title = "List watchpoints",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn list_watchpoints(
        &self,
        Parameters(_): Parameters<ListWatchpointsParams>,
    ) -> CallToolResult {
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .list_watchpoints()
        {
            Ok(watchpoints) => {
                CallToolResult::structured(serde_json::json!({ "watchpoints": watchpoints }))
            }
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Read back memory at the given address and compare it against the expected data. width is one of u8/u16/u32/u64.",
        annotations(
            title = "Verify memory",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn verify_memory(
        &self,
        Parameters(params): Parameters<VerifyMemoryParams>,
    ) -> CallToolResult {
        let width = match tools_memory::parse_width(&params.width) {
            Some(w) => w,
            None => {
                return error_result(
                    ErrorCode::InvalidArgument,
                    "width must be u8/u16/u32/u64".into(),
                )
            }
        };
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .verify_memory(params.address, width, &params.data)
        {
            Ok(report) => CallToolResult::structured(serde_json::json!({
                "address": params.address,
                "width": params.width,
                "verified": report.verified,
                "mismatches": report.mismatches,
            })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "List connected CMSIS-DAP debug probes.",
        annotations(
            title = "List probes",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn list_probes(&self, Parameters(_): Parameters<ListProbesParams>) -> CallToolResult {
        match self.runtime.session.lock().unwrap().backend().list_probes() {
            Ok(probes) => CallToolResult::structured(serde_json::json!({ "probes": probes })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Get information about one probe by id (or the first probe if omitted).",
        annotations(
            title = "Get probe info",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn get_probe_info(
        &self,
        Parameters(params): Parameters<GetProbeInfoParams>,
    ) -> CallToolResult {
        let probes = match self.runtime.session.lock().unwrap().backend().list_probes() {
            Ok(probes) => probes,
            Err(e) => return error_result(e.code, e.message),
        };
        let probe = match &params.probe_id {
            Some(id) => probes
                .iter()
                .find(|p| p.id == *id || p.serial.as_deref() == Some(id.as_str())),
            None => probes.first(),
        };
        match probe {
            Some(info) => CallToolResult::structured(serde_json::json!({ "probe": info })),
            None => error_result(
                ErrorCode::ProbeNotFound,
                format!("no probe with id {:?}", params.probe_id),
            ),
        }
    }

    #[tool(
        description = "Connect to a target through a probe. protocol is swd (default) or jtag; target is optional (generic Cortex-M if omitted).",
        annotations(
            title = "Connect",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    pub async fn connect(&self, Parameters(params): Parameters<ConnectParams>) -> CallToolResult {
        let protocol = match params.protocol.as_deref() {
            None | Some("swd") => Protocol::Swd,
            Some("jtag") => Protocol::Jtag,
            Some(other) => {
                return error_result(
                    ErrorCode::InvalidArgument,
                    format!("protocol must be swd or jtag, got {other}"),
                )
            }
        };
        let opts = cmsis_dap_core::backend::ConnectOptions {
            probe_id: params.probe_id,
            protocol,
            speed_khz: params.speed_khz,
            target: params.target,
            under_reset: params.under_reset.unwrap_or(false),
        };
        match self.runtime.session.lock().unwrap().connect(&opts) {
            Ok(info) => CallToolResult::structured(serde_json::json!({ "target": info })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Disconnect from the target.",
        annotations(
            title = "Disconnect",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn disconnect(&self, Parameters(_): Parameters<DisconnectParams>) -> CallToolResult {
        match self.runtime.session.lock().unwrap().disconnect() {
            Ok(()) => CallToolResult::structured(serde_json::json!({ "disconnected": true })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Get information about the connected target (core type and memory regions).",
        annotations(
            title = "Get target info",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn get_target_info(
        &self,
        Parameters(_): Parameters<GetTargetInfoParams>,
    ) -> CallToolResult {
        let session = self.runtime.session.lock().unwrap();
        match session.target_info() {
            Some(info) => CallToolResult::structured(serde_json::json!({ "target": info })),
            None => error_result(
                ErrorCode::NotConnected,
                "no active session; call connect first".into(),
            ),
        }
    }

    #[tool(
        description = "Get the current server runtime configuration: allow_destructive (gates flash erase/program and destructive scripts), tcp_port, gdb_port, and the source config_file.",
        annotations(
            title = "Get config",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn get_config(&self, Parameters(_): Parameters<GetConfigParams>) -> CallToolResult {
        let cfg = self.runtime.config.read().unwrap().clone();
        CallToolResult::structured(cfg.to_json())
    }

    #[tool(
        description = "Update the server runtime configuration WITHOUT restarting. Omit a field to keep its current value. allow_destructive enables flash erase/program and destructive script commands. tcp_port/gdb_port start (or, for TCP, move) the remote/GDB servers. Invalid values are rejected and the config is left unchanged.",
        annotations(
            title = "Update config",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    pub async fn update_config(
        &self,
        Parameters(params): Parameters<UpdateConfigParams>,
    ) -> CallToolResult {
        match self
            .runtime
            .update(params.allow_destructive, params.tcp_port, params.gdb_port)
        {
            Ok(cfg) => CallToolResult::structured(serde_json::json!({
                "updated": true,
                "config": cfg.to_json(),
            })),
            Err(e) => error_result(ErrorCode::ConfigError, e),
        }
    }

    #[tool(
        description = "Re-read the config file supplied at startup (--config-file) and apply it. Fails with a clear message if no config file was given, the file is missing, or its contents are invalid. Edits to the watched file also take effect automatically.",
        annotations(
            title = "Reload config",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    pub async fn reload_config(
        &self,
        Parameters(_): Parameters<ReloadConfigParams>,
    ) -> CallToolResult {
        match self.runtime.apply_config_file() {
            Ok(cfg) => CallToolResult::structured(serde_json::json!({
                "reloaded": true,
                "config": cfg.to_json(),
            })),
            Err(e) => error_result(ErrorCode::ConfigError, e),
        }
    }

    #[tool(
        description = "Define a custom/unknown chip at runtime from a Keil FLM flash algorithm so connect can attach to it by name. Supply the FLM path plus the Flash/SRAM address ranges, and optionally a core type (default armv6m) and chip name (default: FLM file stem). The generated probe-rs target YAML is registered in the running server; no standalone probe-rs CLI is used. SVD peripheral descriptions are loaded separately via load_svd.",
        annotations(
            title = "Define chip",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    pub async fn define_chip(
        &self,
        Parameters(params): Parameters<DefineChipParams>,
    ) -> CallToolResult {
        let flm_path = std::path::Path::new(&params.flm);
        let name = params.name.clone().unwrap_or_else(|| {
            flm_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "chip".to_string())
        });
        match cmsis_dap_core::backend::chip::generate_target_yaml(
            flm_path,
            params.flash_start,
            params.flash_size,
            params.sram_start,
            params.sram_size,
            &params.core,
            &name,
        ) {
            Ok(yaml) => {
                match self
                    .runtime
                    .session
                    .lock()
                    .unwrap()
                    .backend()
                    .define_target(&yaml)
                {
                    Ok(()) => CallToolResult::structured(serde_json::json!({
                        "defined": true,
                        "name": name,
                        "note": "call connect with this name to attach; load an SVD separately via load_svd for peripheral access",
                    })),
                    Err(e) => error_result(e.code, e.message),
                }
            }
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Start SWO/SWV trace with the given baud rate and TPIU clock frequency.",
        annotations(
            title = "Start SWO",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    pub async fn start_swo(
        &self,
        Parameters(params): Parameters<StartSwoParams>,
    ) -> CallToolResult {
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .start_swo(params.baud, params.tpiu_clk)
        {
            Ok(()) => CallToolResult::structured(serde_json::json!({
                "started": true,
                "baud": params.baud,
                "tpiu_clk": params.tpiu_clk,
            })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Stop SWO/SWV trace.",
        annotations(
            title = "Stop SWO",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    pub async fn stop_swo(&self, Parameters(_): Parameters<StopSwoParams>) -> CallToolResult {
        match self.runtime.session.lock().unwrap().backend().stop_swo() {
            Ok(()) => CallToolResult::structured(serde_json::json!({ "stopped": true })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Read available SWO/SWV trace bytes. Returns hex-encoded data.",
        annotations(
            title = "Read SWO data",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    pub async fn read_swo(&self, Parameters(_): Parameters<ReadSwoParams>) -> CallToolResult {
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .read_swo_data()
        {
            Ok(data) => {
                let hex: String = data.iter().map(|b| format!("{b:02x}")).collect();
                CallToolResult::structured(serde_json::json!({
                    "bytes": data.len(),
                    "data_hex": hex,
                }))
            }
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Read chip option bytes (STM32: RDP, USER, DATA0, DATA1 from FLASH_OPTCR). Chip-family specific.",
        annotations(
            title = "Read option bytes",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn read_option_bytes(
        &self,
        Parameters(_): Parameters<ReadOptionBytesParams>,
    ) -> CallToolResult {
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .read_option_bytes()
        {
            Ok(bytes) => CallToolResult::structured(serde_json::json!({ "option_bytes": bytes })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(
        description = "Write chip option bytes (STM32: RDP, USER, DATA0, DATA1). DESTRUCTIVE: can lock the device.",
        annotations(
            title = "Write option bytes",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    pub async fn write_option_bytes(
        &self,
        Parameters(params): Parameters<WriteOptionBytesParams>,
    ) -> CallToolResult {
        if let Err(e) = self.require_destructive() {
            return e;
        }
        let option_bytes: Vec<OptionByte> = params
            .bytes
            .into_iter()
            .map(|b| OptionByte {
                name: b.name,
                address: b.address,
                value: b.value,
                description: None,
            })
            .collect();
        match self
            .runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .write_option_bytes(&option_bytes)
        {
            Ok(()) => CallToolResult::structured(serde_json::json!({ "written": true })),
            Err(e) => error_result(e.code, e.message),
        }
    }
}

#[rmcp::tool_handler(router = Self::tool_router())]
impl ServerHandler for CmsisDapMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(SERVER_INSTRUCTIONS)
    }
}
