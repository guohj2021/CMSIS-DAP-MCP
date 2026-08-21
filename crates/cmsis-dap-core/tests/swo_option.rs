use cmsis_dap_core::backend::mock::MockBackend;
use cmsis_dap_core::backend::{Backend, ConnectOptions, OptionByte, Protocol};
use cmsis_dap_core::error::ErrorCode;

fn connect(b: &mut MockBackend) {
    b.connect(&ConnectOptions {
        probe_id: None,
        protocol: Protocol::Swd,
        speed_khz: None,
        target: None,
        under_reset: false,
    })
    .unwrap();
}

#[test]
fn swo_start_stop_read_flow() {
    let mut b = MockBackend::new();
    connect(&mut b);
    b.start_swo(2_000_000, 8_000_000).unwrap();
    // The mock returns a fixed predictable trace once SWO is active.
    assert_eq!(b.read_swo_data().unwrap(), vec![0x01, 0x02, 0x03]);
    b.stop_swo().unwrap();
    let err = b.read_swo_data().unwrap_err();
    assert_eq!(err.code, ErrorCode::NotConnected);
}

#[test]
fn swo_requires_connection() {
    let mut b = MockBackend::new();
    let err = b.start_swo(2_000_000, 8_000_000).unwrap_err();
    assert_eq!(err.code, ErrorCode::NotConnected);
}

#[test]
fn read_swo_before_start_errors() {
    let mut b = MockBackend::new();
    connect(&mut b);
    let err = b.read_swo_data().unwrap_err();
    assert_eq!(err.code, ErrorCode::NotConnected);
}

#[test]
fn option_bytes_read_write_roundtrip() {
    let mut b = MockBackend::new();
    connect(&mut b);
    let bytes = b.read_option_bytes().unwrap();
    assert_eq!(bytes.len(), 1);
    assert_eq!(bytes[0].name, "RDP");
    assert_eq!(bytes[0].value, 0xAA);
    b.write_option_bytes(&[OptionByte {
        name: "DATA0".into(),
        address: 0x4002_3C14,
        value: 0x55,
        description: None,
    }])
    .unwrap();
}

#[test]
fn option_bytes_require_connection() {
    let mut b = MockBackend::new();
    let err = b.read_option_bytes().unwrap_err();
    assert_eq!(err.code, ErrorCode::NotConnected);
}
