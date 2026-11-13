//! What the runners of the contract cases share: the text of a case whose characters are bytes, the results as JSON,
//! and the category of each kind (plan F-3).

#![allow(dead_code, reason = "each test crate uses a part of the helpers")]

use std::fs;
use std::path::{Path, PathBuf};

use binding_core::fault::{self, Armed};
use binding_core::Io;
use polyspec_http_form::{Error, Fields, Files};
use serde_json::{json, Value};

/// The repository root.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The wait token of one call that does blocking I/O (spec F-17), as generated code makes it.
pub fn io() -> Io {
    Io::for_generated_call()
}

/// The JSON of a contract file.
pub fn contract(name: &str) -> Value {
    let path = root().join("contracts").join(name);
    let text = fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The cases of a group.
pub fn group<'a>(cases: &'a Value, name: &str) -> &'a [Value] {
    match cases.get(name).and_then(Value::as_array) {
        Some(list) => list,
        None => panic!("the contract has no group {name}"),
    }
}

/// A text of a case whose characters U+0000 to U+00FF are bytes (F-4).
pub fn latin1(value: &Value) -> Vec<u8> {
    let Some(text) = value.as_str() else { panic!("{value} is no text") };
    text.chars().map(|c| u8::try_from(u32::from(c)).unwrap_or_else(|_| panic!("{c:?} is no byte"))).collect()
}

/// Bytes of a case in the case convention of binding (G-5): `{"hex": ...}`.
pub fn bytes(value: &Value) -> Vec<u8> {
    let Some(hex) = value.get("hex").and_then(Value::as_str) else { panic!("{value} is no {{\"hex\"}}") };
    (0..hex.len())
        .step_by(2)
        .map(|at| match hex.get(at..at + 2).map(|pair| u8::from_str_radix(pair, 16)) {
            Some(Ok(byte)) => byte,
            _ => panic!("{hex} is no hex"),
        })
        .collect()
}

/// The content type of a case, `null` for none (an `optional<latin1>` of G-5).
pub fn content_type(case: &Value) -> Option<Vec<u8>> {
    match case.get("content_type") {
        Some(Value::Null) => None,
        Some(value) => Some(latin1(value)),
        None => panic!("{}: no content_type", id(case)),
    }
}

/// The record `limits` of a case (T-4).
pub fn limits(case: &Value) -> polyspec_http_form::BodyLimits {
    let limits = &case["limits"];
    polyspec_http_form::BodyLimits {
        limit: number(limits, "limit"),
        value_limit: number(limits, "value_limit"),
        part_limit: number(limits, "part_limit"),
        file_limit: number(limits, "file_limit"),
    }
}

/// Bytes as a text whose characters are the bytes.
pub fn text(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}

/// A text field of a case.
pub fn string<'a>(case: &'a Value, key: &str) -> &'a str {
    match case.get(key).and_then(Value::as_str) {
        Some(text) => text,
        None => panic!("{}: no text {key}", id(case)),
    }
}

/// A number field of a case.
pub fn number(case: &Value, key: &str) -> u64 {
    match case.get(key).and_then(Value::as_u64) {
        Some(number) => number,
        None => panic!("{}: no number {key}", id(case)),
    }
}

/// The id of a case, or the name of an entry.
pub fn id(case: &Value) -> &str {
    match case.get("id").or_else(|| case.get("file")).and_then(Value::as_str) {
        Some(id) => id,
        None => panic!("a case without an id: {case}"),
    }
}

/// The category of a kind (plan F-3; the table of docs/spec/forms.md, Errors).
pub fn category(kind: &str) -> &'static str {
    match kind {
        "invalid"
        | "header_limit"
        | "no_boundary"
        | "malformed_content_type"
        | "invalid_content_length"
        | "body_limit"
        | "value_limit"
        | "part_limit"
        | "file_limit"
        | "moved"
        | "exists"
        | "removed"
        | "invalid_number" => "value",
        "not_finished" | "not_streamed" | "finished" => "type",
        "not_found" | "permission_denied" | "storage_full" | "io" | "source_left" | "partial_target" => "io",
        "internal" => "internal",
        other => panic!("a kind without a category: {other}"),
    }
}

/// An error as the `raises` of a case (plan F-3). No rule of this crate fixes a message, so it is null.
pub fn raises(error: &Error) -> Value {
    json!({
        "kind": error.kind(),
        "status": error.status(),
        "category": category(error.kind()),
        "message": Value::Null,
    })
}

