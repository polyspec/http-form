//! Runs the cases of `contracts/fields.json`: urlencoded text (F-20), multipart bodies (F-21 to F-24), bodies read in
//! chunks with their limits and files (F-9, F-10, F-11, F-25) and files made from bytes (F-22.2). A urlencoded text and
//! a body are bytes in the case convention of binding (G-5, `{"hex": ...}`), a content type is a text whose characters
//! are its bytes, and the limits are the record `limits`. A case with `raises` gives that failure. A case that only a
//! binding can run, because its input is no value of the core's types, is listed in the group `binding` of
//! `contracts/forms.json` with its rule.
mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use common::{
    bytes, content_type, contract, fields_json, files_json, group, id, io, limits, number, raised, raises, string,
    text, Arming, Failures,
};
use polyspec_http_form::{
    fields_from_body, files_from_body, multipart_boundary, parse_urlencoded, Collector, Error, Form, Limits,
    MultipartReader, ReadError, RequestBody, UploadedFile, HEADER_LIMIT,
};
use serde_json::{json, Value};

/// The ids of the cases of a group of fields.json that a binding runs.
fn binding_cases(source_group: &str) -> BTreeMap<String, Value> {
    let forms = contract("forms.json");
    group(&forms, "binding")
        .iter()
        .filter(|entry| entry["source"] == json!("fields.json") && entry["group"] == json!(source_group))
        .map(|entry| (string(entry, "id").to_owned(), entry.clone()))
        .collect()
}

/// The failure of a case, or `null` (plan F-3).
fn failed(error: Option<&Error>) -> Value {
    match error {
        Some(error) => raises(error),
        None => Value::Null,
    }
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
    let mut form = Collector::default();
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
        Ok(()) => form.into_form(),
        Err(ReadError::Form(error)) => Err(error),
        Err(ReadError::Sink(never)) => match never {},
        Err(ReadError::SinkFailed) => panic!("{label}: a sink failed"),
    }
}

