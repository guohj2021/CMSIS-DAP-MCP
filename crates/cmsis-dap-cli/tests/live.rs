use clap::Parser;
use cmsis_dap_cli::cmd::live;
use cmsis_dap_cli::cmd::repl;
use cmsis_dap_cli::cmd::{run, CliArgs, ReplOptions};
use cmsis_dap_core::backend::mock::MockBackend;
use cmsis_dap_core::backend::{ConnectOptions, Protocol};
use cmsis_dap_core::session::SessionManager;
use object::write::{
    BinaryFormat, Object, SectionKind, Symbol, SymbolFlags, SymbolKind, SymbolScope, SymbolSection,
};
use object::{Architecture, Endianness};
use serde_json::Value;
use std::io::Cursor;
use std::path::Path;

/// Build a tiny ARM ELF with data symbols used by watch/RTT/EV Recorder.
fn build_elf() -> Vec<u8> {
    let mut obj = Object::new(BinaryFormat::Elf, Architecture::Arm, Endianness::Little);
    let section = obj.add_section(vec![], b".data".to_vec(), SectionKind::Data);
    let data = vec![0u8; 0x400];
    obj.section_mut(section).append_data(&data, 4);
    let mut sym = |name: &[u8], value: u64| {
        obj.add_symbol(Symbol {
            name: name.to_vec(),
            value,
            size: 4,
            kind: SymbolKind::Data,
            scope: SymbolScope::Linkage,
            weak: false,
            section: SymbolSection::Section(section),
            flags: SymbolFlags::None,
        });
    };
    sym(b"counter", 0x2000_0000);
    sym(b"_SEGGER_RTT", 0x2000_0100);
    sym(b"EventRecorderInfo", 0x2000_0200);
    obj.write().unwrap()
}

fn write_elf(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("firmware.elf");
    std::fs::write(&path, build_elf()).unwrap();
    path
}

fn connect(mock: MockBackend) -> SessionManager {
    let mut session = SessionManager::new(Box::new(mock));
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
}

fn assert_timestamped(line: &str) {
    assert!(
        line.starts_with('[')
            && line.len() > 20
            && line.as_bytes()[1..5].iter().all(u8::is_ascii_digit),
        "line is not timestamped: {line:?}"
    );
}

fn parse_json_lines(stdout: &[u8]) -> Vec<Value> {
    let text = String::from_utf8_lossy(stdout);
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("line is JSON"))
        .collect()
}

#[test]
fn symbols_list_and_resolve() {
    let dir = tempfile::tempdir().unwrap();
    let elf = write_elf(dir.path());
    let list = live::symbols_list(Some(&elf), None).unwrap();
    let symbols = list["symbols"].as_array().unwrap();
    assert!(symbols.iter().any(|s| s["name"] == "counter"));
    assert!(symbols.iter().any(|s| s["name"] == "_SEGGER_RTT"));
    assert!(symbols.iter().any(|s| s["name"] == "EventRecorderInfo"));

    let filtered = live::symbols_list(Some(&elf), Some("count")).unwrap();
    assert_eq!(filtered["count"].as_u64(), Some(1));
    assert_eq!(filtered["symbols"][0]["name"], "counter");

    let resolved = live::symbols_resolve(Some(&elf), "counter").unwrap();
    assert_eq!(resolved["address"].as_u64(), Some(0x2000_0000));
    let missing = live::symbols_resolve(Some(&elf), "nope").unwrap();
    assert_eq!(missing["found"], serde_json::json!(false));
}