/// The errnos that the cases name, by their symbol (G-5: the `errno` of `raises` is a symbol).
const ERRNOS: [(&str, rustix::io::Errno); 7] = [
    ("EACCES", rustix::io::Errno::ACCESS),
    ("EEXIST", rustix::io::Errno::EXIST),
    ("EINVAL", rustix::io::Errno::INVAL),
    ("EIO", rustix::io::Errno::IO),
    ("ENOENT", rustix::io::Errno::NOENT),
    ("ENOSPC", rustix::io::Errno::NOSPC),
    ("EXDEV", rustix::io::Errno::XDEV),
];

/// The symbol of an errno, `null` for none.
pub fn errno_symbol(errno: Option<i32>) -> Value {
    match errno {
        None => Value::Null,
        Some(errno) => match ERRNOS.iter().find(|(_, known)| known.raw_os_error() == errno) {
            Some((symbol, _)) => json!(symbol),
            None => panic!("the errno {errno} has no symbol of the cases"),
        },
    }
}

/// An error as the `raises` of the case: [`raises`], and the symbol of its `errno` when the case names one (G-5).
pub fn raised(case: &Value, error: &Error) -> Value {
    let mut value = raises(error);
    if case["raises"].get("errno").is_some() {
        value["errno"] = errno_symbol(error.errno());
    }
    value
}

/// The faults that a case arms for one call of a member (G-5): the call runs with them installed, and every armed
/// fault must fire. A case without `faults` arms nothing.
pub struct Arming {
    id: Option<u64>,
    armed: Armed,
    label: String,
}

impl Arming {
    /// Arms the `faults` of the case for `member`, as the generated tests do.
    pub fn of(case: &Value, member: &str) -> Arming {
        let label = id(case).to_owned();
        match case.get("faults") {
            None => Arming { id: None, armed: Armed::empty(), label },
            Some(faults) => match fault::arm(member, &faults.to_string()) {
                Ok(armed) => Arming { id: Some(armed), armed: fault::take(member), label },
                Err(error) => panic!("{label}: {error}"),
            },
        }
    }

    /// Runs the call with the faults installed.
    pub fn run<T>(&self, work: impl FnOnce() -> T) -> T {
        self.armed.run(work)
    }

    /// Fails when a fault that the case armed did not fire.
    pub fn done(self) {
        if let Some(armed) = self.id {
            match fault::left(armed) {
                Ok(left) => assert_eq!(left, Vec::<String>::new(), "{}: every armed fault fires", self.label),
                Err(error) => panic!("{}: {error}", self.label),
            }
        }
    }
}

/// Fields as pairs of a name and its values.
pub fn fields_json(fields: &Fields) -> Value {
    Value::Array(fields.iter().map(|(name, values)| json!([name, values])).collect())
}

/// Files in request order, with their size when `size` is true.
pub fn files_json(files: &Files, size: bool) -> Value {
    let mut list = Vec::new();
    for (_, items) in files {
        for item in items {
            let content = match item.content(io()) {
                Ok(bytes) => text(&bytes),
                Err(error) => panic!("the content of {}: {error}", item.name()),
            };
            let mut file = json!({
                "name": item.name(),
                "filename": item.filename(),
                "contentType": item.content_type(),
                "content": content,
            });
            if size {
                file["size"] = json!(item.size());
            }
            list.push(file);
        }
    }
    Value::Array(list)
}

/// Collects the differences of the cases of a group and fails with all of them.
#[derive(Default)]
pub struct Failures {
    list: Vec<String>,
    compared: usize,
}

impl Failures {
    /// Compares a key of a case with the result; a key that the case lacks is no expectation.
    pub fn compare(&mut self, case: &Value, key: &str, actual: &Value) {
        if let Some(expected) = case.get(key) {
            self.compared += 1;
            if expected != actual {
                self.list.push(format!("{}: {key}: expected {expected}, got {actual}", id(case)));
            }
        }
    }

    /// Records a difference.
    pub fn push(&mut self, failure: String) {
        self.list.push(failure);
    }

    /// Fails with every difference, and when nothing was compared.
    pub fn check(self, group: &str) {
        assert!(self.list.is_empty(), "{group}: {} failures:\n{}", self.list.len(), self.list.join("\n"));
        assert!(self.compared > 0, "{group}: nothing was compared");
    }
}
