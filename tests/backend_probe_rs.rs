use cmsis_dap_mcp::backend::probe_rs::ProbeRsBackend;
use cmsis_dap_mcp::backend::{AccessWidth, Backend};
use cmsis_dap_mcp::error::ErrorCode;

#[test]
fn memory_read_without_connect_fails() {
    let mut b = ProbeRsBackend::new();
    let err = b.read_memory(0x2000_0000, AccessWidth::U32, 1).unwrap_err();
    assert_eq!(err.code, ErrorCode::NotConnected);
}
