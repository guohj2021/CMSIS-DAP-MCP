use cmsis_dap_core::backend::mock::MockBackend;
use cmsis_dap_core::backend::{AccessWidth, Backend, ConnectOptions, Protocol};
use cmsis_dap_core::error::ErrorCode;

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
        core_index: None,
    })
    .unwrap();
    b.write_memory(0x2000_0000, AccessWidth::U32, &[0xDEAD_BEEF])
        .unwrap();
    let v = b.read_memory(0x2000_0000, AccessWidth::U32, 1).unwrap();
    assert_eq!(v, vec![0xDEAD_BEEF]);
}

#[test]
fn flash_breakpoint_patches_and_restores() {
    let mut b = MockBackend::new();
    b.connect(&ConnectOptions {
        probe_id: None,
        protocol: Protocol::Swd,
        speed_khz: None,
        target: None,
        under_reset: false,
        core_index: None,
    })
    .unwrap();
    // Put a known Thumb instruction (0x2000 = "movs r0, #0") in flash first.
    b.write_memory(0x0800_0000, AccessWidth::U16, &[0x2000])
        .unwrap();
    b.set_flash_breakpoint(0x0800_0000).unwrap();
    assert_eq!(b.list_flash_breakpoints().unwrap(), vec![0x0800_0000]);
    // BKPT bytes programmed into flash.
    let patched = b.read_memory(0x0800_0000, AccessWidth::U8, 2).unwrap();
    assert_eq!(patched, vec![0x00, 0xBE]);
    // Clear restores the original instruction.
    b.clear_flash_breakpoints().unwrap();
    let restored = b.read_memory(0x0800_0000, AccessWidth::U8, 2).unwrap();
    assert_eq!(restored, vec![0x00, 0x20]);
    assert!(b.list_flash_breakpoints().unwrap().is_empty());
}

#[test]
fn flash_breakpoint_requires_aligned_address() {
    let mut b = MockBackend::new();
    b.connect(&ConnectOptions {
        probe_id: None,
        protocol: Protocol::Swd,
        speed_khz: None,
        target: None,
        under_reset: false,
        core_index: None,
    })
    .unwrap();
    let err = b.set_flash_breakpoint(0x0800_0001).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidArgument);
}

#[test]
fn flash_breakpoint_requires_connection() {
    let mut b = MockBackend::new();
    let err = b.set_flash_breakpoint(0x0800_0000).unwrap_err();
    assert_eq!(err.code, ErrorCode::NotConnected);
}

#[test]
fn target_info_reports_cores() {
    let mut b = MockBackend::new();
    let info = b
        .connect(&ConnectOptions {
            probe_id: None,
            protocol: Protocol::Swd,
            speed_khz: None,
            target: None,
            under_reset: false,
            core_index: None,
        })
        .unwrap();
    assert_eq!(info.core_count, 1);
    assert_eq!(info.cores.len(), 1);
    assert_eq!(info.cores[0].index, 0);
    assert_eq!(info.cores[0].name, "main");
}

#[test]
fn flash_breakpoint_set_is_idempotent() {
    let mut b = MockBackend::new();
    b.connect(&ConnectOptions {
        probe_id: None,
        protocol: Protocol::Swd,
        speed_khz: None,
        target: None,
        under_reset: false,
        core_index: None,
    })
    .unwrap();
    b.write_memory(0x0800_0000, AccessWidth::U16, &[0x2000])
        .unwrap();
    b.set_flash_breakpoint(0x0800_0000).unwrap();
    b.set_flash_breakpoint(0x0800_0000).unwrap();
    assert_eq!(b.list_flash_breakpoints().unwrap(), vec![0x0800_0000]);
    b.clear_flash_breakpoints().unwrap();
    let restored = b.read_memory(0x0800_0000, AccessWidth::U8, 2).unwrap();
    assert_eq!(restored, vec![0x00, 0x20]);
}

#[test]
fn flash_breakpoint_rejects_non_nvm_address() {
    let mut b = MockBackend::new();
    b.connect(&ConnectOptions {
        probe_id: None,
        protocol: Protocol::Swd,
        speed_khz: None,
        target: None,
        under_reset: false,
        core_index: None,
    })
    .unwrap();
    let err = b.set_flash_breakpoint(0x2000_0000).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidArgument);
}

#[test]
fn flash_breakpoint_clear_is_idempotent() {
    let mut b = MockBackend::new();
    b.connect(&ConnectOptions {
        probe_id: None,
        protocol: Protocol::Swd,
        speed_khz: None,
        target: None,
        under_reset: false,
        core_index: None,
    })
    .unwrap();
    b.clear_flash_breakpoints().unwrap();
}

#[test]
fn flash_breakpoint_list_is_sorted() {
    let mut b = MockBackend::new();
    b.connect(&ConnectOptions {
        probe_id: None,
        protocol: Protocol::Swd,
        speed_khz: None,
        target: None,
        under_reset: false,
        core_index: None,
    })
    .unwrap();
    b.write_memory(0x0800_0004, AccessWidth::U16, &[0x2000])
        .unwrap();
    b.write_memory(0x0800_0000, AccessWidth::U16, &[0x2000])
        .unwrap();
    b.set_flash_breakpoint(0x0800_0004).unwrap();
    b.set_flash_breakpoint(0x0800_0000).unwrap();
    assert_eq!(
        b.list_flash_breakpoints().unwrap(),
        vec![0x0800_0000, 0x0800_0004]
    );
}

#[test]
fn connect_rejects_out_of_range_core() {
    let mut b = MockBackend::new();
    let err = b
        .connect(&ConnectOptions {
            probe_id: None,
            protocol: Protocol::Swd,
            speed_khz: None,
            target: None,
            under_reset: false,
            core_index: Some(1),
        })
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidArgument);
}
