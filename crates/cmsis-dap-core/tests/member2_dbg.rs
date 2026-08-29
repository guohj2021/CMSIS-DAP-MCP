// Temporary: verify struct member size + value path with a fake reader.
use cmsis_dap_core::symbols::DwarfLocals;
use std::collections::HashMap;

#[test]
fn member_sizes() {
    let path = r"C:\Workspace\DemoWorkspace\workspace\DemoMCU_SDK\TEST_Example\HAL_Driver\Project\keil\Objects\demo.axf";
    let locals = DwarfLocals::load(std::path::Path::new(path))
        .unwrap()
        .unwrap();
    let regs: HashMap<u16, u64> = HashMap::from([(13u16, 0x2000_2000u64), (15u16, 0x0800_5A38u64)]);
    let mut read = |_a: u64, b: &mut [u8]| -> Result<(), String> {
        for x in b.iter_mut() {
            *x = 0xAA;
        }
        Ok(())
    };
    let lv = locals.locals_at(0x0800_5A38, &regs, Some(0x2000_2000), &mut read);
    for v in &lv {
        if v.name == "GPIO_InitStruct" {
            println!("struct {} children:", v.type_name);
            for c in &v.children {
                println!(
                    "  {} : {} ({}) = {} @{:?}",
                    c.name, c.type_name, c.kind, c.value, c.address
                );
            }
        }
    }
}
