//! Header lines and the parameters of a `Content-Disposition` (HY-113.12 to HY-113.14, spec F-6, F-16). A content type
//! is a media type, which `content.rs` reads with `polyspec-http-media-type` (spec F-12).
//!
//! Every function reads bytes. A blank is a space or a tab; no other byte, such as a no-break space, separates
//! anything, and a name is compared without case for ASCII letters only.

use crate::Error;

/// One parameter of a header value: its name in lower case and its value.
pub type Parameters = Vec<(Vec<u8>, Vec<u8>)>;

fn blank(byte: u8) -> bool {
    byte == b' ' || byte == b'\t'
}

/// Removes the spaces and tabs at both ends.
pub fn trim_blanks(bytes: &[u8]) -> &[u8] {
    let Some(start) = bytes.iter().position(|&byte| !blank(byte)) else {
        return &bytes[bytes.len()..];
    };
    match bytes.iter().rposition(|&byte| !blank(byte)) {
        Some(last) => &bytes[start..=last],
        None => &bytes[bytes.len()..],
    }
}

fn find(bytes: &[u8], from: usize, needle: u8) -> Option<usize> {
    bytes.get(from..)?.iter().position(|&byte| byte == needle).map(|offset| from + offset)
}

/// The text before the first `;`, or the whole text.
fn head(text: &[u8]) -> &[u8] {
    match find(text, 0, b';') {
        Some(semicolon) => &text[..semicolon],
        None => text,
    }
}

/// The text before the first `;` without the spaces and tabs at both ends, with the ASCII letters in lower case: the
/// type of a `Content-Disposition` (HY-113.13).
fn disposition_type(value: &[u8]) -> Vec<u8> {
    trim_blanks(head(value)).to_ascii_lowercase()
}

/// Returns the parameters of a header value after the first `;` in their order: each follows a `;` and blanks, its name runs up to its
/// `=`, and its value is a quoted string without its quotes or the text up to the next `;`, space or tab
/// (HY-113.13, HY-113.14). A text before a `;` without a `=` is no parameter. A quoted value without its end (F-6) and
/// a parameter whose name, compared without case, a parameter before it has (F-16) fail with `None`.
fn parameters(text: &[u8]) -> Option<Parameters> {
    let length = text.len();
    let mut index = find(text, 0, b';');
    let mut found: Parameters = Vec::new();
    while let Some(semicolon) = index {
        let mut at = semicolon + 1;
        while at < length && blank(text[at]) {
            at += 1;
        }
        let start = at;
        while at < length && text[at] != b'=' && text[at] != b';' {
            at += 1;
        }
        if at >= length {
            break;
        }
        if text[at] != b'=' {
            index = Some(at);
            continue;
        }
        let name = text[start..at].to_ascii_lowercase();
        at += 1;
        let value;
        if at < length && text[at] == b'"' {
            let close = find(text, at + 1, b'"')?;
            value = text[at + 1..close].to_vec();
            at = close + 1;
        } else {
            let start = at;
            while at < length && text[at] != b';' && !blank(text[at]) {
                at += 1;
            }
            value = text[start..at].to_vec();
        }
        // F-16: a parameter given twice makes the value invalid (RFC 6266, section 4.1).
        if found.iter().any(|(known, _)| *known == name) {
            return None;
        }
        found.push((name, value));
        index = find(text, at, b';');
    }
    Some(found)
}

/// The value of the parameter of a name.
pub fn parameter<'a>(parameters: &'a Parameters, name: &[u8]) -> Option<&'a [u8]> {
    parameters.iter().find(|(known, _)| known == name).map(|(_, value)| value.as_slice())
}

/// The `name` and the parameters of the `Content-Disposition` of a part (HY-113.13, F-16): its type is `form-data`
/// and it has a `name` parameter, or the part fails with `Error::Invalid` (RFC 7578, section 4.2); a quoted value
/// without its end (F-6) and a parameter given twice (F-16) fail likewise.
pub fn form_data(value: &[u8]) -> Result<(Vec<u8>, Parameters), Error> {
    let found = parameters(value).ok_or(Error::Invalid)?;
    if disposition_type(value) != b"form-data" {
        return Err(Error::Invalid);
    }
    match parameter(&found, b"name") {
        Some(name) => Ok((name.to_vec(), found)),
        None => Err(Error::Invalid),
    }
}

/// Returns the value of the first header line of the name, compared without case, without the blanks around it
/// (HY-113.12). A line without `:` and a later line of the same name are ignored inputs (spec I-5).
pub fn header_value<'a>(lines: &[&'a [u8]], name: &[u8]) -> Option<&'a [u8]> {
    lines.iter().find_map(|line| {
        let colon = line.iter().position(|&byte| byte == b':')?;
        line[..colon].eq_ignore_ascii_case(name).then(|| trim_blanks(&line[colon + 1..]))
    })
}
