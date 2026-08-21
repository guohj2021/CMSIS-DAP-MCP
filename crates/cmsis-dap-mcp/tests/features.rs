use cmsis_dap_core::backend::mock::MockBackend;
use cmsis_dap_core::backend::{ConnectOptions, Protocol};
use cmsis_dap_core::session::SessionManager;
use cmsis_dap_mcp::mcp::{
    ClearWatchpointsParams, CmsisDapMcp, ConnectParams, DisconnectParams, EraseFlashParams,
    GetCoreStatusParams, HaltParams, ListCoreRegistersParams, ListWatchpointsParams,
    ProgramFlashParams, ReadMemoryParams, ResetParams, ResumeParams, RunScriptParams,
    SetWatchpointParams, VerifyMemoryParams, WriteMemoryParams,
};
use rmcp::handler::server::wrapper::Parameters;

fn mcp(allow_destructive: bool) -> CmsisDapMcp {
    CmsisDapMcp::new(
        SessionManager::new(Box::new(MockBackend::new())),
        allow_destructive,
    )
}

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

fn structured_value(res: &rmcp::model::CallToolResult, key: &str) -> serde_json::Value {
    res.structured_content
        .as_ref()
        .and_then(|c| c.get(key))
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

#[tokio::test]
async fn list_core_registers_returns_names() {
    let mcp = mcp(false);
    connect(&mcp);
    let res = mcp
        .list_core_registers(Parameters(ListCoreRegistersParams {}))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let regs = structured_value(&res, "registers");
    let names: Vec<String> = regs
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    assert!(names.iter().any(|n| n == "pc"));
    assert!(names.iter().any(|n| n == "r0"));
}

#[tokio::test]
async fn get_core_status_reflects_halt_and_resume() {
    let mcp = mcp(false);
    connect(&mcp);
    let res = mcp
        .get_core_status(Parameters(GetCoreStatusParams {}))
        .await;
    assert_eq!(
        structured_value(&res, "state"),
        serde_json::json!("running")
    );
    mcp.halt(Parameters(HaltParams {})).await;
    let res = mcp
        .get_core_status(Parameters(GetCoreStatusParams {}))
        .await;
    assert_eq!(structured_value(&res, "state"), serde_json::json!("halted"));
    mcp.resume(Parameters(ResumeParams {})).await;
    let res = mcp
        .get_core_status(Parameters(GetCoreStatusParams {}))
        .await;
    assert_eq!(
        structured_value(&res, "state"),
        serde_json::json!("running")
    );
}

#[tokio::test]
async fn watchpoint_roundtrip() {
    let mcp = mcp(false);
    connect(&mcp);
    let res = mcp
        .set_watchpoint(Parameters(SetWatchpointParams {
            address: 0x2000_1000,
            access: "write".into(),
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let res = mcp
        .list_watchpoints(Parameters(ListWatchpointsParams {}))
        .await;
    let wps = structured_value(&res, "watchpoints");
    assert_eq!(wps[0]["address"], serde_json::json!(0x2000_1000));
    assert_eq!(wps[0]["access"], serde_json::json!("write"));
    let res = mcp
        .clear_watchpoints(Parameters(ClearWatchpointsParams {}))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let res = mcp
        .list_watchpoints(Parameters(ListWatchpointsParams {}))
        .await;
    assert_eq!(
        structured_value(&res, "watchpoints")
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[tokio::test]
async fn watchpoint_rejects_invalid_access() {
    let mcp = mcp(false);
    connect(&mcp);
    let res = mcp
        .set_watchpoint(Parameters(SetWatchpointParams {
            address: 0x2000_1000,
            access: "execute".into(),
        }))
        .await;
    assert!(res.is_error.unwrap_or(false));
    assert_eq!(
        structured_value(&res, "code"),
        serde_json::json!("InvalidArgument")
    );
}

#[tokio::test]
async fn verify_memory_reports_mismatch() {
    let mcp = mcp(false);
    connect(&mcp);
    mcp.write_memory(Parameters(WriteMemoryParams {
        address: 0x2000_0000,
        width: "u32".into(),
        values: vec![0x1111_1111, 0x2222_2222],
    }))
    .await;
    let res = mcp
        .verify_memory(Parameters(VerifyMemoryParams {
            address: 0x2000_0000,
            width: "u32".into(),
            data: vec![0x1111_1111, 0x9999_9999],
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    assert_eq!(structured_value(&res, "verified"), serde_json::json!(false));
    assert_eq!(
        structured_value(&res, "mismatches")[0]["index"],
        serde_json::json!(1)
    );
}

#[tokio::test]
async fn verify_memory_accepts_matching_data() {
    let mcp = mcp(false);
    connect(&mcp);
    let res = mcp
        .verify_memory(Parameters(VerifyMemoryParams {
            address: 0x2000_0000,
            width: "u32".into(),
            data: vec![0, 0],
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    assert_eq!(structured_value(&res, "verified"), serde_json::json!(true));
}

#[tokio::test]
async fn reset_supports_run_and_halt_modes() {
    let mcp = mcp(false);
    connect(&mcp);
    mcp.halt(Parameters(HaltParams {})).await;
    let res = mcp
        .reset(Parameters(ResetParams {
            mode: Some("run".into()),
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let res = mcp
        .get_core_status(Parameters(GetCoreStatusParams {}))
        .await;
    assert_eq!(
        structured_value(&res, "state"),
        serde_json::json!("running")
    );
    let res = mcp
        .reset(Parameters(ResetParams {
            mode: Some("halt".into()),
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let res = mcp
        .get_core_status(Parameters(GetCoreStatusParams {}))
        .await;
    assert_eq!(structured_value(&res, "state"), serde_json::json!("halted"));
}

#[tokio::test]
async fn reset_rejects_invalid_mode() {
    let mcp = mcp(false);
    connect(&mcp);
    let res = mcp
        .reset(Parameters(ResetParams {
            mode: Some("restart".into()),
        }))
        .await;
    assert!(res.is_error.unwrap_or(false));
    assert_eq!(
        structured_value(&res, "code"),
        serde_json::json!("InvalidArgument")
    );
}

#[tokio::test]
async fn connect_accepts_under_reset() {
    let mcp = mcp(false);
    let res = mcp
        .connect(Parameters(ConnectParams {
            probe_id: None,
            protocol: Some("swd".into()),
            speed_khz: None,
            target: None,
            under_reset: Some(true),
            core: None,
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
}

#[tokio::test]
async fn program_flash_accepts_verify_flag() {
    let mcp = mcp(true);
    connect(&mcp);
    let res = mcp
        .program_flash(Parameters(ProgramFlashParams {
            address: 0x0800_0000,
            data: Some(vec![0xAA, 0xBB]),
            verify: Some(true),
            path: None,
            format: None,
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
}

#[tokio::test]
async fn erase_flash_rejects_zero_size() {
    let mcp = mcp(true);
    connect(&mcp);
    let res = mcp
        .erase_flash(Parameters(EraseFlashParams {
            address: 0x0800_0000,
            size: 0,
        }))
        .await;
    assert!(res.is_error.unwrap_or(false));
    assert_eq!(
        structured_value(&res, "code"),
        serde_json::json!("InvalidArgument")
    );
}

#[tokio::test]
async fn probe_info_has_capability_fields() {
    let mcp = mcp(false);
    let res = mcp
        .list_probes(Parameters(cmsis_dap_mcp::mcp::ListProbesParams {}))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let probes = structured_value(&res, "probes");
    assert!(probes[0].get("protocols").is_some());
    assert!(probes[0].get("speed_khz").is_some());
    assert!(probes[0].get("target_voltage").is_some());
    assert!(probes[0].get("is_hid").is_some());
}

#[tokio::test]
async fn target_info_has_ap_count_cpu_and_memory() {
    let mcp = mcp(false);
    connect(&mcp);
    let res = mcp
        .get_target_info(Parameters(cmsis_dap_mcp::mcp::GetTargetInfoParams {}))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let target = structured_value(&res, "target");
    assert_eq!(target["core_count"], serde_json::json!(1));
    assert!(target["ap_count"].as_u64().unwrap() >= 1);
    assert!(target.get("cpu_id").is_some());
    assert!(target.get("dp_id").is_some());
    assert!(target.get("memory_regions").is_some());
}

#[tokio::test]
async fn disconnect_still_works_after_new_tools() {
    let mcp = mcp(false);
    connect(&mcp);
    let res = mcp.disconnect(Parameters(DisconnectParams {})).await;
    assert!(!res.is_error.unwrap_or(true));
}

#[tokio::test]
async fn read_memory_still_works_after_width_parsing_changes() {
    let mcp = mcp(false);
    connect(&mcp);
    let res = mcp
        .read_memory(Parameters(ReadMemoryParams {
            address: 0x2000_0000,
            width: "u8".into(),
            count: 1,
            path: None,
            format: None,
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
}

#[tokio::test]
async fn run_script_inline_with_mock() {
    let mcp = mcp(false);
    connect(&mcp);
    let res = mcp
        .run_script(Parameters(RunScriptParams {
            path: None,
            script: Some("halt\ngo\n".into()),
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true));
    let structured = res.structured_content.unwrap();
    assert_eq!(structured["ok"], serde_json::json!(true));
    assert_eq!(structured["commands"], serde_json::json!(2));
}

#[tokio::test]
async fn run_script_requires_path_or_script() {
    let mcp = mcp(false);
    let res = mcp
        .run_script(Parameters(RunScriptParams {
            path: None,
            script: None,
        }))
        .await;
    assert!(res.is_error.unwrap_or(false));
    assert_eq!(
        res.structured_content.unwrap()["code"],
        serde_json::json!("InvalidArgument")
    );
}

#[tokio::test]
async fn program_flash_accepts_bin_file() {
    use std::io::Write;
    let mcp = mcp(true);
    connect(&mcp);
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(&[0xAA, 0xBB, 0xCC, 0xDD]).unwrap();
    let path = f.path().to_string_lossy().to_string();
    let res = mcp
        .program_flash(Parameters(ProgramFlashParams {
            address: 0x0800_0000,
            data: None,
            verify: Some(true),
            path: Some(path),
            format: Some("bin".into()),
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true), "{res:?}");
    let res = mcp
        .read_memory(Parameters(ReadMemoryParams {
            address: 0x0800_0000,
            width: "u8".into(),
            count: 4,
            path: None,
            format: None,
        }))
        .await;
    let structured = res.structured_content.unwrap();
    assert_eq!(structured["values"][0].as_u64(), Some(0xAA));
    assert_eq!(structured["values"][3].as_u64(), Some(0xDD));
}

#[tokio::test]
async fn program_flash_path_missing_file_returns_file_error() {
    let mcp = mcp(true);
    connect(&mcp);
    let res = mcp
        .program_flash(Parameters(ProgramFlashParams {
            address: 0x0800_0000,
            data: None,
            verify: None,
            path: Some("Z:/definitely/missing/file.bin".into()),
            format: Some("bin".into()),
        }))
        .await;
    assert!(res.is_error.unwrap_or(false));
    assert_eq!(
        res.structured_content.unwrap()["code"],
        serde_json::json!("FileError")
    );
}

#[tokio::test]
async fn read_memory_exports_bin_file() {
    let mcp = mcp(false);
    connect(&mcp);
    mcp.write_memory(Parameters(WriteMemoryParams {
        address: 0x2000_0000,
        width: "u32".into(),
        values: vec![0x1122_3344],
    }))
    .await;
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("dump.bin");
    let res = mcp
        .read_memory(Parameters(ReadMemoryParams {
            address: 0x2000_0000,
            width: "u8".into(),
            count: 4,
            path: Some(out.to_string_lossy().to_string()),
            format: Some("bin".into()),
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true), "{res:?}");
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(bytes, vec![0x44, 0x33, 0x22, 0x11]);
}

#[tokio::test]
async fn read_memory_exports_hex_file() {
    let mcp = mcp(false);
    connect(&mcp);
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("dump.hex");
    let res = mcp
        .read_memory(Parameters(ReadMemoryParams {
            address: 0x0800_0000,
            width: "u8".into(),
            count: 1,
            path: Some(out.to_string_lossy().to_string()),
            format: Some("hex".into()),
        }))
        .await;
    assert!(!res.is_error.unwrap_or(true), "{res:?}");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.starts_with(':'));
    assert!(text.ends_with(":00000001FF\n"));
}
