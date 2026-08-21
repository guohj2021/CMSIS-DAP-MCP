use clap::Parser;
use cmsis_dap_cli::cmd::{option, run, swo, CliArgs, CliError, Command};
use cmsis_dap_core::backend::mock::MockBackend;

fn parse(args: &[&str]) -> CliArgs {
    CliArgs::try_parse_from(args.iter().map(|s| s.to_string())).expect("args should parse")
}

fn parse_err(args: &[&str]) -> clap::Error {
    CliArgs::try_parse_from(args.iter().map(|s| s.to_string())).expect_err("args should fail")
}

#[test]
fn parses_list() {
    let args = parse(&["cmsis-dap-cli", "list"]);
    assert!(matches!(args.command, Command::List));
}

#[test]
fn parses_global_connection_flags() {
    let args = parse(&[
        "cmsis-dap-cli",
        "--probe-id",
        "ABC123",
        "--protocol",
        "jtag",
        "--speed-khz",
        "4000",
        "--under-reset",
        "connect",
    ]);
    assert_eq!(args.probe_id.as_deref(), Some("ABC123"));
    assert_eq!(args.protocol, "jtag");
    assert_eq!(args.speed_khz, Some(4000));
    assert!(args.under_reset);
}

#[test]
fn rejects_invalid_protocol() {
    let err = parse_err(&["cmsis-dap-cli", "--protocol", "i2c", "connect"]);
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn rejects_invalid_width() {
    let err = parse_err(&[
        "cmsis-dap-cli",
        "read",
        "--address",
        "0x20000000",
        "--width",
        "u128",
    ]);
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn rejects_invalid_format() {
    let err = parse_err(&[
        "cmsis-dap-cli",
        "flash",
        "program",
        "--address",
        "0x8000000",
        "--file",
        "fw.bin",
        "--format",
        "srec",
    ]);
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn missing_required_address_fails() {
    let err = parse_err(&["cmsis-dap-cli", "read"]);
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn parses_hex_address_and_values() {
    let args = parse(&[
        "cmsis-dap-cli",
        "write",
        "--address",
        "0x20000000",
        "--width",
        "u32",
        "--values",
        "0xDEADBEEF,1,0x10",
    ]);
    let Command::Write(w) = args.command else {
        panic!("expected write command");
    };
    assert_eq!(w.address, 0x2000_0000);
    assert_eq!(w.width, "u32");
    assert_eq!(w.values, vec![0xDEAD_BEEF, 1, 0x10]);
}

#[test]
fn parses_read_export_flags() {
    let args = parse(&[
        "cmsis-dap-cli",
        "read",
        "--address",
        "0x8000000",
        "--width",
        "u8",
        "--count",
        "0x100",
        "--output",
        "dump.bin",
        "--format",
        "hex",
    ]);
    let Command::Read(r) = args.command else {
        panic!("expected read command");
    };
    assert_eq!(r.count, 0x100);
    assert_eq!(
        r.output.as_deref().map(|p| p.to_string_lossy().to_string()),
        Some("dump.bin".into())
    );
    assert_eq!(r.format, "hex");
}

#[test]
fn parses_reg_get_and_set() {
    let get = parse(&["cmsis-dap-cli", "reg", "get", "pc"]);
    let Command::Reg(g) = get.command else {
        panic!("expected reg command");
    };
    assert!(matches!(g.action, cmsis_dap_cli::cmd::RegAction::Get(_)));

    let set = parse(&["cmsis-dap-cli", "reg", "set", "r0", "0x1234"]);
    let Command::Reg(s) = set.command else {
        panic!("expected reg command");
    };
    let cmsis_dap_cli::cmd::RegAction::Set(sargs) = s.action else {
        panic!("expected set action");
    };
    assert_eq!(sargs.register, "r0");
    assert_eq!(sargs.value, 0x1234);
}

#[test]
fn parses_flash_program_with_verify() {
    let args = parse(&[
        "cmsis-dap-cli",
        "flash",
        "program",
        "--address",
        "0x8000000",
        "--file",
        "fw.hex",
        "--verify",
    ]);
    let Command::Flash(f) = args.command else {
        panic!("expected flash command");
    };
    let cmsis_dap_cli::cmd::FlashAction::Program(p) = f.action else {
        panic!("expected program action");
    };
    assert!(p.verify);
    assert_eq!(p.address, 0x800_0000);
}

#[test]
fn parses_svd_read_target_and_json() {
    let args = parse(&["cmsis-dap-cli", "--json", "svd", "read", "GPIOA.ODR.ODR0"]);
    assert!(args.json);
    let Command::Svd(s) = args.command else {
        panic!("expected svd command");
    };
    let cmsis_dap_cli::cmd::SvdAction::Read(r) = s.action else {
        panic!("expected read action");
    };
    assert_eq!(r.target, "GPIOA.ODR.ODR0");
}

#[test]
fn script_requires_exactly_one_source() {
    let args = parse(&["cmsis-dap-cli", "script"]);
    let err = run(args, Box::new(MockBackend::new())).unwrap_err();
    assert!(matches!(err, CliError::InvalidArgument(_)));
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn parses_chip_list() {
    let args = parse(&["cmsis-dap-cli", "chip", "list"]);
    assert!(matches!(args.command, Command::Chip(_)));
}

#[test]
fn chip_search_requires_keyword() {
    let err = parse_err(&["cmsis-dap-cli", "chip", "search"]);
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn parses_swo_start() {
    let args = parse(&[
        "cmsis-dap-cli",
        "swo",
        "start",
        "--baud",
        "2000000",
        "--tpiu-clock",
        "8000000",
    ]);
    match args.command {
        Command::Swo(a) => match a.action {
            swo::SwoAction::Start(s) => {
                assert_eq!(s.baud, 2_000_000);
                assert_eq!(s.tpiu_clock, 8_000_000);
            }
            _ => panic!("expected swo start"),
        },
        _ => panic!("expected swo command"),
    }
}

#[test]
fn parses_swo_monitor() {
    let args = parse(&[
        "cmsis-dap-cli",
        "swo",
        "monitor",
        "--count",
        "5",
        "--interval-ms",
        "50",
    ]);
    match args.command {
        Command::Swo(a) => match a.action {
            swo::SwoAction::Monitor(m) => {
                assert_eq!(m.count, 5);
                assert_eq!(m.interval_ms, 50);
            }
            _ => panic!("expected swo monitor"),
        },
        _ => panic!("expected swo command"),
    }
}

#[test]
fn parses_option_read_write() {
    let read = parse(&["cmsis-dap-cli", "option", "read"]);
    match read.command {
        Command::Option(a) => assert!(matches!(a.action, option::OptionAction::Read)),
        _ => panic!("expected option command"),
    }
    let write = parse(&["cmsis-dap-cli", "option", "write", "DATA0", "0x55"]);
    match write.command {
        Command::Option(a) => match a.action {
            option::OptionAction::Write(w) => {
                assert_eq!(w.name, "DATA0");
                assert_eq!(w.value, 0x55);
            }
            _ => panic!("expected option write"),
        },
        _ => panic!("expected option command"),
    }
}
