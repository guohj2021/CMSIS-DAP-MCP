//! Symbol database + SVD introspection tests (portable, no hardware).

use cmsis_dap_core::svd::SvdDatabase;
use cmsis_dap_core::symbols::SymbolDatabase;
use std::io::Write;

fn make_elf() -> Vec<u8> {
    use object::write::{Object, Symbol as WSymbol, SymbolSection, SymbolScope};
    use object::{Architecture, BinaryFormat, Endianness, SymbolFlags, SymbolKind as ObjKind};
    let mut obj = Object::new(BinaryFormat::Elf, Architecture::Arm, Endianness::Little);
    obj.add_file_symbol("main.c".as_bytes().to_vec());
    let text = obj.add_section(Vec::new(), b".text".to_vec(), object::SectionKind::Text);
    let data = obj.add_section(Vec::new(), b".data".to_vec(), object::SectionKind::Data);
    obj.append_section_data(text, &[0x00; 64], 4);
    obj.append_section_data(data, &[0x11; 16], 4);
    obj.add_symbol(WSymbol {
        name: b"main".to_vec(),
        value: 0x0800_0100,
        size: 64,
        kind: ObjKind::Text,
        scope: SymbolScope::Dynamic,
        weak: false,
        section: SymbolSection::Section(text),
        flags: SymbolFlags::None,
    });
    obj.add_symbol(WSymbol {
        name: b"g_motor_speed".to_vec(),
        value: 0x2000_0000,
        size: 4,
        kind: ObjKind::Data,
        scope: SymbolScope::Dynamic,
        weak: false,
        section: SymbolSection::Section(data),
        flags: SymbolFlags::None,
    });
    let mut buf = Vec::new();
    obj.write_stream(&mut buf).unwrap();
    buf
}

#[test]
fn symbol_database_load_search_resolve() {
    let elf = make_elf();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("demo.axf");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(&elf).unwrap();

    let db = SymbolDatabase::load(&path).unwrap();
    assert!(!db.is_empty());

    let main = db.resolve_name("main").expect("main symbol");
    assert_eq!(main.address, 0x0800_0100);
    assert!(matches!(main.kind, cmsis_dap_core::symbols::SymbolKind::Function));
    assert_eq!(main.section.as_deref(), Some(".text"));

    let speed = db.resolve_name("g_motor_speed").expect("variable");
    assert_eq!(speed.address, 0x2000_0000);
    assert!(matches!(speed.kind, cmsis_dap_core::symbols::SymbolKind::Variable));

    // Nearest symbol at/below an address.
    let (sym, offset) = db.resolve_address(0x0800_0120).expect("near symbol");
    assert_eq!(sym.name, "main");
    assert_eq!(offset, 0x20);

    // Search + pagination.
    let (total, page) = db.search(None, Some("motor"), 0, 10);
    assert_eq!(total, 1);
    assert_eq!(page[0].name, "g_motor_speed");
}

const MINI_SVD: &str = r#"<?xml version="1.0"?>
<device schemaVersion="1.1">
<vendor>Test</vendor><name>Mini</name><version>1.0</version><description>Mini</description>
<addressUnitBits>8</addressUnitBits><width>32</width><size>32</size><access>read-write</access>
<resetValue>0x00000000</resetValue><resetMask>0xFFFFFFFF</resetMask>
<peripherals>
<peripheral><name>GPIOA</name><description>GPIO A</description><baseAddress>0x40010000</baseAddress>
<addressBlock><offset>0x0</offset><size>0x400</size><usage>registers</usage></addressBlock>
<registers><register><name>IDR</name><description>Input data</description><addressOffset>0x10</addressOffset>
<size>32</size><access>read-only</access><resetValue>0x0</resetValue>
<fields><field><name>ID0</name><bitOffset>0</bitOffset><bitWidth>1</bitWidth>
<enumeratedValues><enumeratedValue><name>LOW</name><value>0</value></enumeratedValue>
<enumeratedValue><name>HIGH</name><value>1</value></enumeratedValue></enumeratedValues>
</field></fields></register></registers></peripheral>
</peripherals></device>"#;

#[test]
fn svd_introspection_and_decode() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mini.svd");
    std::fs::write(&path, MINI_SVD).unwrap();
    let db = SvdDatabase::load(&path).unwrap();

    assert_eq!(db.list_peripherals(), vec!["GPIOA"]);
    let periph = db.get_peripheral("GPIOA").unwrap();
    assert_eq!(periph.base, 0x4001_0000);
    let reg = db.get_register("GPIOA", "IDR").unwrap();
    assert_eq!(reg.offset, 0x10);
    assert_eq!(reg.size_bits, 32);
    assert_eq!(reg.access.as_deref(), Some("read-only"));
    assert_eq!(reg.fields[0].name, "ID0");
    assert_eq!(reg.fields[0].values[0].name, "LOW");

    // decode_register: value bit0=1 -> HIGH.
    let decoded = db.decode_register("GPIOA", "IDR", 0x1).unwrap();
    assert_eq!(decoded.fields[0].value, 1);
    assert_eq!(decoded.fields[0].text.as_deref(), Some("HIGH"));

    // resolve address.
    let (addr, _) = db.resolve("GPIOA", "IDR", None).unwrap();
    assert_eq!(addr, 0x4001_0010);
}
