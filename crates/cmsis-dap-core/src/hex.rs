//! Intel HEX encoding and parsing.

/// One contiguous address range of an Intel HEX image.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct HexSegment {
    pub start: u64,
    pub end: u64,
    pub data: Vec<u8>,
}

impl HexSegment {
    pub fn size(&self) -> u64 {
        self.end - self.start
    }
}

/// Parse an Intel HEX file (text) into contiguous segments.
///
/// Supports type 00 (data), 01 (EOF), 02 (extended segment), and 04
/// (extended linear address) records. Returns an error on malformed lines or
/// checksum mismatches. Empty images return an empty vector.
pub fn parse_ihex(text: &str) -> Result<Vec<HexSegment>, String> {
    let mut segments: Vec<HexSegment> = Vec::new();
    let mut base: u64 = 0;
    let mut eof = false;

    for (idx, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with(':') {
            return Err(format!("line {}: not an Intel HEX record: {}", idx + 1, line));
        }
        let bytes = decode_record(line, idx + 1)?;
        if bytes.len() < 5 {
            return Err(format!("line {}: record too short", idx + 1));
        }
        let byte_count = bytes[0] as usize;
        if bytes.len() != byte_count + 5 {
            return Err(format!(
                "line {}: record length mismatch (declared {byte_count}, got {})",
                idx + 1,
                bytes.len() - 5
            ));
        }
        let address16 = ((bytes[1] as u16) << 8) | bytes[2] as u16;
        let record_type = bytes[3];
        let data = &bytes[4..4 + byte_count];

        match record_type {
            0x00 => {
                let address = base + address16 as u64;
                append_segment(&mut segments, address, data);
            }
            0x01 => {
                eof = true;
            }
            0x02 => {
                if data.len() != 2 {
                    return Err(format!("line {}: invalid extended segment record", idx + 1));
                }
                base = ((data[0] as u64) << 8 | data[1] as u64) * 16;
            }
            0x04 => {
                if data.len() != 2 {
                    return Err(format!("line {}: invalid extended linear address record", idx + 1));
                }
                base = ((data[0] as u64) << 8 | data[1] as u64) << 16;
            }
            _ => {
                // 0x03 / 0x05 (start address) carry no data; ignore.
            }
        }
    }

    if !eof {
        return Err("missing EOF record".into());
    }
    Ok(segments)
}

fn decode_record(line: &str, line_no: usize) -> Result<Vec<u8>, String> {
    let hex = &line[1..];
    if !hex.len().is_multiple_of(2) {
        return Err(format!("line {line_no}: odd hex length"));
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for i in (0..hex.len()).step_by(2) {
        let b = u8::from_str_radix(&hex[i..i + 2], 16)
            .map_err(|_| format!("line {line_no}: invalid hex digit"))?;
        bytes.push(b);
    }
    let checksum: u32 = bytes.iter().map(|b| *b as u32).sum();
    if (checksum & 0xFF) != 0 {
        return Err(format!("line {line_no}: checksum mismatch"));
    }
    Ok(bytes)
}

fn append_segment(segments: &mut Vec<HexSegment>, address: u64, data: &[u8]) {
    if data.is_empty() {
        return;
    }
    if let Some(last) = segments.last_mut() {
        if last.end == address {
            last.data.extend_from_slice(data);
            last.end = address + data.len() as u64;
            return;
        }
    }
    segments.push(HexSegment {
        start: address,
        end: address + data.len() as u64,
        data: data.to_vec(),
    });
}

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

    use super::parse_ihex;

    #[test]
    fn parse_roundtrip_single_segment() {
        let hex = encode_ihex(&[0x01, 0x02, 0x03], 0x0800_0000);
        let segs = parse_ihex(&hex).unwrap();
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].start, 0x0800_0000);
        assert_eq!(segs[0].end, 0x0800_0003);
        assert_eq!(segs[0].data, vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn parse_merges_contiguous_and_rejects_bad_checksum() {
        let hex = ":020000040800F2\n:040000001122334452\n:00000001FF\n";
        let segs = parse_ihex(hex).unwrap();
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].start, 0x0800_0000);
        assert_eq!(segs[0].data, vec![0x11, 0x22, 0x33, 0x44]);

        let bad = ":040000001122334453\n:00000001FF\n";
        assert!(parse_ihex(bad).is_err());
    }

    #[test]
    fn low_address_has_no_ela() {
        let hex = encode_ihex(&[0x01, 0x02], 0x0000_0000);
        assert_eq!(hex, ":020000000102FB\n:00000001FF\n");
    }
}
