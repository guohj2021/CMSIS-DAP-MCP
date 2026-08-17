use clap::Parser;
use cmsis_dap_cli::cmd::actions;
use cmsis_dap_cli::cmd::{run, CliArgs, DumpArgs};
use cmsis_dap_core::backend::mock::MockBackend;
use cmsis_dap_core::backend::{AccessWidth, ConnectOptions, CoreRegister, Protocol};
use cmsis_dap_core::session::SessionManager;
use std::io::Cursor;

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

fn execute(args: &[&str]) -> Result<serde_json::Value, cmsis_dap_cli::cmd::CliError> {
    let args = CliArgs::try_parse_from(args.iter().map(|s| s.to_string())).expect("parse");
    let out = run(args, Box::new(MockBackend::new()))?;
    Ok(out.expect("structured output"))
}

#[test]
fn dump_via_run_returns_cpu_state() {
    let out = execute(&["cmsis-dap-cli", "dump", "--address", "0x20000000"]).unwrap();
    assert_eq!(out["state"], "running");
    assert!(out["registers"].is_array());
    assert!(out["fault"].is_array());
    assert!(out["memory"].as_array().map(|a| a.len()).unwrap_or(0) >= 1);
}

#[test]
fn dump_restores_running_core() {
    let mut session = connect(MockBackend::new());
    session
        .backend()
        .write_core_register(&CoreRegister::Name("pc".into()), 0x0800_0100)
        .unwrap();
    session
        .backend()
        .write_memory(0x2000_0000, AccessWidth::U32, &[0x1234_5678])
        .unwrap();
    let args = DumpArgs {
        addresses: vec!["0x20000000".into()],
        stack_words: 4,
        no_restore: false,
    };
    let out = actions::dump(&mut session, None, &args).unwrap();
    assert_eq!(out["pc"].as_u64(), Some(0x0800_0100));
    assert_eq!(out["memory"][0]["value"].as_u64(), Some(0x1234_5678));
    assert_eq!(
        session.backend().get_core_status().unwrap().state,
        "running"
    );

    // no-restore leaves the core halted.
    let args = DumpArgs {
        addresses: vec![],
        stack_words: 0,
        no_restore: true,
    };
    actions::dump(&mut session, None, &args).unwrap();
    assert_eq!(session.backend().get_core_status().unwrap().state, "halted");
}

#[test]
fn dump_resolves_elf_symbol_addresses() {
    use object::write::{
        BinaryFormat, Object, SectionKind, Symbol, SymbolFlags, SymbolKind, SymbolScope,
        SymbolSection,
    };
    use object::{Architecture, Endianness};
    use std::io::Write;

    let dir = tempfile::tempdir().unwrap();
    let elf = dir.path().join("fw.elf");
    let mut obj = Object::new(BinaryFormat::Elf, Architecture::Arm, Endianness::Little);
    let section = obj.add_section(vec![], b".data".to_vec(), SectionKind::Data);
    obj.section_mut(section).append_data(&[0u8; 16], 4);
    obj.add_symbol(Symbol {
        name: b"counter".to_vec(),
        value: 0x2000_0000,
        size: 4,
        kind: SymbolKind::Data,
        scope: SymbolScope::Linkage,
        weak: false,
        section: SymbolSection::Section(section),
        flags: SymbolFlags::None,
    });
    let mut f = std::fs::File::create(&elf).unwrap();
    f.write_all(&obj.write().unwrap()).unwrap();

    let mut session = connect(MockBackend::new());
    session
        .backend()
        .write_memory(0x2000_0000, AccessWidth::U32, &[0xCAFE_BABE])
        .unwrap();
    let args = DumpArgs {
        addresses: vec!["counter".into()],
        stack_words: 0,
        no_restore: false,
    };
    let out = actions::dump(&mut session, Some(&elf), &args).unwrap();
    assert_eq!(out["memory"][0]["address"].as_u64(), Some(0x2000_0000));
    assert_eq!(out["memory"][0]["value"].as_u64(), Some(0xCAFE_BABE));
}

#[test]
fn repl_dump_command_works() {
    use cmsis_dap_cli::cmd::repl;
    use cmsis_dap_cli::cmd::ReplOptions;
    let mut session = connect(MockBackend::new());
    session
        .backend()
        .write_memory(0x2000_0000, AccessWidth::U32, &[0xDEAD_BEEF])
        .unwrap();
    let mut reader =
        Cursor::new(b"connect\ndump --address 0x20000000 --stack-words 2\nq\n".to_vec());
    repl::run(&ReplOptions::default(), &mut session, &mut reader, false).unwrap();
    // The dump should have restored the run state.
    assert_eq!(
        session.backend().get_core_status().unwrap().state,
        "running"
    );
}

#[test]
fn server_args_parse() {
    fn parse_err(args: &[&str]) -> clap::Error {
        CliArgs::try_parse_from(args.iter().map(|s| s.to_string())).expect_err("should fail")
    }
    let ok = CliArgs::try_parse_from(
        [
            "cmsis-dap-cli",
            "gdb-server",
            "--port",
            "3333",
            "--reset-halt",
        ]
        .iter()
        .map(|s| s.to_string()),
    )
    .unwrap();
    assert!(matches!(
        ok.command,
        cmsis_dap_cli::cmd::Command::GdbServer(_)
    ));
    let ok = CliArgs::try_parse_from(
        ["cmsis-dap-cli", "tcp-server", "--port", "5000"]
            .iter()
            .map(|s| s.to_string()),
    )
    .unwrap();
    assert!(matches!(
        ok.command,
        cmsis_dap_cli::cmd::Command::TcpServer(_)
    ));
    assert_eq!(
        parse_err(&["cmsis-dap-cli", "gdb-server", "--port", "abc"]).exit_code(),
        2
    );
}
