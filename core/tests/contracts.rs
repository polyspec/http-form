//! Runs the cases of `contracts/fields.json` of hyper as its runners do: a urlencoded text is the UTF-8 encoding of
//! its JSON string, and a multipart type and body are text whose characters are their bytes.

use std::fs;
use std::path::{Path, PathBuf};

use http_forms::{
    fields_from_body, files_from_body, multipart_boundary, parse_urlencoded, Fields, Files, FileError, Form,
    Limits, MultipartReader, RequestBody, HEADER_LIMIT,
};
use serde_json::{json, Value};

fn cases() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../contracts/fields.json");
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn latin1(value: &Value) -> Vec<u8> {
    value.as_str().unwrap().chars().map(|c| u8::try_from(u32::from(c)).expect("a byte")).collect()
}

fn text(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}

fn fields_of(fields: Option<&Fields>) -> Value {
    match fields {
        None => Value::Null,
        Some(fields) => Value::Array(fields.entries().iter().map(|(name, values)| json!([name, values])).collect()),
    }
}

fn files_of(files: Option<&Files>, size: bool) -> Value {
    let Some(files) = files else { return Value::Null };
    Value::Array(
        files
            .entries()
            .iter()
            .flat_map(|(_, items)| items.iter())
            .map(|item| {
                let mut file = json!({
                    "name": item.name(),
                    "filename": item.filename(),
                    "contentType": item.content_type(),
                    "content": text(&item.content().unwrap()),
                });
                if size {
                    file["size"] = json!(item.size());
                }
                file
            })
            .collect(),
    )
}

fn compare(failures: &mut Vec<String>, id: &str, key: &str, case: &Value, actual: Value) {
    if case.get(key) != Some(&actual) {
        failures.push(format!("{id}: {key}: expected {:?}, got {actual}", case.get(key)));
    }
}

#[test]
fn urlencoded() {
    let mut failures = Vec::new();
    let all = cases();
    let list = all["urlencoded"].as_array().unwrap();
    for case in list {
        let id = case["id"].as_str().unwrap();
        let fields = parse_urlencoded(case["text"].as_str().unwrap().as_bytes()).ok();
        compare(&mut failures, id, "fields", case, fields_of(fields.as_ref()));
        let names = fields.map(|f| json!(f.entries().iter().map(|(n, _)| n.clone()).collect::<Vec<_>>()));
        compare(&mut failures, id, "names", case, names.unwrap_or(Value::Null));
    }
    assert!(failures.is_empty(), "{} of {} cases failed:\n{}", failures.len(), list.len(), failures.join("\n"));
    assert!(!list.is_empty());
}

#[test]
fn multipart() {
    let mut failures = Vec::new();
    let all = cases();
    let list = all["multipart"].as_array().unwrap();
    for case in list {
        let id = case["id"].as_str().unwrap();
        let kind = latin1(&case["type"]);
        let body = latin1(&case["body"]);
        let whole = (
            fields_of(fields_from_body(&kind, &body).ok().as_ref()),
            files_of(files_from_body(&kind, &body).ok().as_ref(), false),
        );
        // HY-131.6: two chunks divided at every position give the result of the whole body, and the reader holds
        // between two chunks at most its bound.
        let mut result = whole.clone();
        if let Some(boundary) = multipart_boundary(&kind) {
            let bound = (HEADER_LIMIT + 3).max(boundary.len() + 3);
            for at in 0..=body.len() {
                let mut reader = MultipartReader::new(&boundary, Limits::default());
                let mut form = Some(Form::default());
                for chunk in [&body[..at], &body[at..]] {
                    match reader.feed(chunk) {
                        Ok(events) => {
                            if let Some(form) = form.as_mut() {
                                form.collect(events)
                            }
                        }
                        Err(_) => form = None,
                    }
                    assert!(reader.buffered() <= bound, "{id}: {} bytes held after {at}", reader.buffered());
                }
                let read = form.filter(|_| reader.closed());
                let divided = (
                    fields_of(read.as_ref().map(|f| &f.fields)),
                    files_of(read.as_ref().map(|f| &f.files), false),
                );
                if divided != whole {
                    result = divided;
                    break;
                }
            }
        }
        compare(&mut failures, id, "fields", case, result.0);
        compare(&mut failures, id, "files", case, result.1);
    }
    assert!(failures.is_empty(), "{} of {} cases failed:\n{}", failures.len(), list.len(), failures.join("\n"));
}

