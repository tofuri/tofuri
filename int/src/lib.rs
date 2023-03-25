use std::num::ParseIntError;
use tofuri_core::*;
#[derive(Debug)]
pub enum Error {
    FromStr(ParseIntError),
}
pub fn to_be_bytes(uint: u128) -> AmountBytes {
    if uint == 0 {
        return [0; AMOUNT_BYTES];
    }
    let bytes = uint.to_be_bytes();
    let mut i = 0;
    for byte in bytes {
        if byte != 0 {
            break;
        }
        i += 1;
    }
    let size = 15 - i;
    let mut output = [0; AMOUNT_BYTES];
    for (j, v) in output.iter_mut().enumerate().take(AMOUNT_BYTES) {
        let k = i + j;
        if k == 16 {
            break;
        }
        *v = bytes[k];
    }
    output[AMOUNT_BYTES - 1] = (output[AMOUNT_BYTES - 1] & 0xf0) | size as u8;
    output
}
pub fn from_be_slice(slice: &[u8; AMOUNT_BYTES]) -> u128 {
    let size = slice[AMOUNT_BYTES - 1] as usize & 0x0f;
    let mut bytes = [0; 16];
    for (i, v) in slice.iter().enumerate().take(AMOUNT_BYTES) {
        let j = 15 - size + i;
        if j == 16 {
            break;
        }
        if i == AMOUNT_BYTES - 1 {
            bytes[j] = v & 0xf0;
            break;
        }
        bytes[j] = *v;
    }
    u128::from_be_bytes(bytes)
}
pub fn floor(uint: u128) -> u128 {
    from_be_slice(&to_be_bytes(uint))
}
pub fn to_string(uint: u128) -> String {
    let mut string = format!("{}{}", "0".repeat(DECIMAL_PLACES), uint);
    string.insert(string.len() - DECIMAL_PLACES, '.');
    string = string
        .trim_start_matches('0')
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string();
    if string.starts_with('.') {
        let mut s = "0".to_string();
        s.push_str(&string);
        string = s;
    }
    if string.is_empty() {
        string.push('0');
    }
    string
}
pub fn from_str(str: &str) -> Result<u128, Error> {
    let (mut string, diff) = match str.split_once('.') {
        Some((a, b)) => {
            let mut string = a.to_string();
            string.push_str(b);
            (string, DECIMAL_PLACES - b.len())
        }
        None => (str.to_string(), DECIMAL_PLACES),
    };
    string.push_str(&"0".repeat(diff));
    string.parse().map_err(Error::FromStr)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_encode() {
        assert_eq!([1, 0, 0, 8], to_be_bytes(0x10000000000000000));
    }
    #[test]
    fn test_decode() {
        assert_eq!(0x10000000000000000, from_be_slice(&[1, 0, 0, 8]));
    }
    #[test]
    fn test_decode_max() {
        assert_eq!(
            0xfffffff0000000000000000000000000,
            from_be_slice(&[0xff, 0xff, 0xff, 0xff])
        );
    }
    #[test]
    fn test_to_string() {
        assert_eq!("10.01", to_string(10_010_000_000_000_000_000));
        assert_eq!("1", to_string(1_000_000_000_000_000_000));
        assert_eq!("10", to_string(10_000_000_000_000_000_000));
        assert_eq!("0.1", to_string(100_000_000_000_000_000));
        assert_eq!("0", to_string(0));
    }
    #[test]
    fn test_from_string() {
        assert_eq!(10_010_000_000_000_000_000, from_str("010.010").unwrap());
        assert_eq!(1_000_000_000_000_000_000, from_str("1").unwrap());
        assert_eq!(10_000_000_000_000_000_000, from_str("10").unwrap());
        assert_eq!(10_000_000_000_000_000_000, from_str("10.").unwrap());
        assert_eq!(10_000_000_000_000_000_000, from_str("10.0").unwrap());
        assert_eq!(100_000_000_000_000_000, from_str(".1").unwrap());
        assert_eq!(0, from_str("0").unwrap());
    }
}
