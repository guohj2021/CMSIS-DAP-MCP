use cmsis_dap_core::backend::probe_rs::{builtin_registry, list_chips, search_chips};

#[test]
fn builtin_registry_lists_known_chips() {
    let registry = builtin_registry();
    let chips = list_chips(&registry);
    assert!(
        chips.len() > 1000,
        "expected a large built-in chip database"
    );
    assert!(chips.iter().any(|c| c.name == "STM32F103C8"));
    let stm32 = chips.iter().find(|c| c.name == "STM32F103C8").unwrap();
    assert!(stm32.flash.is_some(), "STM32F103C8 should define flash");
}

#[test]
fn search_chips_is_case_insensitive() {
    let registry = builtin_registry();
    let hits = search_chips(&registry, "stm32f103c8");
    assert!(
        hits.iter()
            .any(|c| c.name.eq_ignore_ascii_case("STM32F103C8")),
        "expected STM32F103C8 among hits"
    );
}
