use cmsis_dap_core::backend::mock::MockBackend;
use cmsis_dap_core::backend::{ConnectOptions, Protocol};
use cmsis_dap_core::security::SecurityPolicy;
use cmsis_dap_core::session::SessionManager;
use cmsis_dap_mcp::mcp::{
    ClearBreakpointsParams, CmsisDapMcp, ConnectParams, DisconnectParams, EraseFlashParams,
    GetProbeInfoParams, GetTargetInfoParams, HaltParams, ListBreakpointsParams, ListProbesParams,
    ProgramFlashParams, ReadCoreRegisterParams, ReadDapParams, ReadMemoryParams, ResetParams,
    ResumeParams, SetBreakpointParams, StepParams, WriteCoreRegisterParams, WriteDapParams,
    WriteMemoryParams,
};
use rmcp::handler::server::wrapper::Parameters;

fn connect(mcp: &CmsisDapMcp) {
    let opts = ConnectOptions {
        probe_id: None,
        protocol: Protocol::Swd,
        speed_khz: None,
        target: None,
        under_reset: false,
    };
    mcp.session.lock().unwrap().connect(&opts).unwrap();
}

#[tokio::test]
async fn read_memory_returns_mock_values() {
    let mcp = CmsisDapMcp::new(
        SessionManager::new(Box::new(MockBackend::new())),
        SecurityPolicy {
            allow_destructive: false,
        },
    );
    connect(&mcp);
    let params = ReadMemoryParams {
        address: 0x2000_0000,
        width: "u32".into(),
        count: 1,
        path: None,
        format: None,
    };
    let res = mcp.read_memory(Parameters(params)).await;
    assert!(!res.is_error.unwrap_or(true));
    let structured = res.structured_content.unwrap();
    assert_eq!(structured["values"][0].as_u64(), Some(0));
}

#[tokio::test]
async fn write_then_read_memory() {
    let mcp = CmsisDapMcp::new(
        SessionManager::new(Box::new(MockBackend::new())),
        SecurityPolicy {
            allow_destructive: false,
        },
    );
    connect(&mcp);
    let write = WriteMemoryParams {
        address: 0x2000_0000,
        width: "u32".into(),
        values: vec![0xDEAD_BEEF],
    };
    let res = mcp.write_memory(Parameters(write)).await;
    assert!(!res.is_error.unwrap_or(true));
    let read = ReadMemoryParams {
        address: 0x2000_0000,
        width: "u32".into(),
        count: 1,
        path: None,
        format: None,
    };
    let res = mcp.read_memory(Parameters(read)).await;
    let structured = res.structured_content.unwrap();
    assert_eq!(structured["values"][0].as_u64(), Some(0xDEAD_BEEF));
}

