use clap::Parser;
use cmsis_dap_cli::cmd::{actions, SvdAction, SvdArgs, SvdReadArgs, SvdWriteArgs};
use cmsis_dap_cli::cmd::{repl, run, CliArgs, CliError, ReplOptions};
use cmsis_dap_core::backend::mock::MockBackend;
use cmsis_dap_core::backend::{ConnectOptions, Protocol};
use cmsis_dap_core::session::SessionManager;
use std::io::Cursor;
use std::io::Write;

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

fn execute(args: &[&str]) -> Result<serde_json::Value, CliError> {
    let args =
        CliArgs::try_parse_from(args.iter().map(|s| s.to_string())).expect("args should parse");
    let out = run(args, Box::new(MockBackend::new()))?;
    Ok(out.expect("expected structured output"))
}

#[test]
fn list_returns_probes() {
    let out = execute(&["cmsis-dap-cli", "list"]).unwrap();
    let probes = out["probes"].as_array().expect("probes array");
    assert!(!probes.is_empty());
    assert!(probes[0]["id"].as_str().is_some());
}

#[test]
fn connect_returns_target_info() {
    let out = execute(&["cmsis-dap-cli", "connect"]).unwrap();
    assert!(out["target"]["core_type"].as_str().is_some());
}

#[test]
fn read_memory_returns_values() {
    let out = execute(&[
        "cmsis-dap-cli",
        "read",
        "--address",
        "0x20000000",
        "--width",
        "u32",
        "--count",
        "2",
    ])
    .unwrap();
    assert_eq!(out["values"].as_array().map(|a| a.len()), Some(2));
}

#[test]
fn write_then_read_memory_via_script() {
    let out = execute(&[
        "cmsis-dap-cli",
        "script",
        "--text",
        "connect\nw32 0x20000000 0xDEADBEEF\nmem32 0x20000000 1",
    ])
    .unwrap();
    assert_eq!(out["ok"].as_bool(), Some(true));
    let results = out["results"].as_array().unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(
        results[2]["output"]["values"][0].as_u64(),
        Some(0xDEAD_BEEF)
    );
}

#[test]
fn verify_memory_reports_mismatch() {
    let out = execute(&[
        "cmsis-dap-cli",
        "verify",
        "--address",
        "0x20000000",
        "--width",
        "u32",
        "--values",
        "0x1111",
    ])
    .unwrap();
    assert_eq!(out["verified"].as_bool(), Some(false));
    assert!(out["mismatches"].as_array().map(|a| a.len()).unwrap_or(0) > 0);
}

#[test]
fn reg_set_then_get_via_script() {
    let out = execute(&[
        "cmsis-dap-cli",
        "script",
        "--text",
        "connect\nreg pc 0x1000\nreg pc",
    ])
    .unwrap();
    let results = out["results"].as_array().unwrap();
    assert_eq!(results[2]["output"]["value"].as_u64(), Some(0x1000));
}

#[test]
fn halt_resume_step_and_status() {
    assert_eq!(
        execute(&["cmsis-dap-cli", "halt"]).unwrap()["halted"],
        serde_json::json!(true)
    );
    assert_eq!(
        execute(&["cmsis-dap-cli", "resume"]).unwrap()["running"],
        serde_json::json!(true)
    );
    assert_eq!(
        execute(&["cmsis-dap-cli", "step"]).unwrap()["stepped"],
        serde_json::json!(true)
    );
    let status = execute(&["cmsis-dap-cli", "status"]).unwrap();
    assert!(status["state"].as_str().is_some());
}

#[test]
fn reset_supports_halt_mode() {
    let out = execute(&["cmsis-dap-cli", "reset", "--mode", "halt"]).unwrap();
    assert_eq!(out["mode"], serde_json::json!("halt"));
}

#[test]
fn breakpoint_set_and_list() {
    let set = execute(&["cmsis-dap-cli", "bp", "set", "0x8000100"]).unwrap();
    assert_eq!(set["set"], serde_json::json!(true));
    let list = execute(&["cmsis-dap-cli", "bp", "list"]).unwrap();
    assert!(list["breakpoints"].is_array());
}

