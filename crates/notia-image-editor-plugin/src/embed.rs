//! Base64 helpers for the community plugin staging format (optional).

/// Decode standard base64 into bytes (no data-URL prefix).
pub fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim();
    let payload = input
        .strip_prefix("data:")
        .and_then(|rest| rest.split_once(',').map(|(_, b)| b))
        .unwrap_or(input);
    base64_decode(payload).ok_or_else(|| "invalid base64".into())
}

/// Encode bytes as standard base64.
pub fn encode_base64(bytes: &[u8]) -> String {
    base64_encode(bytes)
}

// Minimal base64 without extra deps (same alphabet as RFC 4648).
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(TABLE[(n >> 18) as usize & 63]);
        out.push(TABLE[(n >> 12) as usize & 63]);
        out.push(if chunk.len() > 1 {
            TABLE[(n >> 6) as usize & 63]
        } else {
            b'='
        });
        out.push(if chunk.len() > 2 {
            TABLE[n as usize & 63]
        } else {
            b'='
        });
    }
    String::from_utf8(out).expect("ascii")
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes: Vec<u8> = input
        .bytes()
        .filter(|&b| b != b'=' && !b.is_ascii_whitespace())
        .collect();
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        if chunk.len() < 2 {
            break;
        }
        let a = val(chunk[0])?;
        let b = val(chunk[1])?;
        let c = chunk.get(2).and_then(|&x| val(x));
        let d = chunk.get(3).and_then(|&x| val(x));
        let n = (a as u32) << 18
            | (b as u32) << 12
            | c.map(|x| x as u32).unwrap_or(0) << 6
            | d.map(|x| x as u32).unwrap_or(0);
        out.push((n >> 16) as u8);
        if c.is_some() {
            out.push((n >> 8) as u8);
        }
        if d.is_some() {
            out.push(n as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let raw = b"hello";
        assert_eq!(decode_base64(&encode_base64(raw)).unwrap(), raw);
    }
}