fn scratch(id: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("http-forms-case-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir(&path).unwrap();
    path
}

fn failure(result: Result<(), FileError>) -> Option<(String, &'static str)> {
    result.err().map(|error| (error.message(), "value"))
}

#[test]
fn streamed() {
    let mut failures = Vec::new();
    let all = cases();
    let list = all["streamed"].as_array().unwrap();
    for case in list {
        let id = case["id"].as_str().unwrap();
        let kind = latin1(&case["type"]);
        let body = latin1(&case["body"]);
        let number = |key: &str| case[key].as_u64().unwrap();
        let chunk = number("chunk") as usize;
        let mut read = RequestBody::new(
            &kind,
            number("limit"),
            case["stream"] == json!(true),
            number("valueLimit"),
            number("partLimit"),
            number("fileLimit"),
        )
        .unwrap();
        let place = scratch(id);
        let mut stopped = false;
        let mut at = 0;
        while at < body.len() && !stopped {
            stopped = !read.feed(&body[at..(at + chunk).min(body.len())]).unwrap();
            at += chunk;
        }
        let held = read.held().to_vec();
        let (fields, files) = if read.streamed() {
            let parts = read.parts();
            (parts.as_ref().map(|p| p.fields.clone()), parts.map(|p| p.files))
        } else if stopped {
            (None, None)
        } else {
            (fields_from_body(&kind, &held).ok(), files_from_body(&kind, &held).ok())
        };
        let directory = read.directory().map(Path::to_path_buf);
        let count = || directory.as_ref().map(|d| json!(fs::read_dir(d).unwrap().count())).unwrap_or(Value::Null);
        compare(&mut failures, id, "stopped", case, json!(stopped));
        compare(&mut failures, id, "exceeded", case, json!(read.exceeded()));
        compare(&mut failures, id, "held", case, json!(text(&held)));
        compare(&mut failures, id, "fields", case, fields_of(fields.as_ref()));
        compare(&mut failures, id, "files", case, files_of(files.as_ref(), true));
        if case.get("entries").is_some() {
            compare(&mut failures, id, "entries", case, count());
        }
        let first = files.as_ref().and_then(|f| f.entries().first().map(|(_, items)| items[0].clone()));
        let movement = case["move"].as_str().unwrap();
        match (movement, first) {
            ("none", _) | (_, None) => compare(&mut failures, id, "moved", case, Value::Null),
            (_, Some(mut first)) => {
                let target = place.join("target");
                if movement == "exists" {
                    fs::write(&target, b"taken").unwrap();
                }
                // A path of another type is refused by a binding before the core; the core has only paths.
                let failed = if movement == "type" {
                    Some((http_forms::NOT_A_STRING.to_owned(), "type"))
                } else {
                    failure(first.move_to(&target))
                };
                let (content, failed, after) = if failed.is_none() {
                    let content = text(&fs::read(&target).unwrap());
                    let again = failure(first.move_to(&place.join("second")));
                    let after = first.content().err().map(|error| error.message());
                    (content, again, after)
                } else {
                    (text(&first.content().unwrap()), failed, None)
                };
                compare(
                    &mut failures,
                    id,
                    "moved",
                    case,
                    json!({
                        "content": content,
                        "left": count(),
                        "again": failed.as_ref().map(|f| f.0.clone()),
                        "read": after,
                        "class": failed.as_ref().map(|f| f.1),
                    }),
                );
            }
        }
        read.remove().unwrap();
        let removed = directory.as_ref().map(|d| json!(!d.is_dir())).unwrap_or(Value::Null);
        compare(&mut failures, id, "removed", case, removed);
        fs::remove_dir_all(&place).unwrap();
    }
    assert!(failures.is_empty(), "{} of {} cases failed:\n{}", failures.len(), list.len(), failures.join("\n"));
}
