//! `name=value&name=value` (HY-56).

use crate::{Fields, FormError};

/// Splits at every `&`, skips an empty part, separates the name at the first `=`, and decodes `+` to a space and
/// `%XX` with two hexadecimal digits to its byte; any other `%` stays. Names and values must be UTF-8.
pub fn parse(text: &[u8]) -> Result<Fields, FormError> {
    let mut fields = Fields::default();
    for part in text.split(|&byte| byte == b'&') {
        if part.is_empty() {
            continue;
        }
        let (name, value) = match part.iter().position(|&byte| byte == b'=') {
            Some(equals) => (&part[..equals], &part[equals + 1..]),
            None => (part, &part[part.len()..]),
        };
        fields.push(utf8(decode(name))?, utf8(decode(value))?);
    }
    Ok(fields)
}

fn decode(text: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut index = 0;
    while index < text.len() {
        match text[index] {
            b'+' => {
                bytes.push(b' ');
                index += 1;
            }
            b'%' => {
                match (hex(text.get(index + 1)), hex(text.get(index + 2))) {
                    (Some(high), Some(low)) => {
                        bytes.push(high << 4 | low);
                        index += 3;
                    }
                    _ => {
                        bytes.push(b'%');
                        index += 1;
                    }
                }
            }
            byte => {
                bytes.push(byte);
                index += 1;
            }
        }
    }
    bytes
}

fn hex(byte: Option<&u8>) -> Option<u8> {
    match byte? {
        digit @ b'0'..=b'9' => Some(digit - b'0'),
        letter @ b'a'..=b'f' => Some(letter - b'a' + 10),
        letter @ b'A'..=b'F' => Some(letter - b'A' + 10),
        _ => None,
    }
}

pub(crate) fn utf8(bytes: Vec<u8>) -> Result<String, FormError> {
    String::from_utf8(bytes).map_err(|_| FormError::Invalid)
}
