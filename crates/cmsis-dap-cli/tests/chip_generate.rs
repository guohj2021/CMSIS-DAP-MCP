use cmsis_dap_cli::cmd::chip::{generate_yaml, parse_flm};
use object::write::{
    BinaryFormat, Object, SectionKind, Symbol, SymbolFlags, SymbolKind, SymbolScope, SymbolSection,
};
use object::{Architecture, Endianness};

/// Build a minimal ARM ELF that mimics a Keil FLM: one loadable segment with
/// code, a `FlashDevice` descriptor, and the standard entry-point symbols.
fn build_flm() -> Vec<u8> {
    let mut obj = Object::new(BinaryFormat::Elf, Architecture::Arm, Endianness::Little);
    let section = obj.add_section(vec![], b".text".to_vec(), SectionKind::Text);

    let mut data = vec![0u8; 0x13C];
    let mut desc = Vec::new();
    desc.extend_from_slice(&0x0101u16.to_le_bytes()); // Vers
    let mut devname = [0u8; 128];
    devname[..8].copy_from_slice(b"TestChip");
    desc.extend_from_slice(&devname);
    desc.extend_from_slice(&0u16.to_le_bytes()); // DevType
    desc.extend_from_slice(&0x0800_0000u32.to_le_bytes()); // DevAdr
    desc.extend_from_slice(&0x1_0000u32.to_le_bytes()); // FlashSize
    desc.extend_from_slice(&0x400u32.to_le_bytes()); // PageSize
    desc.extend_from_slice(&0u32.to_le_bytes()); // reserved
    desc.extend_from_slice(&0xFFu32.to_le_bytes()); // ErasedVal
    desc.extend_from_slice(&100u32.to_le_bytes()); // ProgPollTime
    desc.extend_from_slice(&6000u32.to_le_bytes()); // EraseSectorTimeout
    desc.extend_from_slice(&0x400u32.to_le_bytes()); // sector 0x400
    desc.extend_from_slice(&0u32.to_le_bytes()); // terminator
    data.extend_from_slice(&desc);
    obj.section_mut(section).append_data(&data, 4);

    let mut sym = |name: &[u8], value: u64, kind: SymbolKind| {
        obj.add_symbol(Symbol {
            name: name.to_vec(),
            value,
            size: 0,
            kind,
            scope: SymbolScope::Linkage,
            weak: false,
            section: SymbolSection::Section(section),
            flags: SymbolFlags::None,
        });
    };
    sym(b"Init", 0x1, SymbolKind::Text);
    sym(b"UnInit", 0x2F, SymbolKind::Text);
    sym(b"ProgramPage", 0xC3, SymbolKind::Text);
    sym(b"EraseSector", 0x81, SymbolKind::Text);
    sym(b"EraseChip", 0x3D, SymbolKind::Text);
    sym(b"PrgData", 0x138, SymbolKind::Data);
    sym(b"FlashDevice", 0x13C, SymbolKind::Data);

    obj.write().unwrap()
}

#[test]
fn parses_flm_fields() {
    let parsed = parse_flm(&build_flm()).unwrap();
    assert_eq!(parsed.pc_init, Some(0x1));
    assert_eq!(parsed.pc_uninit, Some(0x2F));
    assert_eq!(parsed.pc_program_page, Some(0xC3));
    assert_eq!(parsed.pc_erase_sector, Some(0x81));
    assert_eq!(parsed.pc_erase_all, Some(0x3D));
    assert_eq!(parsed.data_section_offset, 0x138);
    assert!(parsed.instructions.len() >= 0x13C + 0xA8);
    assert!(parsed.instructions.windows(8).any(|w| w == b"TestChip"));
    assert_eq!(parsed.device.name, "TestChip");
    assert_eq!(parsed.device.dev_addr, 0x0800_0000);
    assert_eq!(parsed.device.flash_size, 0x1_0000);
    assert_eq!(parsed.device.page_size, 0x400);
    assert_eq!(parsed.device.erased_value, 0xFF);
    assert_eq!(parsed.device.program_poll_time, 100);
    assert_eq!(parsed.device.erase_sector_timeout, 6000);
    assert_eq!(parsed.device.sectors, vec![0x400]);
    assert_eq!(parsed.pc_erase_all, Some(0x3D));
}

#[test]
fn generates_probe_rs_yaml() {
    let parsed = parse_flm(&build_flm()).unwrap();
    let yaml = generate_yaml(
        &parsed,
        "TestChip",
        0x0800_0000,
        0x0801_0000,
        0x2000_0000,
        0x2000_2000,
        "armv6m",
    )
    .unwrap();
    assert!(yaml.contains("name: TestChip"));
    assert!(yaml.contains("type: armv6m"));
    assert!(yaml.contains("!Nvm"));
    assert!(yaml.contains("!Ram"));
    assert!(yaml.contains("start: 0x8000000"));
    assert!(yaml.contains("end: 0x8010000"));
    assert!(yaml.contains("pc_init: 0x1"));
    assert!(yaml.contains("pc_program_page: 0xc3"));
    assert!(yaml.contains("pc_erase_all: 0x3d"));
    assert!(yaml.contains("data_section_offset: 0x138"));
    assert!(yaml.contains("load_address: 0x20000020"));
    assert!(yaml.contains("page_size: 0x400"));
    assert!(yaml.contains("erased_byte_value: 0xff"));
    assert!(yaml.contains("program_page_timeout: 100"));
    assert!(yaml.contains("erase_sector_timeout: 6000"));
    assert!(yaml.contains("transfer_encoding: raw"));
    assert!(yaml.contains("instructions: \""));
}

#[test]
fn generate_command_runs_with_mock_input() {
    use clap::Parser;
    use cmsis_dap_cli::cmd::{run, CliArgs};
    use std::io::Write;

    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(&build_flm()).unwrap();
    let args = CliArgs::try_parse_from(
        [
            "cmsis-dap-cli",
            "chip",
            "generate",
            "--flm",
            f.path().to_str().unwrap(),
            "--flash-start",
            "0x08000000",
            "--flash-size",
            "0x10000",
            "--sram-start",
            "0x20000000",
            "--sram-size",
            "0x2000",
            "--name",
            "TestChip",
        ]
        .iter()
        .map(|s| s.to_string()),
    )
    .unwrap();
    let out = run(
        args,
        Box::new(cmsis_dap_core::backend::mock::MockBackend::new()),
    )
    .unwrap()
    .expect("output");
    assert_eq!(out["generated"], serde_json::json!(true));
    assert_eq!(out["name"], serde_json::json!("TestChip"));
    assert!(out["yaml"].as_str().unwrap().contains("!Nvm"));
}