#[test]
fn watchpoint_set_and_list() {
    let set = execute(&["cmsis-dap-cli", "wp", "set", "0x20000000", "--access", "rw"]).unwrap();
    assert_eq!(set["set"], serde_json::json!(true));
    let list = execute(&["cmsis-dap-cli", "wp", "list"]).unwrap();
    assert!(list["watchpoints"].is_array());
}

#[test]
fn dap_write_and_read() {
    let write = execute(&["cmsis-dap-cli", "dap", "write", "0x4", "0xDEADBEEF"]).unwrap();
    assert_eq!(write["written"], serde_json::json!(true));
    let read = execute(&["cmsis-dap-cli", "dap", "read", "0x4"]).unwrap();
    assert!(read["value"].is_number());
}

#[test]
fn svd_list_and_field_read() {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(MINI_SVD.as_bytes()).unwrap();
    let svd = f.path().to_str().unwrap();
    let list = execute(&["cmsis-dap-cli", "--svd", svd, "svd", "list"]).unwrap();
    assert_eq!(list["peripherals"], serde_json::json!(["GPIOA"]));
    let read = execute(&[
        "cmsis-dap-cli",
        "--svd",
        svd,
        "svd",
        "read",
        "GPIOA.ODR.ODR0",
    ])
    .unwrap();
    assert_eq!(read["address"].as_u64(), Some(0x4800_0014));
    assert_eq!(read["value"].as_u64(), Some(0));
}

#[test]
fn svd_field_write() {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(MINI_SVD.as_bytes()).unwrap();
    let mut session = SessionManager::new(Box::new(MockBackend::new()));
    session
        .connect(&ConnectOptions {
            probe_id: None,
            protocol: Protocol::Swd,
            speed_khz: None,
            target: None,
            under_reset: false,
        })
        .unwrap();
    session.load_svd(f.path()).unwrap();
    let write = actions::svd(
        &mut session,
        &SvdArgs {
            action: SvdAction::Write(SvdWriteArgs {
                target: "GPIOA.ODR.ODR0".into(),
                value: 1,
            }),
        },
    )
    .unwrap();
    assert_eq!(write["written"], serde_json::json!(true));
    let read = actions::svd(
        &mut session,
        &SvdArgs {
            action: SvdAction::Read(SvdReadArgs {
                target: "GPIOA.ODR.ODR0".into(),
            }),
        },
    )
    .unwrap();
    assert_eq!(read["value"].as_u64(), Some(1));
}

#[test]
fn flash_program_programs_file() {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(&[0x01u8, 0x02, 0x03]).unwrap();
    let file = f.path().to_str().unwrap();
    let out = execute(&[
        "cmsis-dap-cli",
        "flash",
        "program",
        "--address",
        "0x8000000",
        "--file",
        file,
        "--format",
        "bin",
        "--verify",
    ])
    .unwrap();
    assert_eq!(out["programmed"], serde_json::json!(true));
    assert_eq!(out["bytes"].as_u64(), Some(3));
}

#[test]
fn flash_erase_requires_flash_definition() {
    let args = CliArgs::try_parse_from(
        [
            "cmsis-dap-cli",
            "flash",
            "erase",
            "--address",
            "0",
            "--size",
            "0x1000",
        ]
        .iter()
        .map(|s| s.to_string()),
    )
    .unwrap();
    let err = run(args, Box::new(MockBackend::without_flash())).unwrap_err();
    assert!(matches!(err, CliError::Mcp(_)));
    assert_eq!(err.exit_code(), 1);
}

#[test]
fn flash_erase_runs_directly() {
    let out = execute(&[
        "cmsis-dap-cli",
        "flash",
        "erase",
        "--address",
        "0",
        "--size",
        "0x1000",
    ])
    .unwrap();
    assert_eq!(out["erased"], serde_json::json!(true));
}

#[test]
fn script_runs_destructive_commands_directly() {
    let out = execute(&["cmsis-dap-cli", "script", "--text", "connect\nerase"]).unwrap();
    assert_eq!(out["ok"], serde_json::json!(true));
}

#[test]
fn repl_executes_lines_with_persistent_state() {
    let mut session = SessionManager::new(Box::new(MockBackend::new()));
    let mut reader = Cursor::new(b"connect\nreg pc 0x1000\nreg pc\nq\n".to_vec());
    repl::run(&ReplOptions::default(), &mut session, &mut reader, false).unwrap();
    assert!(session.target_info().is_some());
}

