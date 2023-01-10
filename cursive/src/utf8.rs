use std::char::from_u32;

/// Reads a potentially multi-bytes utf8 codepoint.
///
/// Reads the given first byte, and uses the given
/// function to get more if needed.
///
/// Returns an error if the stream is invalid utf-8.
#[allow(dead_code)]
pub fn read_char<F>(first: u8, next: F) -> Result<char, String>
where
    F: Fn() -> Option<u8>,
{
    if first < 0x80 {
        return Ok(first as char);
    }

    // Number of leading 1s determines the number of bytes we'll have to read
    let n_bytes = match (!first).leading_zeros() {
        n @ 2..=6 => n as usize,
        1 => return Err("First byte is continuation byte.".to_string()),
        7..=8 => return Err("WTF is this byte??".to_string()),
        _ => unreachable!(),
    };

    let mut res = 0_u32;

    // First, get the data - only the few last bits
    res |= u32::from(first & make_mask(7 - n_bytes));

    // We already have one byte, now read the others.
    for _ in 1..n_bytes {
        let byte = next().ok_or_else(|| "Missing UTF-8 byte".to_string())?;
        if byte & 0xC0 != 0x80 {
            return Err(format!(
                "Found non-continuation byte after leading: `{byte}`",
            ));
        }
        // We have 6 fresh new bits to read, make room.
        res <<= 6;
        // 0x3F is 00111111, so we keep the last 6 bits
        res |= u32::from(byte & 0x3F);
    }

    // from_u32 could return an error if we gave it invalid utf-8.
    // But we're probably safe since we respected the rules when building it.
    Ok(from_u32(res).unwrap())
}

// Returns a simple bitmask with n 1s to the right.
#[allow(dead_code)]
fn make_mask(n: usize) -> u8 {
    let mut r = 0_u8;
    for i in 0..n {
        r |= 1 << i;
    }
    r
}
