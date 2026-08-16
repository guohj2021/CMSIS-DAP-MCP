use cmsis_dap_core::backend::mock::MockBackend;
use cmsis_dap_core::backend::{AccessWidth, ConnectOptions, Protocol};
use cmsis_dap_core::script;
use cmsis_dap_core::security::SecurityPolicy;
use cmsis_dap_core::session::SessionManager;
use std::io::Write;

fn session() -> SessionManager {
    SessionManager::new(Box::new(MockBackend::new()))
}

fn policy(allow_destructive: bool) -> SecurityPolicy {
    SecurityPolicy { allow_destructive }
}

fn connect(sm: &mut SessionManager) {
    sm.connect(&ConnectOptions {
        probe_id: None,
        protocol: Protocol::Swd,
        speed_khz: None,
        target: None,
        under_reset: false,
    })
    .unwrap();
}

#[test]
fn runs_linear_commands_and_reports_results() {
    let mut sm = session();
    connect(&mut sm);
    let report = script::run(&mut sm, &policy(false), "halt\ngo\nmem32 0x20000000 2\n").unwrap();
    assert!(report.ok);
    assert_eq!(report.commands, 3);
    assert!(report.results.iter().all(|r| r.status == "ok"));
    assert_eq!(report.results[2].output["values"][0], serde_json::json!(0));
}

#[test]
fn supports_openocd_aliases_and_semicolons() {
    let mut sm = session();
    connect(&mut sm);
    let report = script::run(
        &mut sm,
        &policy(false),
        "halt; mdw 0x20000000 2; resume; reg pc",
    )
    .unwrap();
    assert!(report.ok);
    assert_eq!(report.commands, 4);
    assert_eq!(
        report.results[3].output["value"],
        serde_json::json!(0x0800_0100)
    );
}

#[test]
fn unknown_command_fails_and_stops() {
    let mut sm = session();
    connect(&mut sm);
    let report = script::run(&mut sm, &policy(false), "halt\nfrobnicate 1\nstep\n").unwrap();
    assert!(!report.ok);
    assert_eq!(report.results[1].status, "error");
    assert_eq!(report.commands, 2);
}

#[test]
fn destructive_commands_require_flag() {
    let mut sm = session();
    connect(&mut sm);
    let report = script::run(&mut sm, &policy(false), "erase\n").unwrap();
    assert!(!report.ok);
    assert_eq!(report.results[0].status, "error");
    assert_eq!(
        report.results[0].output["code"],
        serde_json::json!("DestructiveDisabled")
    );
}

#[test]
fn destructive_commands_run_with_flag() {
    let mut sm = session();
    connect(&mut sm);
    let report = script::run(&mut sm, &policy(true), "erase\n").unwrap();
    assert!(report.ok, "{report:?}");
}

#[test]
fn savebin_and_loadbin_roundtrip() {
    let mut sm = session();
    connect(&mut sm);
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("dump.bin");
    let report = script::run(
        &mut sm,
        &policy(true),
        &format!(
            "w32 0x20000000 0x11223344\nsavebin {} 0x20000000 4\n",
            bin.display()
        ),
    )
    .unwrap();
    assert!(report.ok, "{report:?}");
    let bytes = std::fs::read(&bin).unwrap();
    assert_eq!(bytes, vec![0x44, 0x33, 0x22, 0x11]);

    // loadbin writes the file bytes back into memory
    let report = script::run(
        &mut sm,
        &policy(true),
        &format!("loadbin {} 0x20000010\n", bin.display()),
    )
    .unwrap();
    assert!(report.ok, "{report:?}");
    let vals = sm
        .backend()
        .read_memory(0x2000_0010, AccessWidth::U8, 4)
        .unwrap();
    assert_eq!(vals, vec![0x44, 0x33, 0x22, 0x11]);
}

#[test]
fn verifybin_reports_mismatch() {
    let mut sm = session();
    connect(&mut sm);
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("expect.bin");
    let mut f = std::fs::File::create(&bin).unwrap();
    f.write_all(&[0x99, 0x00, 0x00, 0x00]).unwrap();
    let report = script::run(
        &mut sm,
        &policy(false),
        &format!("verifybin {} 0x20000000\n", bin.display()),
    )
    .unwrap();
    assert!(report.ok);
    assert_eq!(
        report.results[0].output["verified"],
        serde_json::json!(false)
    );
}

#[test]
fn sleep_and_echo_are_supported() {
    let mut sm = session();
    let report = script::run(&mut sm, &policy(false), "echo hello world\nsleep 5\n").unwrap();
    assert!(report.ok);
    assert_eq!(
        report.results[0].output["text"],
        serde_json::json!("hello world")
    );
}

#[test]
fn comments_and_blank_lines_are_ignored() {
    let mut sm = session();
    connect(&mut sm);
    let report = script::run(
        &mut sm,
        &policy(false),
        "# comment\n\n// another comment\nhalt\n",
    )
    .unwrap();
    assert!(report.ok);
    assert_eq!(report.commands, 1);
}
