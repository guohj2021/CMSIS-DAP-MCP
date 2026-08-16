use cmsis_dap_core::svd::SvdDatabase;
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

#[test]
fn parses_mini_svd_and_resolves() {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(MINI_SVD.as_bytes()).unwrap();
    let db = SvdDatabase::load(f.path()).unwrap();
    assert_eq!(db.list_peripherals(), vec!["GPIOA"]);
    let (addr, field) = db.resolve("GPIOA", "ODR", Some("ODR0")).unwrap();
    assert_eq!(addr, 0x4800_0014);
    assert_eq!(field, Some((0x1, 0)));
}
