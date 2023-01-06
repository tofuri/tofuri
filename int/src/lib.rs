use std::error::Error;
pub const BYTES: usize = 4;
pub fn to_be_bytes(u: u128) -> [u8; BYTES] {
    if u == 0 {
        return [0; BYTES];
    }
    let bytes = u.to_be_bytes();
    let mut i = 0;
    for byte in bytes {
        if byte != 0 {
            break;
        }
        i += 1;
    }
    let size = 15 - i;
    let mut output = [0; BYTES];
    for (j, v) in output.iter_mut().enumerate().take(BYTES) {
        let k = i + j;
        if k == 16 {
            break;
        }
        *v = bytes[k];
    }
    output[BYTES - 1] = (output[BYTES - 1] & 0xf0) | size as u8;
    output
}
pub fn from_be_bytes(b: &[u8; BYTES]) -> u128 {
    let size = b[BYTES - 1] as usize & 0x0f;
    let mut bytes = [0; 16];
    for (i, v) in b.iter().enumerate().take(BYTES) {
        let j = 15 - size + i;
        if j == 16 {
            break;
        }
        if i == BYTES - 1 {
            bytes[j] = v & 0xf0;
            break;
        }
        bytes[j] = *v;
    }
    u128::from_be_bytes(bytes)
}
pub fn floor(u: u128) -> u128 {
    from_be_bytes(&to_be_bytes(u))
}
pub fn to_string(u: u128, decimal_places: usize) -> String {
    let mut string = format!("{}{}", "0".repeat(decimal_places), u);
    string.insert(string.len() - decimal_places, '.');
    string = string.trim_start_matches('0').trim_end_matches('0').trim_end_matches('.').to_string();
    if string.starts_with('.') {
        let mut s = "0".to_string();
        s.push_str(&string);
        string = s;
    }
    if string == "" {
        string.push('0');
    }
    string
}
pub fn from_str(s: &str, decimal_places: usize) -> Result<u128, Box<dyn Error>> {
    let (mut string, diff) = match s.split_once(".") {
        Some((a, b)) => {
            let mut string = a.to_string();
            string.push_str(b);
            (string, decimal_places - b.len())
        }
        None => (s.to_string(), decimal_places),
    };
    string.push_str(&"0".repeat(diff));
    Ok(string.parse()?)
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
        assert_eq!(0x10000000000000000, from_be_bytes(&[1, 0, 0, 8]));
    }
    #[test]
    fn test_decode_max() {
        assert_eq!(0xfffffff0000000000000000000000000, from_be_bytes(&[0xff, 0xff, 0xff, 0xff]));
    }
    #[test]
    fn test_to_string() {
        assert_eq!("10.01", to_string(10_010_000_000_000_000_000, 18));
        assert_eq!("1", to_string(1_000_000_000_000_000_000, 18));
        assert_eq!("10", to_string(10_000_000_000_000_000_000, 18));
        assert_eq!("0.1", to_string(100_000_000_000_000_000, 18));
        assert_eq!("0", to_string(0, 18));
    }
    #[test]
    fn test_from_string() {
        assert_eq!(10_010_000_000_000_000_000, from_str("010.010", 18).unwrap());
        assert_eq!(1_000_000_000_000_000_000, from_str("1", 18).unwrap());
        assert_eq!(10_000_000_000_000_000_000, from_str("10", 18).unwrap());
        assert_eq!(10_000_000_000_000_000_000, from_str("10.", 18).unwrap());
        assert_eq!(10_000_000_000_000_000_000, from_str("10.0", 18).unwrap());
        assert_eq!(100_000_000_000_000_000, from_str(".1", 18).unwrap());
        assert_eq!(0, from_str("0", 18).unwrap());
    }
}
