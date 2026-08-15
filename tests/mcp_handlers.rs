use cmsis_dap_mcp::backend::mock::MockBackend;
use cmsis_dap_mcp::backend::{ConnectOptions, Protocol};
use cmsis_dap_mcp::mcp::{
    ClearBreakpointsParams, CmsisDapMcp, HaltParams, ListBreakpointsParams, ReadCoreRegisterParams,
    ReadMemoryParams, ResetParams, ResumeParams, SetBreakpointParams, StepParams,
    WriteCoreRegisterParams, WriteMemoryParams,
};
use cmsis_dap_mcp::security::SecurityPolicy;
use cmsis_dap_mcp::session::SessionManager;
use rmcp::handler::server::wrapper::Parameters;

fn connect(mcp: &CmsisDapMcp) {
    let opts = ConnectOptions { probe_id: None, protocol: Protocol::Swd, speed_khz: None, target: None };
    mcp.session.lock().unwrap().connect(&opts).unwrap();
}

#[tokio::test]
async fn read_memory_returns_mock_values() {
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), SecurityPolicy { allow_destructive: false });
    connect(&mcp);
    let params = ReadMemoryParams { address: 0x2000_0000, width: "u32".into(), count: 1 };
    let res = mcp.read_memory(Parameters(params)).await;
    assert!(!res.is_error.unwrap_or(true));
    let structured = res.structured_content.unwrap();
    assert_eq!(structured["values"][0].as_u64(), Some(0));
}

#[tokio::test]
async fn write_then_read_memory() {
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), SecurityPolicy { allow_destructive: false });
    connect(&mcp);
    let write = WriteMemoryParams { address: 0x2000_0000, width: "u32".into(), values: vec![0xDEAD_BEEF] };
    let res = mcp.write_memory(Parameters(write)).await;
    assert!(!res.is_error.unwrap_or(true));
    let read = ReadMemoryParams { address: 0x2000_0000, width: "u32".into(), count: 1 };
    let res = mcp.read_memory(Parameters(read)).await;
    let structured = res.structured_content.unwrap();
    assert_eq!(structured["values"][0].as_u64(), Some(0xDEAD_BEEF));
}

#[tokio::test]
async fn core_control_flow_with_mock() {
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), SecurityPolicy { allow_destructive: false });
    connect(&mcp);
    assert!(!mcp.halt(Parameters(HaltParams {})).await.is_error.unwrap_or(true));
    assert!(!mcp.resume(Parameters(ResumeParams {})).await.is_error.unwrap_or(true));
    assert!(!mcp.step(Parameters(StepParams {})).await.is_error.unwrap_or(true));
    assert!(!mcp.set_breakpoint(Parameters(SetBreakpointParams { address: 0x0800_0000 })).await.is_error.unwrap_or(true));
    let res = mcp.list_breakpoints(Parameters(ListBreakpointsParams {})).await;
    assert_eq!(res.structured_content.unwrap()["breakpoints"][0].as_u64(), Some(0x0800_0000));
    assert!(!mcp.clear_breakpoints(Parameters(ClearBreakpointsParams {})).await.is_error.unwrap_or(true));
    let res = mcp.list_breakpoints(Parameters(ListBreakpointsParams {})).await;
    assert_eq!(res.structured_content.unwrap()["breakpoints"].as_array().unwrap().len(), 0);
    assert!(!mcp.reset(Parameters(ResetParams {})).await.is_error.unwrap_or(true));
}

#[tokio::test]
async fn core_register_roundtrip_with_mock() {
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), SecurityPolicy { allow_destructive: false });
    connect(&mcp);
    let write = WriteCoreRegisterParams { name: Some("r0".into()), number: None, value: 0x1234 };
    assert!(!mcp.write_core_register(Parameters(write)).await.is_error.unwrap_or(true));
    let read = ReadCoreRegisterParams { name: Some("r0".into()), number: None };
    let res = mcp.read_core_register(Parameters(read)).await;
    assert_eq!(res.structured_content.unwrap()["value"].as_u64(), Some(0x1234));
}

#[test]
fn instructions_are_self_contained() {
    assert!(cmsis_dap_mcp::mcp::SERVER_INSTRUCTIONS.len() >= 512);
}