use cmsis_dap_core::backend::mock::MockBackend;
use cmsis_dap_core::backend::{AccessWidth, ConnectOptions, Protocol};
use cmsis_dap_core::session::SessionManager;
use cmsis_dap_mcp::mcp::tools_option;
use cmsis_dap_mcp::mcp::{
    ClearBreakpointsParams, ClearFlashBreakpointsParams, CmsisDapMcp, ConnectParams,
    DisconnectParams, DumpCpuStateParams, EraseFlashParams, GetConfigParams, GetProbeInfoParams,
    GetTargetInfoParams, HaltParams, ListBreakpointsParams, ListFlashBreakpointsParams,
    ListProbesParams, ProgramFlashParams, ReadCoreRegisterParams, ReadDapParams, ReadMemoryParams,
    ReadOptionBytesParams, ReadSwoParams, ReloadConfigParams, ResetParams, ResumeParams,
    SetBreakpointParams, SetFlashBreakpointParams, StartSwoParams, StepParams, StopSwoParams,
    UpdateConfigParams, VerifyMemoryParams, WriteCoreRegisterParams, WriteDapParams,
    WriteMemoryParams, WriteOptionBytesParams,
};
use rmcp::handler::server::wrapper::Parameters;

fn connect(mcp: &CmsisDapMcp) {
    let opts = ConnectOptions {
        probe_id: None,
        protocol: Protocol::Swd,
        speed_khz: None,
        target: None,
        under_reset: false,
        core_index: None,
    };
    mcp.runtime.session.lock().unwrap().connect(&opts).unwrap();
}

#[tokio::test]
async fn dump_cpu_state_returns_structured_dump() {
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
    connect(&mcp);
    mcp.runtime
        .session
        .lock()
        .unwrap()
        .backend()
        .write_memory(0x2000_0000, AccessWidth::U32, &[0x1234_5678])
        .unwrap();
    let params = DumpCpuStateParams {
        addresses: Some(vec![0x2000_0000]),
        stack_words: Some(4),
        restore: Some(true),
    };
    let res = mcp.dump_cpu_state(Parameters(params)).await;
    assert!(!res.is_error.unwrap_or(true));
    let structured = res.structured_content.unwrap();
    assert_eq!(structured["state"], "running");
    assert!(!structured["registers"].as_array().unwrap().is_empty());
    assert_eq!(structured["memory"][0]["value"].as_u64(), Some(0x1234_5678));
    assert_eq!(
        mcp.runtime
            .session
            .lock()
            .unwrap()
            .backend()
            .get_core_status()
            .unwrap()
            .state,
        "running"
    );
}

#[tokio::test]
async fn read_memory_returns_mock_values() {
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
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
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
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
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
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
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
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
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
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
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
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
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
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
async fn erase_flash_requires_flash_definition() {
    let mcp = CmsisDapMcp::new(
        SessionManager::new(Box::new(MockBackend::without_flash())),
        true,
    );
    connect(&mcp);
    let res = mcp
        .erase_flash(Parameters(EraseFlashParams {
            address: 0,
            size: u64::MAX,
        }))
        .await;
    assert!(res.is_error.unwrap_or(false));
    let structured = res.structured_content.unwrap();
    assert_eq!(structured["code"], "UnsupportedFeature");
}

#[tokio::test]
async fn flash_works_with_flag() {
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), true);
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
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
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
            core: None,
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
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
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

/// A server started with no startup flags sits in a "to-be-configured" state:
/// destructive tools are gated, but can be enabled at runtime via
/// `update_config` and take effect immediately without a restart.
#[tokio::test]
async fn runtime_config_enables_destructive_tools() {
    // No flags: allow_destructive = false, no config file wired in.
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
    connect(&mcp);

    // get_config reflects the to-be-configured state.
    let cfg = mcp.get_config(Parameters(GetConfigParams {})).await;
    assert!(!cfg.is_error.unwrap_or(true));
    assert_eq!(
        cfg.structured_content.clone().unwrap()["allow_destructive"],
        serde_json::json!(false)
    );

    // Destructive tool is blocked while the gate is closed.
    let rejected = mcp
        .erase_flash(Parameters(EraseFlashParams {
            address: 0x0800_0000,
            size: 1024,
        }))
        .await;
    assert!(rejected.is_error.unwrap_or(false));
    assert_eq!(
        rejected.structured_content.clone().unwrap()["code"],
        serde_json::json!("DestructiveDisabled")
    );

    // Enable destructive tools at runtime; no restart needed.
    let updated = mcp
        .update_config(Parameters(UpdateConfigParams {
            allow_destructive: Some(true),
            tcp_port: None,
            gdb_port: None,
        }))
        .await;
    assert!(!updated.is_error.unwrap_or(true));
    assert!(updated.structured_content.clone().unwrap()["updated"]
        .as_bool()
        .unwrap());

    // get_config now reflects the live change.
    let cfg = mcp.get_config(Parameters(GetConfigParams {})).await;
    assert_eq!(
        cfg.structured_content.clone().unwrap()["allow_destructive"],
        serde_json::json!(true)
    );

    // The same destructive tool now runs successfully.
    let ok = mcp
        .erase_flash(Parameters(EraseFlashParams {
            address: 0x0800_0000,
            size: 1024,
        }))
        .await;
    assert!(!ok.is_error.unwrap_or(true));

    // reload_config fails clearly when no --config-file was supplied.
    let missing = mcp.reload_config(Parameters(ReloadConfigParams {})).await;
    assert!(missing.is_error.unwrap_or(false));
    assert_eq!(
        missing.structured_content.clone().unwrap()["code"],
        serde_json::json!("ConfigError")
    );
}

