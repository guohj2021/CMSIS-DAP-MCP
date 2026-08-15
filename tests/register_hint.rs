use cmsis_dap_mcp::backend::{register_hint, RegisterHint};

#[test]
fn classifies_special_registers() {
    assert_eq!(register_hint("pc"), RegisterHint::ProgramCounter);
    assert_eq!(register_hint("PC"), RegisterHint::ProgramCounter);
    assert_eq!(register_hint("sp"), RegisterHint::StackPointer);
    assert_eq!(register_hint("fp"), RegisterHint::FramePointer);
    assert_eq!(register_hint("lr"), RegisterHint::ReturnAddress);
    assert_eq!(register_hint("ra"), RegisterHint::ReturnAddress);
    assert_eq!(register_hint("psr"), RegisterHint::ProcessorStatus);
    assert_eq!(register_hint("xpsr"), RegisterHint::ProcessorStatus);
    assert_eq!(register_hint("msp"), RegisterHint::MainStackPointer);
    assert_eq!(register_hint("psp"), RegisterHint::ProcessStackPointer);
    assert_eq!(register_hint("fpsr"), RegisterHint::FpuStatus);
}

#[test]
fn classifies_general_registers() {
    assert_eq!(register_hint("r0"), RegisterHint::GeneralIndex(0));
    assert_eq!(register_hint("r15"), RegisterHint::GeneralIndex(15));
    assert_eq!(register_hint("R7"), RegisterHint::GeneralIndex(7));
}

#[test]
fn classifies_unknown_names_as_byname() {
    assert_eq!(register_hint("primask"), RegisterHint::ByName);
    assert_eq!(register_hint("r16"), RegisterHint::ByName);
    assert_eq!(register_hint("r"), RegisterHint::ByName);
}
