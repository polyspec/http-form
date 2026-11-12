//! Runs the cases of `contracts/fields.json`, the copy of hyper's file (V-1, `contracts/sources.json`), as hyper's
//! runners do: a urlencoded text is the UTF-8 encoding of its JSON string, and a content type and a body are texts
//! whose characters are their bytes.
//!
//! A case whose rule this library changed is listed in the group `superseded` of `contracts/forms.json` with the rule
//! and the case of this library that replaces it; the runner checks that the replacing case reads the same input and
//! runs that case in `forms.rs`. A case that only a binding can run, because its input is no value of the core's
//! types, is listed in the group `binding` with its rule.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use common::{
    contract, fields_json, files_json, group, id, latin1, number, string, text, Failures,
};
use polyspec_http_forms::{
    fields_from_body, files_from_body, multipart_boundary, parse_urlencoded, Error, Form, Limits, MultipartReader,
    ReadError, RequestBody, UploadedFile, HEADER_LIMIT,
};
use serde_json::{json, Value};

/// The ids of the cases of a group of fields.json that another rule replaces or that a binding runs.
fn listed(kind: &str, source_group: &str) -> BTreeMap<String, Value> {
    let forms = contract("forms.json");
    group(&forms, kind)
        .iter()
        .filter(|entry| entry["source"] == json!("fields.json") && entry["group"] == json!(source_group))
        .map(|entry| (string(entry, "id").to_owned(), entry.clone()))
        .collect()
}

/// The superseded cases of a group, which must name an existing case of forms.json with the same input.
fn superseded(source_group: &str, cases: &[Value]) -> BTreeMap<String, Value> {
    let forms = contract("forms.json");
    let list = listed("superseded", source_group);
    for (key, entry) in &list {
        let Some(case) = cases.iter().find(|case| id(case) == key) else {
            panic!("superseded {key} is no case of fields.json/{source_group}");
        };
        let target = string(entry, "replacement");
        let Some((target_group, target_id)) = target.split_once('/') else { panic!("{target}: group/id") };
        let Some(replacement) = group(&forms, target_group).iter().find(|other| id(other) == target_id) else {
            panic!("{key}: the replacement {target} is no case of forms.json");
        };
        for input in ["type", "body", "text"] {
            assert_eq!(case.get(input), replacement.get(input), "{key}: the replacement {target} reads another {input}");
        }
        assert!(!string(entry, "rule").is_empty(), "{key}: no rule");
    }
    list
}

fn outcome(form: Result<Form, Error>) -> (Value, Value) {
    match form {
        Ok(form) => (fields_json(&form.fields), files_json(&form.files, false)),
        Err(_) => (Value::Null, Value::Null),
    }
}

/// Reads a body through a reader in the given chunks, collecting its pieces.
fn read_chunks(boundary: &[u8], chunks: &[&[u8]], bound: usize, label: &str) -> Result<Form, Error> {
    let mut reader = MultipartReader::new(boundary, Limits::default());
    let mut form = Form::default();
    let mut failure = None;
    for chunk in chunks {
        let result = reader.feed_with(chunk, |piece| form.take(piece));
        assert!(reader.buffered() <= bound, "{label}: {} bytes held", reader.buffered());
        match result {
            Ok(()) => {}
            Err(ReadError::Form(error) | ReadError::Sink(error)) => {
                failure = Some(error);
                break;
            }
            Err(ReadError::SinkFailed) => panic!("{label}: a sink failed twice"),
        }
    }
    if let Some(error) = failure {
        return Err(error);
    }
    match reader.finish() {
        Ok(()) => Ok(form),
        Err(ReadError::Form(error)) => Err(error),
        Err(ReadError::Sink(never)) => match never {},
        Err(ReadError::SinkFailed) => panic!("{label}: a sink failed"),
    }
}

