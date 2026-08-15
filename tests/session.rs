use cmsis_dap_mcp::backend::mock::MockBackend;
use cmsis_dap_mcp::backend::{AccessWidth, Backend, ConnectOptions, Protocol};
use cmsis_dap_mcp::error::ErrorCode;
use cmsis_dap_mcp::session::SessionManager;

#[test]
fn connect_sets_state_and_auto_disconnects() {
    let mut sm = SessionManager::new(Box::new(MockBackend::new()));
    let opts = ConnectOptions { probe_id: None, protocol: Protocol::Swd, speed_khz: None, target: None };
    sm.connect(&opts).unwrap();
    sm.connect(&opts).unwrap();
    sm.ensure_connected().unwrap();
}

#[test]
fn memory_before_connect_fails() {
    let mut sm = SessionManager::new(Box::new(MockBackend::new()));
    let err = sm.backend().read_memory(0, AccessWidth::U32, 1).unwrap_err();
    assert_eq!(err.code, ErrorCode::NotConnected);
}