#[test]
fn repl_erase_runs_without_prompt() {
    let mut session = SessionManager::new(Box::new(MockBackend::new()));
    session
        .connect(&ConnectOptions {
            probe_id: None,
            protocol: Protocol::Swd,
            speed_khz: None,
            target: None,
            under_reset: false,
        })
        .unwrap();
    session
        .backend()
        .write_memory(
            0x2000_0000,
            cmsis_dap_core::backend::AccessWidth::U32,
            &[0xAA],
        )
        .unwrap();

    // Destructive commands run directly (no approval prompt) and clear memory.
    let mut reader = Cursor::new(b"connect\nerase\nq\n".to_vec());
    repl::run(&ReplOptions::default(), &mut session, &mut reader, true).unwrap();
    assert_eq!(
        session
            .backend()
            .read_memory(0x2000_0000, cmsis_dap_core::backend::AccessWidth::U32, 1)
            .unwrap()[0],
        0
    );
}

#[test]
fn repl_connect_uses_seeded_target() {
    let mut session = SessionManager::new(Box::new(MockBackend::new()));
    let opts = ReplOptions {
        target: Some("STM32F030C8".into()),
        ..Default::default()
    };
    let mut reader = Cursor::new(b"connect\nq\n".to_vec());
    repl::run(&opts, &mut session, &mut reader, false).unwrap();
    assert!(session.target_info().is_some());
}

#[test]
fn read_exports_bin_file() {
    let out_dir = tempfile::tempdir().unwrap();
    let dump = out_dir.path().join("dump.bin");
    let out = execute(&[
        "cmsis-dap-cli",
        "read",
        "--address",
        "0x20000000",
        "--width",
        "u8",
        "--count",
        "4",
        "--output",
        dump.to_str().unwrap(),
        "--format",
        "bin",
    ])
    .unwrap();
    assert_eq!(out["exported"], serde_json::json!(true));
    assert_eq!(out["bytes"].as_u64(), Some(4));
    assert!(dump.exists());
}

#[test]
fn chip_search_finds_builtin_chip() {
    let out = execute(&["cmsis-dap-cli", "chip", "search", "STM32F103C8"]).unwrap();
    assert!(out["count"].as_u64().unwrap() > 0);
    let chips = out["chips"].as_array().unwrap();
    assert!(chips.iter().any(|c| c["name"] == "STM32F103C8"));
}

#[test]
fn chip_list_returns_builtin_chips() {
    let out = execute(&["cmsis-dap-cli", "chip", "list"]).unwrap();
    assert!(out["count"].as_u64().unwrap() > 1000);
}

#[test]
fn info_returns_probe() {
    let out = execute(&["cmsis-dap-cli", "info"]).unwrap();
    assert!(out["probe"]["id"].as_str().is_some());
}

#[test]
fn disconnect_returns_ok() {
    let out = execute(&["cmsis-dap-cli", "disconnect"]).unwrap();
    assert_eq!(out["disconnected"], serde_json::json!(true));
}

#[test]
fn target_auto_connects() {
    let out = execute(&["cmsis-dap-cli", "target"]).unwrap();
    assert!(out["target"]["core_type"].as_str().is_some());
}

#[test]
fn regs_returns_register_names() {
    let out = execute(&["cmsis-dap-cli", "regs"]).unwrap();
    let names = out["registers"].as_array().unwrap();
    assert!(!names.is_empty());
    assert!(names.iter().any(|r| r.as_str() == Some("pc")));
}

#[test]
fn breakpoint_clear_works() {
    let out = execute(&["cmsis-dap-cli", "bp", "clear"]).unwrap();
    assert_eq!(out["cleared"], serde_json::json!(true));
}

#[test]
fn watchpoint_clear_works() {
    let out = execute(&["cmsis-dap-cli", "wp", "clear"]).unwrap();
    assert_eq!(out["cleared"], serde_json::json!(true));
}

#[test]
fn reset_run_mode_works() {
    let out = execute(&["cmsis-dap-cli", "reset", "--mode", "run"]).unwrap();
    assert_eq!(out["mode"], serde_json::json!("run"));
}