#[test]
fn urlencoded() {
    let cases = contract("fields.json");
    let list = group(&cases, "urlencoded");
    let replaced = superseded("urlencoded", list);
    let mut failures = Failures::default();
    for case in list.iter().filter(|case| !replaced.contains_key(id(case))) {
        let read = parse_urlencoded(string(case, "text").as_bytes());
        let (fields, names) = match &read {
            Ok(fields) => (
                fields_json(fields),
                json!(fields.entries().iter().map(|(name, _)| name.clone()).collect::<Vec<_>>()),
            ),
            Err(_) => (Value::Null, Value::Null),
        };
        failures.compare(case, "fields", &fields);
        failures.compare(case, "names", &names);
    }
    failures.check("urlencoded");
}

#[test]
fn multipart() {
    let cases = contract("fields.json");
    let list = group(&cases, "multipart");
    let replaced = superseded("multipart", list);
    let mut failures = Failures::default();
    for case in list.iter().filter(|case| !replaced.contains_key(id(case))) {
        let label = id(case);
        let kind = latin1(&case["type"]);
        let kind = Some(kind.as_slice());
        let body = latin1(&case["body"]);
        let whole = (
            match fields_from_body(kind, &body) {
                Ok(fields) => fields_json(&fields),
                Err(_) => Value::Null,
            },
            match files_from_body(kind, &body) {
                Ok(files) => files_json(&files, false),
                Err(_) => Value::Null,
            },
        );
        let mut result = whole.clone();
        // HY-131.6: two chunks divided at every position, and every byte alone, give the result of the whole body,
        // and the reader holds between two chunks at most its bound.
        if let Ok(Some(boundary)) = multipart_boundary(kind) {
            let bound = (HEADER_LIMIT + 3).max(boundary.len() + 3);
            let bytes: Vec<&[u8]> = body.chunks(1).collect();
            let mut divisions: Vec<Vec<&[u8]>> = vec![bytes];
            for at in 0..=body.len() {
                divisions.push(vec![&body[..at], &body[at..]]);
            }
            for chunks in divisions {
                let divided = outcome(read_chunks(&boundary, &chunks, bound, label));
                if divided != whole {
                    result = divided;
                    break;
                }
            }
        }
        failures.compare(case, "fields", &result.0);
        failures.compare(case, "files", &result.1);
    }
    failures.check("multipart");
}

fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("http-forms-case-{}-{label}", std::process::id()));
    match fs::remove_dir_all(&path) {
        Ok(()) => {}
        Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::NotFound, "{label}"),
    }
    fs::create_dir(&path).unwrap_or_else(|error| panic!("{label}: {error}"));
    path
}

fn entries(directory: &Option<PathBuf>) -> Value {
    match directory {
        Some(directory) => match fs::read_dir(directory) {
            Ok(list) => json!(list.count()),
            Err(error) => panic!("{}: {error}", directory.display()),
        },
        None => Value::Null,
    }
}

/// The class of HY-46.2 is the category `value` (plan F-3).
fn failure(error: &Error) -> (String, &'static str) {
    (error.to_string(), common::category(error.kind()))
}

