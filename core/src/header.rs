//! Content types, header lines and header parameters (HY-57.6, HY-113.11 to HY-113.14).
//!
//! Every function reads bytes. A blank is a space or a tab; no other byte, such as a no-break space, separates
//! anything, and a name is compared without case for ASCII letters only.

/// One parameter of a header value: its name in lower case and its value.
pub type Parameters = Vec<(Vec<u8>, Vec<u8>)>;

fn blank(byte: u8) -> bool {
    byte == b' ' || byte == b'\t'
}

/// Removes the spaces and tabs at both ends.
pub fn trim_blanks(bytes: &[u8]) -> &[u8] {
    let start = bytes.iter().position(|&byte| !blank(byte)).unwrap_or(bytes.len());
    let end = bytes.iter().rposition(|&byte| !blank(byte)).map_or(start, |last| last + 1);
    &bytes[start..end]
}

fn find(bytes: &[u8], from: usize, needle: u8) -> Option<usize> {
    bytes.get(from..)?.iter().position(|&byte| byte == needle).map(|offset| from + offset)
}

/// True when the content type starts with `application/x-www-form-urlencoded`, compared without case, followed by
/// spaces or tabs and then `;` or the end (HY-57.6).
pub fn is_urlencoded(content_type: &[u8]) -> bool {
    const TYPE: &[u8] = b"application/x-www-form-urlencoded";
    if content_type.len() < TYPE.len() || !content_type[..TYPE.len()].eq_ignore_ascii_case(TYPE) {
        return false;
    }
    let rest = &content_type[TYPE.len()..];
    let after = rest.iter().position(|&byte| !blank(byte)).unwrap_or(rest.len());
    after == rest.len() || rest[after] == b';'
}

/// Returns the text before the first `;` without the blanks around it, in lower case, and the parameters after it in
/// their order: each follows a `;` and blanks, its name runs up to its `=`, and its value is a quoted string without
/// its quotes, or the rest when the quote does not close, or the text up to the next `;`, space or tab (HY-113.13).
pub fn parameters(text: &[u8]) -> (Vec<u8>, Parameters) {
    let length = text.len();
    let mut index = find(text, 0, b';');
    let kind = trim_blanks(&text[..index.unwrap_or(length)]).to_ascii_lowercase();
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
        if at >= length || text[at] != b'=' {
            index = (at < length).then_some(at);
            continue;
        }
        let name = text[start..at].to_ascii_lowercase();
        at += 1;
        let value;
        if at < length && text[at] == b'"' {
            match find(text, at + 1, b'"') {
                Some(close) => {
                    value = text[at + 1..close].to_vec();
                    at = close + 1;
                }
                None => {
                    value = text[at + 1..].to_vec();
                    at = length;
                }
            }
        } else {
            let start = at;
            while at < length && text[at] != b';' && !blank(text[at]) {
                at += 1;
            }
            value = text[start..at].to_vec();
        }
        if !found.iter().any(|(known, _)| *known == name) {
            found.push((name, value));
        }
        index = find(text, at, b';');
    }
    (kind, found)
}

/// The value of the first parameter of a name.
pub fn parameter<'a>(parameters: &'a Parameters, name: &[u8]) -> Option<&'a [u8]> {
    parameters.iter().find(|(known, _)| known == name).map(|(_, value)| value.as_slice())
}

/// Returns the boundary of a content type whose type is `multipart/form-data`, or `None` for another type or a
/// missing or empty boundary (HY-113.11).
pub fn boundary(content_type: &[u8]) -> Option<Vec<u8>> {
    let (kind, found) = parameters(content_type);
    if kind != b"multipart/form-data" {
        return None;
    }
    parameter(&found, b"boundary").filter(|value| !value.is_empty()).map(<[u8]>::to_vec)
}

/// Returns the value of the first header line of the name, compared without case, without the blanks around it
/// (HY-113.12).
pub fn header_value<'a>(lines: &[&'a [u8]], name: &[u8]) -> Option<&'a [u8]> {
    lines.iter().find_map(|line| {
        let colon = line.iter().position(|&byte| byte == b':')?;
        line[..colon].eq_ignore_ascii_case(name).then(|| trim_blanks(&line[colon + 1..]))
    })
}
