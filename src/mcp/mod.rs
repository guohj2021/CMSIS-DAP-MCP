pub mod tools_memory;

use crate::error::ErrorCode;
use crate::security::SecurityPolicy;
use crate::session::SessionManager;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ServerCapabilities, ServerInfo};
use rmcp::{ServerHandler, tool, tool_router};
use std::sync::Mutex;

pub use tools_memory::{ReadMemoryParams, WriteMemoryParams};

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

pub struct CmsisDapMcp {
    tool_router: ToolRouter<Self>,
    pub session: Mutex<SessionManager>,
    pub policy: SecurityPolicy,
}

impl CmsisDapMcp {
    pub fn new(session: SessionManager, policy: SecurityPolicy) -> Self {
        Self {
            tool_router: Self::tool_router(),
            session: Mutex::new(session),
            policy,
        }
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
}

impl ServerHandler for CmsisDapMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(SERVER_INSTRUCTIONS)
    }
}