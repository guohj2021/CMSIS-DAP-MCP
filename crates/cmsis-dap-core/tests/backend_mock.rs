use cmsis_dap_core::backend::mock::MockBackend;
use cmsis_dap_core::backend::{AccessWidth, Backend, ConnectOptions, Protocol};

#[test]
fn mock_lists_one_probe() {
    let b = MockBackend::new();
    assert_eq!(b.list_probes().unwrap().len(), 1);
}

#[test]
fn mock_memory_roundtrip() {
    let mut b = MockBackend::new();
    b.connect(&ConnectOptions {
        probe_id: None,
        protocol: Protocol::Swd,
        speed_khz: None,
        target: None,
        under_reset: false,
    })
    .unwrap();
    b.write_memory(0x2000_0000, AccessWidth::U32, &[0xDEAD_BEEF])
        .unwrap();
    let v = b.read_memory(0x2000_0000, AccessWidth::U32, 1).unwrap();
    assert_eq!(v, vec![0xDEAD_BEEF]);
}
