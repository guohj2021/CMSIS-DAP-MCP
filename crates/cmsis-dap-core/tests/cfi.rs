//! Portable tests for the `.debug_frame` CFI interpreter (incl. ARM Compiler's
//! `armcc` CIE augmentation).

use cmsis_dap_core::symbols::cfi::{DebugFrameCfi, RegRule};

// Hand-crafted .debug_frame:
//   CIE: version 3, augmentation "armcc" (+4 extra bytes), code_align 1,
//        data_align -4, ret_addr_reg 14,
//        prologue: def_cfa r13,0 ; same_value r4 ; same_value r7
//   FDE: cie@0, initial 0x08000000, range 0x100,
//        offset r4 = 4 ; advance_loc 1 ; offset r8 = 8
const DEBUG_FRAME: &[u8] = &[
    // CIE
    0x19, 0x00, 0x00, 0x00, // length = 25
    0xFF, 0xFF, 0xFF, 0xFF, // CIE id
    0x03, // version 3
    b'a', b'r', b'm', b'c', b'c', 0x00, // augmentation "armcc"
    0x01, // code_alignment_factor = 1
    0x7C, // data_alignment_factor = -4
    0x0E, // return_address_register = 14
    0x12, 0x0D, 0x00, 0x07, // armcc extra 4 bytes
    0x0C, 0x0D, 0x00, // DW_CFA_def_cfa r13, 0
    0x08, 0x04, // DW_CFA_same_value r4
    0x08, 0x07, // DW_CFA_same_value r7
    // FDE
    0x11, 0x00, 0x00, 0x00, // length = 17
    0x00, 0x00, 0x00, 0x00, // CIE pointer = 0
    0x00, 0x00, 0x00, 0x08, // initial_location = 0x08000000
    0x00, 0x01, 0x00, 0x00, // address_range = 0x100
    0x84, 0x04, // DW_CFA_offset r4 = 4
    0x41, // DW_CFA_advance_loc 1
    0x88, 0x08, // DW_CFA_offset r8 = 8
];

#[test]
fn parses_armcc_cie_and_rows() {
    let cfi = DebugFrameCfi::parse(DEBUG_FRAME, true).unwrap();
    assert_eq!(cfi.fde_count(), 1);

    // Row at 0x08000000 (before the first advance_loc): r4 = Offset(4 * -4).
    let row = cfi.row_for_address(0x0800_0000).unwrap();
    assert_eq!(row.cfa, Some((13, 0)));
    assert_eq!(row.register(4), &RegRule::Offset(-16));
    assert_eq!(row.register(7), &RegRule::SameValue);
    assert_eq!(row.register(8), &RegRule::Undefined);

    // Row at 0x08000002 still uses the first rule set.
    let row2 = cfi.row_for_address(0x0800_0002).unwrap();
    assert_eq!(row2.register(4), &RegRule::Offset(-16));

    // Row after the second advance_loc applies r8 = Offset(8 * -4).
    let row3 = cfi.row_for_address(0x0800_0006).unwrap();
    assert_eq!(row3.register(8), &RegRule::Offset(-32));

    // Outside the FDE range -> no row.
    assert!(cfi.row_for_address(0x0800_0200).is_none());
}
