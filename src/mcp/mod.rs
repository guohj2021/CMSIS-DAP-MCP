pub mod tools_core;
pub mod tools_dap;
pub mod tools_memory;
pub mod tools_svd;

use crate::backend::CoreRegister;
use crate::error::ErrorCode;
use crate::security::SecurityPolicy;
use crate::session::SessionManager;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ServerCapabilities, ServerInfo};
use rmcp::{ServerHandler, tool, tool_router};
use std::sync::Mutex;

pub use tools_core::{
    ClearBreakpointsParams, HaltParams, ListBreakpointsParams, ReadCoreRegisterParams, ResetParams,
    ResumeParams, SetBreakpointParams, StepParams, WriteCoreRegisterParams,
};
pub use tools_dap::{ReadDapParams, WriteDapParams};
pub use tools_memory::{ReadMemoryParams, WriteMemoryParams};
pub use tools_svd::{ListPeripheralsParams, LoadSvdParams, ReadPeripheralParams, WritePeripheralParams};

pub const SERVER_INSTRUCTIONS: &str = "CMSIS-DAP MCP server for Cortex-M targets. \
Security tiers: ReadOnly tools (list_probes, get_target_info, read_memory, read_core_register, \
list_breakpoints, read_dap, list_peripherals, read_peripheral) are always available; Write tools \
(connect, disconnect, write_memory, write_core_register, halt, resume, step, set_breakpoint, \
clear_breakpoints, reset, write_dap, load_svd, write_peripheral) are marked write and governed by \
the MCP client approval policy; Destructive tools (erase_flash, program_flash) are disabled unless \
the server was started with --allow-destructive. Workflow: call list_probes, then connect with the \
probe id, then use memory/core tools. For named peripheral access, first call load_svd with a path \
to an SVD file provided by the user; chip-specific files are never bundled. All tools return \
structured JSON content. Logs go to stderr only.";

pub fn error_result(code: ErrorCode, message: String) -> CallToolResult {
    CallToolResult::structured_error(serde_json::json!({
        "code": format!("{code:?}"),
        "message": message,
    }))
}

fn register_params(name: Option<String>, number: Option<u16>) -> Result<CoreRegister, CallToolResult> {
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
    pub session: Mutex<SessionManager>,
    pub policy: SecurityPolicy,
}

impl CmsisDapMcp {
    pub fn new(session: SessionManager, policy: SecurityPolicy) -> Self {
        Self { session: Mutex::new(session), policy }
    }
}