#[tokio::test]
async fn core_control_flow_with_mock() {
    let mcp = CmsisDapMcp::new(
        SessionManager::new(Box::new(MockBackend::new())),
        SecurityPolicy {
            allow_destructive: false,
        },
    );
    connect(&mcp);
    assert!(!mcp
        .halt(Parameters(HaltParams {}))
        .await
        .is_error
        .unwrap_or(true));
    assert!(!mcp
        .resume(Parameters(ResumeParams {}))
        .await
        .is_error
        .unwrap_or(true));
    assert!(!mcp
        .step(Parameters(StepParams {}))
        .await
        .is_error
        .unwrap_or(true));
    assert!(!mcp
        .set_breakpoint(Parameters(SetBreakpointParams {
            address: 0x0800_0000
        }))
        .await
        .is_error
        .unwrap_or(true));
    let res = mcp
        .list_breakpoints(Parameters(ListBreakpointsParams {}))
        .await;
    assert_eq!(
        res.structured_content.unwrap()["breakpoints"][0].as_u64(),
        Some(0x0800_0000)
    );
    assert!(!mcp
        .clear_breakpoints(Parameters(ClearBreakpointsParams {}))
        .await
        .is_error
        .unwrap_or(true));
    let res = mcp
        .list_breakpoints(Parameters(ListBreakpointsParams {}))
        .await;
    assert_eq!(
        res.structured_content.unwrap()["breakpoints"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert!(!mcp
        .reset(Parameters(ResetParams { mode: None }))
        .await
        .is_error
        .unwrap_or(true));
}

#[tokio::test]
async fn core_register_roundtrip_with_mock() {
    let mcp = CmsisDapMcp::new(
        SessionManager::new(Box::new(MockBackend::new())),
        SecurityPolicy {
            allow_destructive: false,
        },
    );
    connect(&mcp);
    let write = WriteCoreRegisterParams {
        name: Some("r0".into()),
        number: None,
        value: 0x1234,
    };
    assert!(!mcp
        .write_core_register(Parameters(write))
        .await
        .is_error
        .unwrap_or(true));
    let read = ReadCoreRegisterParams {
        name: Some("r0".into()),
        number: None,
    };
    let res = mcp.read_core_register(Parameters(read)).await;
    assert_eq!(
        res.structured_content.unwrap()["value"].as_u64(),
        Some(0x1234)
    );
}

#[test]
fn instructions_are_self_contained() {
    assert!(cmsis_dap_mcp::mcp::SERVER_INSTRUCTIONS.len() >= 512);
}
#[tokio::test]
async fn dap_read_write_with_mock() {
    let mcp = CmsisDapMcp::new(
        SessionManager::new(Box::new(MockBackend::new())),
        SecurityPolicy {
            allow_destructive: false,
        },
    );
    connect(&mcp);
    let res = mcp
        .write_dap(Parameters(WriteDapParams {
            address: 0x4,
            value: 0x1,
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let res = mcp
        .read_dap(Parameters(ReadDapParams { address: 0x4 }))
        .await;
    assert_eq!(res.structured_content.unwrap()["value"].as_u64(), Some(0x1));
}
const MINI_SVD: &str = r#"<?xml version="1.0"?>
<device schemaVersion="1.1">
<vendor>Test</vendor><name>TestDevice</name><version>1.0</version><description>test device</description>
<addressUnitBits>8</addressUnitBits><width>32</width><size>32</size><access>read-write</access>
<resetValue>0x00000000</resetValue><resetMask>0xFFFFFFFF</resetMask>
<peripherals>
<peripheral><name>GPIOA</name><description>GPIO A</description><baseAddress>0x48000000</baseAddress>
<addressBlock><offset>0x0</offset><size>0x400</size><usage>registers</usage></addressBlock>
<registers><register><name>ODR</name><description>output data</description><addressOffset>0x14</addressOffset>
<size>32</size><access>read-write</access><resetValue>0x0</resetValue>
<fields><field><name>ODR0</name><bitOffset>0</bitOffset><bitWidth>1</bitWidth></field></fields>
</register></registers></peripheral>
</peripherals></device>"#;

#[tokio::test]
async fn peripheral_read_write_with_mock() {
    use cmsis_dap_mcp::mcp::{LoadSvdParams, ReadPeripheralParams, WritePeripheralParams};
    use std::io::Write;

    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(MINI_SVD.as_bytes()).unwrap();
    let mcp = CmsisDapMcp::new(
        SessionManager::new(Box::new(MockBackend::new())),
        SecurityPolicy {
            allow_destructive: false,
        },
    );
    connect(&mcp);
    let path = f.path().to_string_lossy().to_string();
    let res = mcp.load_svd(Parameters(LoadSvdParams { path })).await;
    assert!(!res.is_error.unwrap_or(true));
    let res = mcp
        .write_peripheral(Parameters(WritePeripheralParams {
            peripheral: "GPIOA".into(),
            register: "ODR".into(),
            field: Some("ODR0".into()),
            value: 1,
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let res = mcp
        .read_peripheral(Parameters(ReadPeripheralParams {
            peripheral: "GPIOA".into(),
            register: "ODR".into(),
            field: Some("ODR0".into()),
        }))
        .await;
    assert_eq!(res.structured_content.unwrap()["value"].as_u64(), Some(1));
}
#[tokio::test]
async fn flash_blocked_without_flag() {
    let mcp = CmsisDapMcp::new(
        SessionManager::new(Box::new(MockBackend::new())),
        SecurityPolicy {
            allow_destructive: false,
        },
    );
    connect(&mcp);
    let res = mcp
        .erase_flash(Parameters(EraseFlashParams {
            address: 0x0800_0000,
            size: 0x1000,
        }))
        .await;
    let structured = res.structured_content.unwrap_or_default();
    assert_eq!(structured["code"], "DestructiveDisabled");
}

#[tokio::test]
async fn flash_works_with_flag() {
    let mcp = CmsisDapMcp::new(
        SessionManager::new(Box::new(MockBackend::new())),
        SecurityPolicy {
            allow_destructive: true,
        },
    );
    connect(&mcp);
    let res = mcp
        .program_flash(Parameters(ProgramFlashParams {
            address: 0x0800_0000,
            data: Some(vec![0xAA, 0xBB]),
            verify: None,
            path: None,
            format: None,
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let res = mcp
        .read_memory(Parameters(ReadMemoryParams {
            address: 0x0800_0000,
            width: "u8".into(),
            count: 2,
            path: None,
            format: None,
        }))
        .await;
    let structured = res.structured_content.unwrap();
    assert_eq!(structured["values"][0].as_u64(), Some(0xAA));
    assert_eq!(structured["values"][1].as_u64(), Some(0xBB));
}
#[tokio::test]
async fn connect_disconnect_flow() {
    let mcp = CmsisDapMcp::new(
        SessionManager::new(Box::new(MockBackend::new())),
        SecurityPolicy {
            allow_destructive: false,
        },
    );
    let res = mcp.list_probes(Parameters(ListProbesParams {})).await;
    assert!(!res.is_error.unwrap_or(true));
    assert_eq!(
        res.structured_content.unwrap()["probes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let res = mcp
        .connect(Parameters(ConnectParams {
            probe_id: None,
            protocol: Some("swd".into()),
            speed_khz: None,
            target: None,
            under_reset: None,
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let res = mcp
        .get_target_info(Parameters(GetTargetInfoParams {}))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let res = mcp.disconnect(Parameters(DisconnectParams {})).await;
    assert!(!res.is_error.unwrap_or(true));
}

#[tokio::test]
async fn get_probe_info_filters_by_id() {
    let mcp = CmsisDapMcp::new(
        SessionManager::new(Box::new(MockBackend::new())),
        SecurityPolicy {
            allow_destructive: false,
        },
    );
    let res = mcp
        .get_probe_info(Parameters(GetProbeInfoParams {
            probe_id: Some("mock".into()),
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let res = mcp
        .get_probe_info(Parameters(GetProbeInfoParams {
            probe_id: Some("missing".into()),
        }))
        .await;
    assert!(res.is_error.unwrap_or(false));
}