#[test]
fn symbols_require_elf() {
    let err = live::symbols_list(None, None).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn watch_monitors_with_timestamp_and_log_export() {
    let dir = tempfile::tempdir().unwrap();
    let elf = write_elf(dir.path());
    let symbols = cmsis_dap_cli::cmd::symbols::load_symbols(&elf).unwrap();

    let mut session = connect(MockBackend::new());
    session
        .backend()
        .write_memory(
            0x2000_0000,
            cmsis_dap_core::backend::AccessWidth::U32,
            &[0x1234_5678],
        )
        .unwrap();
    let (address, label) = live::resolve_target("counter", Some(&symbols)).unwrap();
    assert_eq!(address, 0x2000_0000);
    assert_eq!(label, "counter");

    let log_dir = dir.path().join("logs");
    let mut stdout = Vec::new();
    let polls = live::watch_run(
        &mut session,
        &[live::WatchItem {
            label: "counter".into(),
            address,
            width: cmsis_dap_core::backend::AccessWidth::U32,
        }],
        1,
        2,
        false,
        &mut stdout,
        live::LogTarget::Dir(log_dir.clone()),
    )
    .unwrap();
    assert_eq!(polls, 2);
    let text = String::from_utf8_lossy(&stdout);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_timestamped(lines[0]);
    assert!(lines[0].contains("counter = 0x12345678"));
    assert!(lines[1].contains("counter = 0x12345678"));

    let log_files: Vec<_> = std::fs::read_dir(&log_dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(log_files.len(), 1);
    let log = std::fs::read_to_string(&log_files[0]).unwrap();
    assert_eq!(log, text);
}

#[test]
fn watch_json_emits_ndjson_with_host_ts() {
    let dir = tempfile::tempdir().unwrap();
    let elf = write_elf(dir.path());
    let symbols = cmsis_dap_cli::cmd::symbols::load_symbols(&elf).unwrap();
    let mut session = connect(MockBackend::new());
    let (address, _) = live::resolve_target("counter", Some(&symbols)).unwrap();
    let mut stdout = Vec::new();
    live::watch_run(
        &mut session,
        &[live::WatchItem {
            label: "counter".into(),
            address,
            width: cmsis_dap_core::backend::AccessWidth::U32,
        }],
        1,
        1,
        true,
        &mut stdout,
        live::LogTarget::Dir(dir.path().to_path_buf()),
    )
    .unwrap();
    let rows = parse_json_lines(&stdout);
    assert_eq!(rows.len(), 1);
    assert!(rows[0]["host_ts"].as_str().unwrap().contains('T'));
    assert_eq!(rows[0]["target"], "counter");
    assert_eq!(rows[0]["width"], "u32");
    assert_eq!(rows[0]["value"].as_u64(), Some(0));
}

#[test]
fn watch_symbol_requires_elf_at_runtime() {
    let mut session = connect(MockBackend::new());
    let err = live::resolve_target("counter", None).unwrap_err();
    assert_eq!(err.exit_code(), 2);

    // A literal address works without an ELF.
    let (address, label) = live::resolve_target("0x20000000", None).unwrap();
    assert_eq!(address, 0x2000_0000);
    assert_eq!(label, "0x20000000");
    let _ = &mut session;
}

#[test]
fn watch_log_file_appends() {
    let dir = tempfile::tempdir().unwrap();
    let log_file = dir.path().join("rtt.log");
    std::fs::write(&log_file, "old line\n").unwrap();
    let mut session = connect(MockBackend::new());
    let mut stdout = Vec::new();
    live::watch_run(
        &mut session,
        &[live::WatchItem {
            label: "0x20000000".into(),
            address: 0x2000_0000,
            width: cmsis_dap_core::backend::AccessWidth::U32,
        }],
        1,
        1,
        false,
        &mut stdout,
        live::LogTarget::File(log_file.clone()),
    )
    .unwrap();
    let log = std::fs::read_to_string(&log_file).unwrap();
    assert!(log.starts_with("old line\n"));
    assert_eq!(log.lines().count(), 2);
    assert_timestamped(log.lines().nth(1).unwrap());
}

#[test]
fn rtt_monitor_multi_channel_and_json() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = connect(MockBackend::with_rtt(&[
        (Some("up0"), b"hello "),
        (Some("up1"), b"world"),
    ]));
    let args = cmsis_dap_cli::cmd::RttMonitorArgs {
        channel: "0,1".into(),
        interval_ms: 1,
        count: 2,
        address: None,
        max_bytes: 1024,
        log_dir: Some(dir.path().to_path_buf()),
        log_file: None,
    };
    let mut stdout = Vec::new();
    live::rtt_monitor(&mut session, &args, None, false, &mut stdout).unwrap();
    let text = String::from_utf8_lossy(&stdout);
    assert!(text.contains("[RTT0 \"up0\"] hello"));
    assert!(text.contains("[RTT1 \"up1\"] world"));
    for line in text.lines() {
        assert_timestamped(line);
    }

    // JSON mode with a fresh mock (pending bytes are drained per session).
    let mut session = connect(MockBackend::with_rtt(&[
        (Some("up0"), b"hello "),
        (Some("up1"), b"world"),
    ]));
    let mut json_out = Vec::new();
    live::rtt_monitor(&mut session, &args, None, true, &mut json_out).unwrap();
    let rows = parse_json_lines(&json_out);
    assert!(!rows.is_empty());
    assert!(rows
        .iter()
        .any(|r| r["channel"] == 0 && r["data_hex"] == "68656C6C6F20"));
    assert!(rows
        .iter()
        .any(|r| r["channel"] == 1 && r["data_hex"] == "776F726C64"));
    assert!(rows.iter().all(|r| r["host_ts"].is_string()));
}

#[test]
fn rtt_rejects_missing_channel() {
    let mut session = connect(MockBackend::with_rtt(&[(None, b"x")]));
    let args = cmsis_dap_cli::cmd::RttMonitorArgs {
        channel: "5".into(),
        interval_ms: 1,
        count: 1,
        address: None,
        max_bytes: 1024,
        log_dir: None,
        log_file: None,
    };
    let err = live::rtt_monitor(&mut session, &args, None, false, &mut Vec::new()).unwrap_err();
    assert_eq!(err.exit_code(), 2);
    assert!(err.to_string().contains("channel 5"));
}

fn evr_record(ts: u32, val1: u32, val2: u32, context: u8, component: u8, message: u8) -> [u8; 16] {
    let mut r = [0u8; 16];
    r[0..4].copy_from_slice(&ts.to_le_bytes());
    r[4..8].copy_from_slice(&val1.to_le_bytes());
    r[8..12].copy_from_slice(&val2.to_le_bytes());
    let info =
        (message as u32) | ((component as u32) << 8) | ((context as u32) << 16) | 0x0800_0000; // VALID
    r[12..16].copy_from_slice(&info.to_le_bytes());
    r
}

#[test]
fn evr_info_and_monitor_decode_events() {
    let dir = tempfile::tempdir().unwrap();
    let records = vec![
        evr_record(0x1000, 1, 2, 0, 0xFE, 0x01),
        evr_record(0x2000, 3, 4, 2, 0x03, 0x02),
    ];
    let mut session = connect(MockBackend::with_evr(8, 1_000_000, records));
    let info = live::evr_info(&mut session, None, Some(0x2000_0200)).unwrap();
    assert_eq!(info["evr"]["record_count"].as_u64(), Some(8));
    assert_eq!(info["evr"]["protocol_version"], "1.1");

    let log_dir = dir.path().join("evr-logs");
    let args = cmsis_dap_cli::cmd::EvrMonitorArgs {
        interval_ms: 1,
        count: 1,
        ctx: vec![],
        address: Some(0x2000_0200),
        log_dir: Some(log_dir.clone()),
        log_file: None,
    };
    let mut stdout = Vec::new();
    live::evr_monitor(&mut session, &args, None, false, &mut stdout).unwrap();
    let text = String::from_utf8_lossy(&stdout);
    assert!(text.contains("ctx=0x0 comp=0xFE msg=0x01"));
    assert!(text.contains("val1=0x00000001"));
    assert!(text.contains("ctx=0x2 comp=0x03 msg=0x02"));
    assert!(text.contains("val1=0x00000003"));
    assert!(text.contains("t=0.004096s"));
    assert!(text.lines().all(|l| l.starts_with('[')));
    let log_files: Vec<_> = std::fs::read_dir(&log_dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(log_files.len(), 1);
    assert_eq!(
        std::fs::read_to_string(&log_files[0]).unwrap(),
        String::from_utf8_lossy(&stdout)
    );

    // JSON mode with a context filter.
    let json_log_dir = dir.path().join("evr-json");
    let args = cmsis_dap_cli::cmd::EvrMonitorArgs {
        interval_ms: 1,
        count: 1,
        ctx: vec![2],
        address: Some(0x2000_0200),
        log_dir: Some(json_log_dir),
        log_file: None,
    };
    let mut json_out = Vec::new();
    live::evr_monitor(&mut session, &args, None, true, &mut json_out).unwrap();
    let rows = parse_json_lines(&json_out);
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|r| r["context"] == 2));
    assert!(rows.iter().all(|r| r["host_ts"].is_string()));
}