#[test]
fn streamed() {
    let cases = contract("fields.json");
    let list = group(&cases, "streamed");
    let replaced = superseded("streamed", list);
    let binding = listed("binding", "streamed");
    let mut failures = Failures::default();
    for case in list.iter().filter(|case| !replaced.contains_key(id(case))) {
        let label = id(case);
        if binding.contains_key(label) {
            // A path of another type than a path is refused by the binding before the core (plan B-3).
            assert_eq!(case["move"], json!("type"), "{label}: only a path of another type is a binding case");
            continue;
        }
        let kind = latin1(&case["type"]);
        let kind = Some(kind.as_slice());
        let body = latin1(&case["body"]);
        let chunk = usize::try_from(number(case, "chunk")).unwrap_or_else(|_| panic!("{label}: chunk"));
        let mut read = RequestBody::new(
            kind,
            None,
            number(case, "limit"),
            case["stream"] == json!(true),
            number(case, "valueLimit"),
            number(case, "partLimit"),
            number(case, "fileLimit"),
        )
        .unwrap_or_else(|error| panic!("{label}: {error}"));
        let place = scratch(label);
        // `stopped` is true when a chunk failed, after which the host reads no more (H-5).
        let mut stopped = None;
        for piece in body.chunks(chunk) {
            if let Err(error) = read.feed(piece) {
                stopped = Some(error);
                break;
            }
        }
        let finished = match &stopped {
            Some(_) => None,
            None => read.finish().err(),
        };
        let held = read.held().to_vec();
        let (fields, files) = if read.streamed() {
            match read.parts() {
                Ok(form) => (Some(form.fields), Some(form.files)),
                Err(_) => (None, None),
            }
        } else if stopped.is_some() || finished.is_some() {
            (None, None)
        } else {
            match (fields_from_body(kind, &held), files_from_body(kind, &held)) {
                (Ok(fields), Ok(files)) => (Some(fields), Some(files)),
                (Err(_), _) | (_, Err(_)) => (None, None),
            }
        };
        let directory = read.directory().map(Path::to_path_buf);
        let exceeded = stopped.as_ref().is_some_and(|error| error.status() == Some(413));
        failures.compare(case, "stopped", &json!(stopped.is_some()));
        failures.compare(case, "exceeded", &json!(exceeded));
        failures.compare(case, "held", &json!(text(&held)));
        failures.compare(case, "fields", &Value::from(fields.as_ref().map(fields_json)));
        failures.compare(case, "files", &Value::from(files.as_ref().map(|f| files_json(f, true))));
        failures.compare(case, "entries", &entries(&directory));
        let first = files.as_ref().and_then(|f| f.entries().first().and_then(|(_, items)| items.first().cloned()));
        let movement = string(case, "move");
        match (movement, first) {
            ("none", _) | (_, None) => failures.compare(case, "moved", &Value::Null),
            (_, Some(first)) => {
                let target = place.join("target");
                if movement == "exists" {
                    fs::write(&target, b"taken").unwrap_or_else(|error| panic!("{label}: {error}"));
                }
                let moved = match first.move_to(&target) {
                    Ok(()) => {
                        let content = fs::read(&target).unwrap_or_else(|error| panic!("{label}: {error}"));
                        let again = first.move_to(&place.join("second")).err();
                        let after = first.content().err();
                        json!({
                            "content": text(&content),
                            "left": entries(&directory),
                            "again": again.as_ref().map(|e| failure(e).0),
                            "read": after.as_ref().map(ToString::to_string),
                            "class": again.as_ref().map(|e| failure(e).1),
                        })
                    }
                    Err(error) => {
                        let content = first.content().unwrap_or_else(|error| panic!("{label}: {error}"));
                        json!({
                            "content": text(&content),
                            "left": entries(&directory),
                            "again": failure(&error).0,
                            "read": Value::Null,
                            "class": failure(&error).1,
                        })
                    }
                };
                failures.compare(case, "moved", &moved);
            }
        }
        read.remove().unwrap_or_else(|error| panic!("{label}: {error}"));
        let removed = Value::from(directory.as_ref().map(|d| !d.exists()));
        failures.compare(case, "removed", &removed);
        fs::remove_dir_all(&place).unwrap_or_else(|error| panic!("{label}: {error}"));
    }
    failures.check("streamed");
}

#[test]
fn uploaded() {
    let cases = contract("fields.json");
    let list = group(&cases, "uploaded");
    let mut failures = Failures::default();
    for case in list {
        let label = id(case);
        // HY-113.3, H-24: the content is bytes, as the characters of the text are.
        let file = UploadedFile::from_value(
            string(case, "name").to_owned(),
            string(case, "filename").to_owned(),
            string(case, "contentType").to_owned(),
            latin1(&case["content"]),
        )
        .unwrap_or_else(|error| panic!("{label}: {error}"));
        let content = file.content().unwrap_or_else(|error| panic!("{label}: {error}"));
        let made = json!({
            "name": file.name(),
            "filename": file.filename(),
            "contentType": file.content_type(),
            "size": file.size(),
            "content": text(&content),
        });
        failures.compare(case, "file", &made);
    }
    failures.check("uploaded");
}
