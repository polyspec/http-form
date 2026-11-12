//! What the runners of the contract cases share: the text of a case whose characters are bytes, the results as JSON,
//! and the category of each kind (plan F-3).

#![allow(dead_code, reason = "each test crate uses a part of the helpers")]

use std::fs;
use std::path::{Path, PathBuf};

use bindgen_core::Io;
use polyspec_http_forms::{Error, Fields, Files};
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
        "not_found" | "permission_denied" | "cross_device" | "storage_full" | "io" | "source_left"
        | "partial_target" => "io",
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
