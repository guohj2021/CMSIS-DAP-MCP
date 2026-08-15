//! Intel HEX encoding for memory export.

/// Encode `data` as an Intel HEX file starting at `start_address`.
///
/// Produces type-00 data records, type-04 extended linear address records
/// when the upper 16 address bits change (including the first record when
/// `start_address >= 0x10000`), and a type-01 EOF record.
pub fn encode_ihex(data: &[u8], start_address: u64) -> String {
    const BYTES_PER_RECORD: usize = 16;
    let mut out = String::new();
    let mut current_upper: Option<u16> = None;
    let mut address = start_address;

    for chunk in data.chunks(BYTES_PER_RECORD) {
        let upper = ((address >> 16) & 0xFFFF) as u16;
        if upper != 0 && current_upper != Some(upper) {
            current_upper = Some(upper);
            out.push_str(&record(0x04, [0x00, 0x00], &upper.to_be_bytes()));
        }
        let lower = (address & 0xFFFF) as u16;
        out.push_str(&record(0x00, lower.to_be_bytes(), chunk));
        address += chunk.len() as u64;
    }

    out.push_str(":00000001FF\n");
    out
}

fn record(record_type: u8, address: [u8; 2], data: &[u8]) -> String {
    let mut body = Vec::with_capacity(5 + data.len());
    body.push(data.len() as u8);
    body.extend_from_slice(&address);
    body.push(record_type);
    body.extend_from_slice(data);
    let checksum = (0u32.wrapping_sub(body.iter().map(|b| *b as u32).sum::<u32>()) & 0xFF) as u8;

    let mut line = String::from(":");
    for b in body {
        line.push_str(&format!("{b:02X}"));
    }
    line.push_str(&format!("{checksum:02X}"));
    line.push('\n');
    line
}

#[cfg(test)]
mod tests {
    use super::encode_ihex;

    #[test]
    fn low_address_has_no_ela() {
        let hex = encode_ihex(&[0x01, 0x02], 0x0000_0000);
        assert_eq!(hex, ":020000000102FB\n:00000001FF\n");
    }
}
