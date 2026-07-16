pub(super) fn hex_val(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

pub(super) fn parse_decimal_digits(text: &[u8]) -> Result<u128, String> {
    let mut v: u128 = 0;
    for &c in text {
        if c == b'\'' {
            continue;
        }
        if !c.is_ascii_digit() {
            return Err("invalid decimal digit".into());
        }
        v = v
            .checked_mul(10)
            .and_then(|x| x.checked_add((c - b'0') as u128))
            .ok_or_else(|| "integer literal overflow".to_string())?;
    }
    Ok(v)
}

pub(super) fn parse_hex_digits(text: &[u8]) -> Result<u128, String> {
    let mut v: u128 = 0;
    for &c in text {
        if c == b'\'' {
            continue;
        }
        v = v
            .checked_mul(16)
            .and_then(|x| x.checked_add(hex_val(c) as u128))
            .ok_or_else(|| "integer literal overflow".to_string())?;
    }
    Ok(v)
}

pub(super) fn parse_binary_digits(text: &[u8]) -> Result<u128, String> {
    let mut v: u128 = 0;
    for &c in text {
        if c == b'\'' {
            continue;
        }
        v = v
            .checked_mul(2)
            .and_then(|x| x.checked_add((c - b'0') as u128))
            .ok_or_else(|| "integer literal overflow".to_string())?;
    }
    Ok(v)
}

pub(super) fn parse_octal_digits(text: &[u8]) -> Result<u128, String> {
    let mut v: u128 = 0;
    for &c in text {
        if c == b'\'' {
            continue;
        }
        if !(b'0'..=b'7').contains(&c) {
            return Err("invalid octal digit".into());
        }
        v = v
            .checked_mul(8)
            .and_then(|x| x.checked_add((c - b'0') as u128))
            .ok_or_else(|| "integer literal overflow".to_string())?;
    }
    Ok(v)
}
