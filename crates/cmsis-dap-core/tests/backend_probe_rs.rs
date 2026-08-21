use cmsis_dap_core::backend::probe_rs::ProbeRsBackend;
use cmsis_dap_core::backend::{AccessWidth, Backend};
use cmsis_dap_core::error::ErrorCode;

#[test]
fn memory_read_without_connect_fails() {
    let mut b = ProbeRsBackend::new();
    let err = b.read_memory(0x2000_0000, AccessWidth::U32, 1).unwrap_err();
    assert_eq!(err.code, ErrorCode::NotConnected);
}

use cmsis_dap_core::backend::Protocol;
use cmsis_dap_core::session::SessionManager;

#[test]
#[ignore = "requires a physical CMSIS-DAP probe and Cortex-M target"]
fn hardware_connect_halt_read_resume() {
    let mut sm = SessionManager::new(Box::new(ProbeRsBackend::new()));
    let opts = cmsis_dap_core::backend::ConnectOptions {
        probe_id: None,
        protocol: Protocol::Swd,
        speed_khz: Some(100),
        target: None,
        under_reset: false,
        core_index: None,
    };
    let info = sm.connect(&opts).expect("connect to first probe");
    assert!(!info.core_type.is_empty());
    sm.backend().halt().expect("halt core");
    let values = sm
        .backend()
        .read_memory(0x2000_0000, AccessWidth::U32, 4)
        .expect("read RAM");
    assert_eq!(values.len(), 4);
    sm.backend().resume().expect("resume core");

    if let Ok(path) = std::env::var("CMSIS_DAP_MCP_SVD_TEST") {
        sm.load_svd(std::path::Path::new(&path)).expect("load SVD");
        let db = sm.svd().expect("svd loaded");
        assert!(db
            .list_peripherals()
            .iter()
            .any(|p| p.eq_ignore_ascii_case("GPIOA")));
        let (addr, _field) = db.resolve("GPIOA", "ODR", None).expect("resolve GPIOA.ODR");
        let raw = sm
            .backend()
            .read_memory(addr, AccessWidth::U32, 1)
            .expect("read GPIOA.ODR");
        assert_eq!(raw.len(), 1);
    }
}