/// `update_config` must reject invalid values atomically: a bad port returns a
/// clear ConfigError and leaves the previously-good config untouched.
#[tokio::test]
async fn update_config_rejects_invalid_port() {
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
    connect(&mcp);

    let bad = mcp
        .update_config(Parameters(UpdateConfigParams {
            allow_destructive: None,
            tcp_port: Some(0),
            gdb_port: None,
        }))
        .await;
    assert!(bad.is_error.unwrap_or(false));
    assert_eq!(
        bad.structured_content.clone().unwrap()["code"],
        serde_json::json!("ConfigError")
    );

    // Config is unchanged: still in the to-be-configured (non-destructive) state.
    let cfg = mcp.get_config(Parameters(GetConfigParams {})).await;
    let c = cfg.structured_content.clone().unwrap();
    assert_eq!(c["allow_destructive"], serde_json::json!(false));
    assert_eq!(c["tcp_port"], serde_json::Value::Null);
}

/// End-to-end flash lifecycle through the MCP layer: erase -> read (sees
/// 0xFF) -> program -> read (sees written bytes) -> verify. Also confirms the
/// destructive gate (runtime-config) blocks erase before it is enabled.
#[tokio::test]
async fn flash_erase_program_read_cycle() {
    let mcp = CmsisDapMcp::new(
        SessionManager::new(Box::new(MockBackend::new())),
        false, // to-be-configured: destructive tools gated
    );
    connect(&mcp);
    let addr = 0x0800_0000u64;

    // 1) Erase is blocked while the destructive gate is closed.
    let blocked = mcp
        .erase_flash(Parameters(EraseFlashParams {
            address: addr,
            size: 256,
        }))
        .await;
    assert!(blocked.is_error.unwrap_or(false));
    assert_eq!(
        blocked.structured_content.clone().unwrap()["code"],
        serde_json::json!("DestructiveDisabled")
    );

    // 2) Enable destructive tools at runtime (no restart).
    mcp.update_config(Parameters(UpdateConfigParams {
        allow_destructive: Some(true),
        tcp_port: None,
        gdb_port: None,
    }))
    .await;

    // 3) Erase the flash region.
    let erased = mcp
        .erase_flash(Parameters(EraseFlashParams {
            address: addr,
            size: 256,
        }))
        .await;
    assert!(!erased.is_error.unwrap_or(true));

    // 4) Read back: erased flash reads as 0xFF.
    let read_after_erase = mcp
        .read_memory(Parameters(ReadMemoryParams {
            address: addr,
            width: "u8".into(),
            count: 4,
            path: None,
            format: None,
        }))
        .await;
    assert!(!read_after_erase.is_error.unwrap_or(true));
    let erased_bytes = read_after_erase.structured_content.clone().unwrap()["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect::<Vec<_>>();
    assert_eq!(erased_bytes, vec![0xFF, 0xFF, 0xFF, 0xFF]);

    // 5) Program four bytes.
    let written = [0xDEu8, 0xAD, 0xBE, 0xEF];
    let programmed = mcp
        .program_flash(Parameters(ProgramFlashParams {
            address: addr,
            data: Some(written.to_vec()),
            verify: Some(true),
            path: None,
            format: None,
        }))
        .await;
    assert!(!programmed.is_error.unwrap_or(true));

    // 6) Read back: programmed bytes are now visible through MCP read_memory.
    let read_after_program = mcp
        .read_memory(Parameters(ReadMemoryParams {
            address: addr,
            width: "u8".into(),
            count: 4,
            path: None,
            format: None,
        }))
        .await;
    assert!(!read_after_program.is_error.unwrap_or(true));
    let programmed_bytes = read_after_program.structured_content.clone().unwrap()["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect::<Vec<_>>();
    assert_eq!(programmed_bytes, written);

    // 7) Verify the flash contents against the expected data.
    let verified = mcp
        .verify_memory(Parameters(VerifyMemoryParams {
            address: addr,
            width: "u8".into(),
            data: written.iter().map(|b| *b as u64).collect(),
        }))
        .await;
    assert!(!verified.is_error.unwrap_or(true));
    assert_eq!(
        verified.structured_content.clone().unwrap()["verified"],
        serde_json::json!(true)
    );
}

#[tokio::test]
async fn swo_start_stop_read_flow() {
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
    connect(&mcp);
    let res = mcp
        .start_swo(Parameters(StartSwoParams {
            baud: 2_000_000,
            tpiu_clk: 8_000_000,
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    assert_eq!(
        res.structured_content.unwrap()["started"],
        serde_json::json!(true)
    );
    // max_bytes is currently accepted but not enforced by the backend; pass
    // Some(4) to document that the handler still returns all available bytes.
    let res = mcp
        .read_swo(Parameters(ReadSwoParams { max_bytes: Some(4) }))
        .await;
    let structured = res.structured_content.unwrap();
    assert_eq!(structured["bytes"].as_u64(), Some(3));
    assert_eq!(structured["data_hex"].as_str(), Some("010203"));
    let res = mcp.stop_swo(Parameters(StopSwoParams {})).await;
    assert!(!res.is_error.unwrap_or(true));
}

#[tokio::test]
async fn read_swo_requires_active_trace() {
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
    connect(&mcp);
    let res = mcp
        .read_swo(Parameters(ReadSwoParams { max_bytes: None }))
        .await;
    assert!(res.is_error.unwrap_or(false));
}

#[tokio::test]
async fn read_swo_without_connection_errors() {
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
    let res = mcp
        .read_swo(Parameters(ReadSwoParams { max_bytes: None }))
        .await;
    assert!(res.is_error.unwrap_or(false));
}

#[tokio::test]
async fn read_option_bytes_returns_mock_rdp() {
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
    connect(&mcp);
    let res = mcp
        .read_option_bytes(Parameters(ReadOptionBytesParams {}))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let structured = res.structured_content.unwrap();
    assert_eq!(
        structured["option_bytes"][0]["name"],
        serde_json::json!("RDP")
    );
    assert_eq!(structured["option_bytes"][0]["value"].as_u64(), Some(0xAA));
}

#[tokio::test]
async fn write_option_bytes_blocked_without_flag() {
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
    connect(&mcp);
    let res = mcp
        .write_option_bytes(Parameters(WriteOptionBytesParams {
            bytes: vec![tools_option::OptionByteParam {
                name: "DATA0".into(),
                address: 0x4002_3C14,
                value: 0x55,
            }],
        }))
        .await;
    let structured = res.structured_content.unwrap_or_default();
    assert_eq!(structured["code"], "DestructiveDisabled");
}

#[tokio::test]
async fn write_option_bytes_works_with_flag() {
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), true);
    connect(&mcp);
    let res = mcp
        .write_option_bytes(Parameters(WriteOptionBytesParams {
            bytes: vec![tools_option::OptionByteParam {
                name: "DATA0".into(),
                address: 0x4002_3C14,
                value: 0x55,
            }],
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    assert_eq!(
        res.structured_content.unwrap()["written"],
        serde_json::json!(true)
    );
}

#[tokio::test]
async fn set_flash_breakpoint_blocked_without_flag() {
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
    connect(&mcp);
    let res = mcp
        .set_flash_breakpoint(Parameters(SetFlashBreakpointParams {
            address: 0x0800_0000,
        }))
        .await;
    let structured = res.structured_content.unwrap_or_default();
    assert_eq!(structured["code"], "DestructiveDisabled");
}

#[tokio::test]
async fn flash_breakpoint_flow_with_flag() {
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), true);
    connect(&mcp);
    let res = mcp
        .set_flash_breakpoint(Parameters(SetFlashBreakpointParams {
            address: 0x0800_0000,
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let res = mcp
        .list_flash_breakpoints(Parameters(ListFlashBreakpointsParams {}))
        .await;
    let structured = res.structured_content.unwrap();
    assert_eq!(
        structured["flash_breakpoints"][0].as_u64(),
        Some(0x0800_0000)
    );
    let res = mcp
        .clear_flash_breakpoints(Parameters(ClearFlashBreakpointsParams {}))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let res = mcp
        .list_flash_breakpoints(Parameters(ListFlashBreakpointsParams {}))
        .await;
    assert_eq!(
        res.structured_content.unwrap()["flash_breakpoints"]
            .as_array()
            .map(|a| a.len()),
        Some(0)
    );
}

#[tokio::test]
async fn connect_accepts_core_index() {
    let mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), false);
    let res = mcp
        .connect(Parameters(ConnectParams {
            probe_id: None,
            protocol: Some("swd".into()),
            speed_khz: None,
            target: None,
            under_reset: None,
            core: Some(0),
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let target = res.structured_content.unwrap()["target"].clone();
    assert_eq!(target["core_count"].as_u64(), Some(1));
    assert_eq!(target["cores"][0]["index"].as_u64(), Some(0));
}