#[tool_router]
impl CmsisDapMcp {
    #[tool(description = "Read memory from the connected target. width is one of u8/u16/u32/u64; count is the number of elements.", annotations(title = "Read memory", read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = false))]
    pub async fn read_memory(&self, Parameters(params): Parameters<ReadMemoryParams>) -> CallToolResult {
        let width = match tools_memory::parse_width(&params.width) {
            Some(w) => w,
            None => return error_result(ErrorCode::InvalidArgument, "width must be u8/u16/u32/u64".into()),
        };
        match self.session.lock().unwrap().backend().read_memory(params.address, width, params.count) {
            Ok(values) => CallToolResult::structured(serde_json::json!({
                "address": params.address,
                "width": params.width,
                "count": params.count,
                "values": values,
            })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(description = "Write memory on the connected target. width is one of u8/u16/u32/u64; values are the elements to write.", annotations(title = "Write memory", read_only_hint = false, destructive_hint = false, idempotent_hint = true, open_world_hint = false))]
    pub async fn write_memory(&self, Parameters(params): Parameters<WriteMemoryParams>) -> CallToolResult {
        let width = match tools_memory::parse_width(&params.width) {
            Some(w) => w,
            None => return error_result(ErrorCode::InvalidArgument, "width must be u8/u16/u32/u64".into()),
        };
        match self.session.lock().unwrap().backend().write_memory(params.address, width, &params.values) {
            Ok(()) => CallToolResult::structured(serde_json::json!({
                "address": params.address,
                "width": params.width,
                "written": params.values.len(),
            })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(description = "Read a core register by name (e.g. r0, sp, pc, xpsr) or by architecture-specific number. Provide exactly one of name or number.", annotations(title = "Read core register", read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = false))]
    pub async fn read_core_register(&self, Parameters(params): Parameters<ReadCoreRegisterParams>) -> CallToolResult {
        let reg = match register_params(params.name, params.number) {
            Ok(r) => r,
            Err(e) => return e,
        };
        match self.session.lock().unwrap().backend().read_core_register(&reg) {
            Ok(value) => CallToolResult::structured(serde_json::json!({ "register": format!("{reg:?}"), "value": value })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(description = "Write a core register by name or number. Provide exactly one of name or number.", annotations(title = "Write core register", read_only_hint = false, destructive_hint = false, idempotent_hint = true, open_world_hint = false))]
    pub async fn write_core_register(&self, Parameters(params): Parameters<WriteCoreRegisterParams>) -> CallToolResult {
        let reg = match register_params(params.name, params.number) {
            Ok(r) => r,
            Err(e) => return e,
        };
        match self.session.lock().unwrap().backend().write_core_register(&reg, params.value) {
            Ok(()) => CallToolResult::structured(serde_json::json!({ "register": format!("{reg:?}"), "value": params.value, "written": true })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(description = "Halt the connected core.", annotations(title = "Halt", read_only_hint = false, destructive_hint = false, idempotent_hint = false, open_world_hint = false))]
    pub async fn halt(&self, Parameters(_): Parameters<HaltParams>) -> CallToolResult {
        match self.session.lock().unwrap().backend().halt() {
            Ok(()) => CallToolResult::structured(serde_json::json!({ "halted": true })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(description = "Resume execution of the connected core.", annotations(title = "Resume", read_only_hint = false, destructive_hint = false, idempotent_hint = false, open_world_hint = false))]
    pub async fn resume(&self, Parameters(_): Parameters<ResumeParams>) -> CallToolResult {
        match self.session.lock().unwrap().backend().resume() {
            Ok(()) => CallToolResult::structured(serde_json::json!({ "running": true })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(description = "Single-step the connected core.", annotations(title = "Step", read_only_hint = false, destructive_hint = false, idempotent_hint = false, open_world_hint = false))]
    pub async fn step(&self, Parameters(_): Parameters<StepParams>) -> CallToolResult {
        match self.session.lock().unwrap().backend().step() {
            Ok(()) => CallToolResult::structured(serde_json::json!({ "stepped": true })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(description = "Set a hardware breakpoint at the given address.", annotations(title = "Set breakpoint", read_only_hint = false, destructive_hint = false, idempotent_hint = true, open_world_hint = false))]
    pub async fn set_breakpoint(&self, Parameters(params): Parameters<SetBreakpointParams>) -> CallToolResult {
        match self.session.lock().unwrap().backend().set_breakpoint(params.address) {
            Ok(()) => CallToolResult::structured(serde_json::json!({ "address": params.address, "set": true })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(description = "Clear all hardware breakpoints.", annotations(title = "Clear breakpoints", read_only_hint = false, destructive_hint = false, idempotent_hint = true, open_world_hint = false))]
    pub async fn clear_breakpoints(&self, Parameters(_): Parameters<ClearBreakpointsParams>) -> CallToolResult {
        match self.session.lock().unwrap().backend().clear_breakpoints() {
            Ok(()) => CallToolResult::structured(serde_json::json!({ "cleared": true })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(description = "List currently set hardware breakpoints.", annotations(title = "List breakpoints", read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = false))]
    pub async fn list_breakpoints(&self, Parameters(_): Parameters<ListBreakpointsParams>) -> CallToolResult {
        match self.session.lock().unwrap().backend().list_breakpoints() {
            Ok(addresses) => CallToolResult::structured(serde_json::json!({ "breakpoints": addresses })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(description = "Reset the connected target. Can interrupt running firmware.", annotations(title = "Reset", read_only_hint = false, destructive_hint = true, idempotent_hint = false, open_world_hint = false))]
    pub async fn reset(&self, Parameters(_): Parameters<ResetParams>) -> CallToolResult {
        match self.session.lock().unwrap().backend().reset() {
            Ok(()) => CallToolResult::structured(serde_json::json!({ "reset": true })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(description = "Read a raw DP or AP register. For AP access include APSEL in bits 24-31 (e.g. 0x010000FC); otherwise bits 0-7 are the DP register address.", annotations(title = "Read DAP register", read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = false))]
    pub async fn read_dap(&self, Parameters(params): Parameters<ReadDapParams>) -> CallToolResult {
        match self.session.lock().unwrap().backend().read_dap(params.address) {
            Ok(value) => CallToolResult::structured(serde_json::json!({ "address": params.address, "value": value })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(description = "Write a raw DP or AP register. For AP access include APSEL in bits 24-31 (e.g. 0x010000FC); otherwise bits 0-7 are the DP register address.", annotations(title = "Write DAP register", read_only_hint = false, destructive_hint = false, idempotent_hint = true, open_world_hint = false))]
    pub async fn write_dap(&self, Parameters(params): Parameters<WriteDapParams>) -> CallToolResult {
        match self.session.lock().unwrap().backend().write_dap(params.address, params.value) {
            Ok(()) => CallToolResult::structured(serde_json::json!({ "address": params.address, "value": params.value, "written": true })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(description = "Load an SVD file (user-provided path) for named peripheral access.", annotations(title = "Load SVD", read_only_hint = false, destructive_hint = false, idempotent_hint = false, open_world_hint = false))]
    pub async fn load_svd(&self, Parameters(params): Parameters<LoadSvdParams>) -> CallToolResult {
        match self.session.lock().unwrap().load_svd(std::path::Path::new(&params.path)) {
            Ok(summary) => CallToolResult::structured(serde_json::json!({
                "name": summary.name,
                "peripherals": summary.peripherals,
            })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(description = "List peripherals from the loaded SVD.", annotations(title = "List peripherals", read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = false))]
    pub async fn list_peripherals(&self, Parameters(_): Parameters<ListPeripheralsParams>) -> CallToolResult {
        match self.session.lock().unwrap().svd() {
            Ok(db) => CallToolResult::structured(serde_json::json!({ "peripherals": db.list_peripherals() })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(description = "Read a peripheral register (or one bit field of it) by name from the loaded SVD.", annotations(title = "Read peripheral register", read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = false))]
    pub async fn read_peripheral(&self, Parameters(params): Parameters<ReadPeripheralParams>) -> CallToolResult {
        let mut session = self.session.lock().unwrap();
        let db = match session.svd() {
            Ok(db) => db.clone(),
            Err(e) => return error_result(e.code, e.message),
        };
        let (addr, field) = match db.resolve(&params.peripheral, &params.register, params.field.as_deref()) {
            Ok(v) => v,
            Err(e) => return error_result(e.code, e.message),
        };
        match session.backend().read_memory(addr, crate::backend::AccessWidth::U32, 1) {
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

    #[tool(description = "Write a peripheral register (or one bit field of it) by name from the loaded SVD. Field writes are read-modify-write.", annotations(title = "Write peripheral register", read_only_hint = false, destructive_hint = false, idempotent_hint = true, open_world_hint = false))]
    pub async fn write_peripheral(&self, Parameters(params): Parameters<WritePeripheralParams>) -> CallToolResult {
        let mut session = self.session.lock().unwrap();
        let db = match session.svd() {
            Ok(db) => db.clone(),
            Err(e) => return error_result(e.code, e.message),
        };
        let (addr, field) = match db.resolve(&params.peripheral, &params.register, params.field.as_deref()) {
            Ok(v) => v,
            Err(e) => return error_result(e.code, e.message),
        };
        let result = match field {
            Some((mask, shift)) => {
                let current = match session.backend().read_memory(addr, crate::backend::AccessWidth::U32, 1) {
                    Ok(values) => values[0],
                    Err(e) => return error_result(e.code, e.message),
                };
                let updated = (current & !((mask as u64) << shift)) | ((params.value & mask as u64) << shift);
                session.backend().write_memory(addr, crate::backend::AccessWidth::U32, &[updated])
            }
            None => session.backend().write_memory(addr, crate::backend::AccessWidth::U32, &[params.value]),
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
}
#[rmcp::tool_handler(router = Self::tool_router())]
impl ServerHandler for CmsisDapMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(SERVER_INSTRUCTIONS)
    }
}