#[test]
fn evr_requires_address_without_elf() {
    let mut session = connect(MockBackend::new());
    let err = live::evr_info(&mut session, None, None).unwrap_err();
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn watch_args_validation() {
    fn parse_err(args: &[&str]) -> clap::Error {
        CliArgs::try_parse_from(args.iter().map(|s| s.to_string())).expect_err("args should fail")
    }
    assert_eq!(
        parse_err(&["cmsis-dap-cli", "watch"]).exit_code(),
        2,
        "watch without targets"
    );
    assert_eq!(
        parse_err(&["cmsis-dap-cli", "watch", "--width", "u128", "counter"]).exit_code(),
        2,
        "invalid width"
    );
    assert_eq!(
        parse_err(&[
            "cmsis-dap-cli",
            "watch",
            "counter",
            "--log-dir",
            "a",
            "--log-file",
            "b"
        ])
        .exit_code(),
        2,
        "log-dir/log-file conflict"
    );
    assert_eq!(
        parse_err(&["cmsis-dap-cli", "evr", "monitor", "--ctx", "9"]).exit_code(),
        2,
        "invalid evr context"
    );
}

#[test]
fn symbols_command_via_run() {
    let dir = tempfile::tempdir().unwrap();
    let elf = write_elf(dir.path());
    let args = CliArgs::try_parse_from(
        [
            "cmsis-dap-cli",
            "--elf",
            elf.to_str().unwrap(),
            "symbols",
            "resolve",
            "counter",
        ]
        .iter()
        .map(|s| s.to_string()),
    )
    .unwrap();
    let out = run(args, Box::new(MockBackend::new())).unwrap().unwrap();
    assert_eq!(out["address"].as_u64(), Some(0x2000_0000));
}

#[test]
fn repl_watch_rtt_evr_commands() {
    let dir = tempfile::tempdir().unwrap();
    let elf = write_elf(dir.path());
    let mut session = connect(MockBackend::with_rtt(&[(Some("up0"), b"ping")]));
    let opts = ReplOptions {
        elf: Some(elf),
        ..Default::default()
    };
    let log_dir = dir.path().join("logs");
    let input = format!(
        "connect\n\
         watch add counter\n\
         watch list\n\
         watch interval 1\n\
         watch run --count 1 --log-dir {}\n\
         rtt monitor --count 1 --interval-ms 1 --log-dir {}\n\
         evr info\n\
         q\n",
        log_dir.display(),
        log_dir.display()
    );
    let mut reader = Cursor::new(input.into_bytes());
    repl::run(&opts, &mut session, &mut reader, false).unwrap();
}
