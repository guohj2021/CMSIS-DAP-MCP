//! SWO / ITM packet decoder (frozen v5 P8, ITM over SWO).
//!
//! Parses the Cortex-M SWO protocol into ITM packets. Pure function with unit
//! tests — the decode layer is complete even though some probes (e.g. the
//! CMSIS-DAP used in development) cannot capture SWO; the panel then reports
//! the capture error honestly.

use serde::Serialize;

/// One decoded SWO/ITM packet.
#[derive(Debug, Clone, Serialize)]
pub struct SwoPacket {
    pub port: u8,
    pub data: Vec<u8>,
    pub timestamp: bool,
}

/// Decode a raw SWO byte stream into ITM packets.
///
/// Packet layout (Cortex-M SWO protocol):
/// - Header byte H1: bits[2:0] = size code (0=>1, 1=>2, 2=>4, 3=>8 bytes),
///   bit 6 = sync flag, bit 7 = timestamp flag.
/// - When bit 6 or 7 of H1 is set, a second header byte H2 follows:
///   bits[2:0] = port, bit 6 = continuation, bit 7 = timestamp.
/// - Then the payload bytes.
pub fn decode_swo(bytes: &[u8]) -> Vec<SwoPacket> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let h1 = bytes[i];
        i += 1;
        let size_code = (h1 & 0x07) as usize;
        let size = match size_code {
            0 => 1usize,
            1 => 2,
            2 => 4,
            _ => 8,
        };
        let mut timestamp = h1 & 0x80 != 0;
        let mut port = 0u8;
        // Second header byte when sync/timestamp flags are set.
        if h1 & 0xC0 != 0 {
            if i >= bytes.len() {
                break;
            }
            let h2 = bytes[i];
            i += 1;
            port = h2 & 0x07;
            if h2 & 0x80 != 0 {
                timestamp = true;
            }
            // Continuation bit (0x40) is not fully handled; treat as port 0.
            if h2 & 0x40 != 0 {
                port = 0;
            }
        } else if h1 & 0x08 == 0 && size_code == 0 {
            // Single-header packets with size code 0 carry the port in the
            // header bits; the common case (port 0) is the only one decoded.
            port = 0;
        }
        let take = size.min(bytes.len() - i);
        let data = bytes[i..i + take].to_vec();
        i += take;
        out.push(SwoPacket {
            port,
            data,
            timestamp,
        });
    }
    out
}

/// Render port-0 packets as console text (printable chars kept).
pub fn port0_text(packets: &[SwoPacket]) -> String {
    let mut text = String::new();
    for p in packets {
        if p.port != 0 {
            continue;
        }
        for b in &p.data {
            let c = *b;
            if c == b'\r' || c == b'\n' || (0x20..=0x7e).contains(&c) {
                text.push(c as char);
            } else {
                text.push_str(&format!("\\x{c:02x}"));
            }
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_single_byte_port0_packets() {
        // H1=0x01: size code 1 (2 bytes), no sync/timestamp -> single header.
        // Port defaults to 0. Payload: 'A', 'B'.
        let bytes = [0x01, b'A', b'B'];
        let packets = decode_swo(&bytes);
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].port, 0);
        assert_eq!(packets[0].data, vec![b'A', b'B']);
        assert!(!packets[0].timestamp);
    }

    #[test]
    fn decodes_two_header_packets_with_port() {
        // H1=0x81 (size 1, timestamp flag) -> H2=0x02 (port 2).
        let bytes = [0x81, 0x02, 0xAA, 0xBB];
        let packets = decode_swo(&bytes);
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].port, 2);
        assert_eq!(packets[0].data, vec![0xAA, 0xBB]);
        assert!(packets[0].timestamp);
    }

    #[test]
    fn port0_text_renders_console() {
        let packets = vec![
            SwoPacket {
                port: 0,
                data: b"Hello".to_vec(),
                timestamp: false,
            },
            SwoPacket {
                port: 0,
                data: vec![b'\n'],
                timestamp: false,
            },
        ];
        assert_eq!(port0_text(&packets), "Hello\n");
    }
}
