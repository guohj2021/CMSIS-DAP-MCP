use cmsis_dap_mcp::hex::encode_ihex;

#[test]
fn encodes_small_range() {
    let data = [0x01u8, 0x02, 0x03, 0x04];
    let hex = encode_ihex(&data, 0x0800_0000);
    let lines: Vec<&str> = hex.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], ":020000040800F2");
    assert_eq!(lines[1], ":0400000001020304F2");
    assert_eq!(lines[2], ":00000001FF");
}

#[test]
fn emits_extended_linear_address_on_64k_boundary() {
    // 4 bytes starting at 0x08010000 must emit a type-04 ELA record.
    let data = [0xAAu8; 4];
    let hex = encode_ihex(&data, 0x0801_0000);
    let lines: Vec<&str> = hex.lines().collect();
    assert_eq!(lines[0], ":020000040801F1");
    assert_eq!(lines[1], ":04000000AAAAAAAA54");
    assert_eq!(lines[2], ":00000001FF");
}

#[test]
fn checksum_is_two_complement() {
    let data = [0x11u8, 0x22, 0x33, 0x44];
    let hex = encode_ihex(&data, 0x2000_0000);
    let first = hex.lines().next().unwrap();
    let checksum = u8::from_str_radix(&first[first.len() - 2..], 16).unwrap();
    // sum of all bytes in the record (length, address, type, data) must be 0 mod 256
    let body = &first[1..first.len() - 2];
    let sum: u32 = (0..body.len())
        .step_by(2)
        .map(|i| u32::from(u8::from_str_radix(&body[i..i + 2], 16).unwrap()))
        .sum();
    assert_eq!((sum + u32::from(checksum)) % 256, 0);
}

#[test]
fn empty_data_emits_only_eof() {
    let hex = encode_ihex(&[], 0x0800_0000);
    assert_eq!(hex, ":00000001FF\n");
}