#[test]
fn urlencoded() {
    let cases = contract("fields.json");
    let list = group(&cases, "urlencoded");
    let mut failures = Failures::default();
    for case in list {
        let read = parse_urlencoded(&bytes(&case["text"]));
        failures.compare(case, "raises", &failed(read.as_ref().err()));
        let (fields, names) = match &read {
            Ok(fields) => (fields_json(fields), json!(fields.iter().map(|(name, _)| name.clone()).collect::<Vec<_>>())),
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
    let mut failures = Failures::default();
    for case in list {
        let label = id(case);
        let kind = content_type(case);
        let kind = kind.as_deref();
        let body = bytes(&case["body"]);
        failures.compare(case, "raises", &failed(fields_from_body(kind, &body).err().as_ref()));
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
        // F-24.6: two chunks divided at every position, and every byte alone, give the result of the whole body,
        // and the reader holds between two chunks at most its bound.
        if let Ok(Some(boundary)) = multipart_boundary(kind) {
            let header = usize::try_from(HEADER_LIMIT).unwrap_or_else(|error| panic!("{label}: {error}"));
            let bound = (header + 3).max(boundary.len() + 3);
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
    let path = std::env::temp_dir().join(format!("http-form-case-{}-{label}", std::process::id()));
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

/// The category of `moved`, `exists` and `removed` is `value` (plan F-3).
fn failure(error: &Error) -> (String, &'static str) {
    (error.to_string(), common::category(error.kind()))
}

#[test]
fn streamed() {
    let cases = contract("fields.json");
    let list = group(&cases, "streamed");
    let binding = binding_cases("streamed");
    let mut failures = Failures::default();
    for case in list {
        let label = id(case);
        if binding.contains_key(label) {
            // A path of another type than a path is refused by the binding before the core (plan B-3).
            assert_eq!(case["move"], json!("type"), "{label}: only a path of another type is a binding case");
            continue;
        }
        let kind = content_type(case);
        let kind = kind.as_deref();
        let body = bytes(&case["body"]);
        let chunk = usize::try_from(number(case, "chunk")).unwrap_or_else(|_| panic!("{label}: chunk"));
        let mut read = RequestBody::new(io(), kind, None, case["stream"] == json!(true), limits(case))
            .unwrap_or_else(|error| panic!("{label}: {error}"));
        let place = scratch(label);
        // `stopped` is true when a chunk failed, after which the host reads no more (H-5).
        let mut stopped = None;
        for piece in body.chunks(chunk) {
            if let Err(error) = read.feed(io(), piece) {
                stopped = Some(error);
                break;
            }
        }
        let finished = match &stopped {
            Some(_) => None,
            None => read.finish(io()).err(),
        };
        let held = read.held().to_vec();
        let read_parts = if let Some(error) = stopped.as_ref().or(finished.as_ref()) {
            Err(error.clone())
        } else if read.streamed() {
            read.parts().map(|form| (form.fields, form.files))
        } else {
            fields_from_body(kind, &held).and_then(|fields| Ok((fields, files_from_body(kind, &held)?)))
        };
        failures.compare(case, "raises", &failed(read_parts.as_ref().err()));
        let (fields, files) = match read_parts {
            Ok((fields, files)) => (Some(fields), Some(files)),
            Err(_) => (None, None),
        };
        let directory = read.directory();
        let exceeded = stopped.as_ref().is_some_and(|error| error.status() == Some(413));
        failures.compare(case, "stopped", &json!(stopped.is_some()));
        failures.compare(case, "exceeded", &json!(exceeded));
        failures.compare(case, "held", &json!(text(&held)));
        failures.compare(case, "fields", &Value::from(fields.as_ref().map(fields_json)));
        failures.compare(case, "files", &Value::from(files.as_ref().map(|f| files_json(f, true))));
        failures.compare(case, "entries", &entries(&directory));
        let first = files.as_ref().and_then(|f| f.first().and_then(|(_, items)| items.first().cloned()));
        let movement = string(case, "move");
        match (movement, first) {
            ("none", _) | (_, None) => failures.compare(case, "moved", &Value::Null),
            (_, Some(first)) => {
                let target = place.join("target");
                if movement == "exists" {
                    fs::write(&target, b"taken").unwrap_or_else(|error| panic!("{label}: {error}"));
                }
                let moved = match first.move_to(io(), &target) {
                    Ok(()) => {
                        let content = fs::read(&target).unwrap_or_else(|error| panic!("{label}: {error}"));
                        let again = first.move_to(io(), &place.join("second")).err();
                        let after = first.content(io()).err();
                        json!({
                            "content": text(&content),
                            "left": entries(&directory),
                            "again": again.as_ref().map(|e| failure(e).0),
                            "read": after.as_ref().map(ToString::to_string),
                            "class": again.as_ref().map(|e| failure(e).1),
                        })
                    }
                    Err(error) => {
                        let content = first.content(io()).unwrap_or_else(|error| panic!("{label}: {error}"));
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
        read.remove(io()).unwrap_or_else(|error| panic!("{label}: {error}"));
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
        // F-22.2, H-24: the content is bytes, as the characters of the text are.
        let file = UploadedFile::from_value(
            string(case, "name").to_owned(),
            string(case, "filename").to_owned(),
            string(case, "content_type").to_owned(),
            bytes(&case["content"]),
        )
        .unwrap_or_else(|error| panic!("{label}: {error}"));
        let content = file.content(io()).unwrap_or_else(|error| panic!("{label}: {error}"));
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

/// A file that an application makes from the keys of `from_value` (G-5).
fn value_file(case: &Value) -> UploadedFile {
    match UploadedFile::from_value(
        string(case, "name").to_owned(),
        string(case, "filename").to_owned(),
        string(case, "content_type").to_owned(),
        bytes(&case["content"]),
    ) {
        Ok(file) => file,
        Err(error) => panic!("{}: {error}", id(case)),
    }
}

/// F-11, F-10, X-1: one call of the members whose other cases are scenarios of `streamed`, with the faults that a
/// case arms (G-5): a file that an application makes moves to a path below a scratch directory or fails, a body that
/// was just made is removed or fails, and the content of a file fails with a broken invariant. A failed move leaves no
/// file at its path.
#[test]
fn one_call() {
    let cases = contract("fields.json");
    let mut failures = Failures::default();
    for case in group(&cases, "uploaded_file.move_to") {
        let label = id(case);
        let file = value_file(case);
        let place = scratch(label);
        let target = place.join(string(case, "path"));
        let arming = Arming::of(case, "uploaded_file.move_to");
        let moved = arming.run(|| file.move_to(io(), &target));
        arming.done();
        match moved {
            Ok(()) => {
                let moved = fs::read(&target).unwrap_or_else(|error| panic!("{label}: {error}"));
                assert_eq!(moved, bytes(&case["content"]), "{label}: the moved bytes");
                assert!(case.get("raises").is_none(), "{label}: the file moved");
            }
            Err(error) => {
                failures.compare(case, "raises", &raised(case, &error));
                if target != place.join(".") {
                    assert!(!target.exists(), "{label}: a failed move leaves no file at its path");
                }
            }
        }
        fs::remove_dir_all(&place).unwrap_or_else(|error| panic!("{label}: {error}"));
    }
    for case in group(&cases, "uploaded_file.content") {
        let file = value_file(case);
        let arming = Arming::of(case, "uploaded_file.content");
        let content = arming.run(|| file.content(io()));
        arming.done();
        match content {
            Ok(content) => failures.compare(case, "content", &json!({"hex": hex(&content)})),
            Err(error) => failures.compare(case, "raises", &raised(case, &error)),
        }
    }
    for case in group(&cases, "request_body.remove") {
        let label = id(case);
        let mut read =
            RequestBody::new(io(), content_type(case).as_deref(), None, case["stream"] == json!(true), limits(case))
                .unwrap_or_else(|error| panic!("{label}: {error}"));
        let directory = read.directory();
        let arming = Arming::of(case, "request_body.remove");
        let removed = arming.run(|| read.remove(io()));
        arming.done();
        match removed {
            Ok(()) => assert!(case.get("raises").is_none(), "{label}: the body was removed"),
            Err(error) => {
                failures.compare(case, "raises", &raised(case, &error));
                // F-10: a removal that failed removes nothing, and a later call removes the directory.
                read.remove(io()).unwrap_or_else(|error| panic!("{label}: {error}"));
            }
        }
        if let Some(directory) = directory {
            assert!(!directory.exists(), "{label}: the directory was removed");
        }
    }
    failures.check("one call");
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
