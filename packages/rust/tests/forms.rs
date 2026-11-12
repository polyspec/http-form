//! Runs the cases of `contracts/forms.json`, the cases of the rules of this library (docs/spec/forms.md). A body is
//! read whole, byte by byte and in two chunks divided at every position, and every division must give the result of
//! the case (HY-131.6).

mod common;

use std::path::Path;

use common::{contract, fields_json, files_json, group, id, latin1, number, raises, string, text, Failures};
use polyspec_http_forms::{multipart_boundary, read_body, Error, Form, Limits, MultipartReader, ReadError, RequestBody};
use serde_json::{json, Value};

/// The divisions of a body: whole, byte by byte, and in two chunks at every position.
fn divisions(body: &[u8]) -> Vec<Vec<&[u8]>> {
    let mut all = vec![vec![body], body.chunks(1).collect()];
    for at in 0..=body.len() {
        all.push(vec![&body[..at], &body[at..]]);
    }
    all
}

fn form_result(result: Result<Form, Error>) -> Value {
    match result {
        Ok(form) => json!({"fields": fields_json(&form.fields), "files": files_json(&form.files, false)}),
        Err(error) => json!({"raises": raises(&error)}),
    }
}

fn read_divided(boundary: &[u8], chunks: &[&[u8]]) -> Result<Form, Error> {
    let mut reader = MultipartReader::new(boundary, Limits::default());
    let mut form = Form::default();
    for chunk in chunks {
        match reader.feed_with(chunk, |piece| form.take(piece)) {
            Ok(()) => {}
            Err(ReadError::Form(error) | ReadError::Sink(error)) => return Err(error),
            Err(ReadError::SinkFailed) => return Err(Error::Internal("a sink failed twice")),
        }
    }
    match reader.finish() {
        Ok(()) => Ok(form),
        Err(ReadError::Form(error)) => Err(error),
        Err(ReadError::Sink(never)) => match never {},
        Err(ReadError::SinkFailed) => Err(Error::Internal("a sink failed")),
    }
}

#[test]
fn multipart() {
    let cases = contract("forms.json");
    let mut failures = Failures::default();
    for case in group(&cases, "multipart") {
        let label = id(case);
        let kind = latin1(&case["type"]);
        let kind = Some(kind.as_slice());
        let body = latin1(&case["body"]);
        let whole = form_result(read_body(kind, &body));
        if let Ok(Some(boundary)) = multipart_boundary(kind) {
            for chunks in divisions(&body) {
                let divided = form_result(read_divided(&boundary, &chunks));
                if divided != whole {
                    failures.push(format!("{label}: {} chunks give {divided}, the whole body {whole}", chunks.len()));
                    break;
                }
            }
        }
        for key in ["fields", "files", "raises"] {
            failures.compare(case, key, &whole[key]);
        }
        let keys = ["fields", "files", "raises"].iter().filter(|key| case.get(**key).is_some()).count();
        assert!(keys > 0, "{label}: no expectation");
    }
    failures.check("multipart");
}

#[test]
fn file_over() {
    let cases = contract("forms.json");
    let mut failures = Failures::default();
    for case in group(&cases, "file_over") {
        let label = id(case);
        let Some(limits) = case["limits"].as_array() else { panic!("{label}: limits") };
        if case.get("layer") == Some(&json!("binding")) {
            // B-2: a limit that is no u64 is refused by the binding with invalid_number before the core.
            for limit in limits {
                assert!(limit.as_u64().is_none(), "{label}: {limit} is a u64, which the core reads");
            }
            assert_eq!(case["raises"]["kind"], json!("invalid_number"), "{label}");
            continue;
        }
        let form = match read_body(Some(&latin1(&case["type"])), &latin1(&case["body"])) {
            Ok(form) => form,
            Err(error) => panic!("{label}: {error}"),
        };
        let over: Vec<bool> = limits
            .iter()
            .map(|limit| match limit.as_u64() {
                Some(limit) => form.file_over(limit),
                None => panic!("{label}: {limit}"),
            })
            .collect();
        failures.compare(case, "over", &json!(over));
    }
    failures.check("file_over");
}

/// The result of a body read in the given chunks: where it failed and with what, or its parts.
fn run_body(case: &Value, chunks: &[&[u8]]) -> Value {
    let label = id(case);
    let content_length = case.get("contentLength").map(latin1);
    let mut read = match RequestBody::new(
        Some(&latin1(&case["type"])),
        content_length.as_deref(),
        number(case, "limit"),
        case["stream"] == json!(true),
        number(case, "valueLimit"),
        number(case, "partLimit"),
        number(case, "fileLimit"),
    ) {
        Ok(read) => read,
        Err(error) => return json!({"stage": "new", "raises": raises(&error)}),
    };
    let directory = read.directory().map(Path::to_path_buf);
    let mut result = json!({});
    let mut failed = None;
    for chunk in chunks {
        if let Err(error) = read.feed(chunk) {
            // HY-131.11: every later call gives the same error.
            assert_eq!(read.feed(b"more"), Err(error.clone()), "{label}");
            failed = Some(("feed", error));
            break;
        }
    }
    if failed.is_none() && case.get("finish") != Some(&json!(false)) {
        if let Err(error) = read.finish() {
            failed = Some(("finish", error));
        }
    }
    result["held"] = json!(text(read.held()));
    if failed.is_none() && (read.streamed() || case.get("parts") == Some(&json!(true))) {
        match read.parts() {
            Ok(form) => {
                result["fields"] = fields_json(&form.fields);
                result["files"] = files_json(&form.files, true);
                if case.get("afterRemoval").is_some() {
                    if let Err(error) = read.remove() {
                        panic!("{label}: {error}");
                    }
                    let mut after = Vec::new();
                    for (_, files) in form.files.entries() {
                        for file in files {
                            after.push(file.content().map_err(|error| raises(&error)));
                            let target = std::env::temp_dir().join(format!("http-forms-after-{}", std::process::id()));
                            after.push(file.move_to(&target).map(|()| Vec::new()).map_err(|error| raises(&error)));
                        }
                    }
                    let mut kinds = after.into_iter().map(|result| match result {
                        Ok(_) => Value::Null,
                        Err(raised) => raised,
                    });
                    let first = kinds.next();
                    assert!(kinds.all(|other| Some(&other) == first.as_ref()), "{label}: the files fail alike");
                    result["afterRemoval"] = match first {
                        Some(first) => first,
                        None => Value::Null,
                    };
                }
            }
            Err(error) => failed = Some(("parts", error)),
        }
    }
    if let Some((stage, error)) = failed {
        result["stage"] = json!(stage);
        result["raises"] = raises(&error);
        if case.get("held").is_none() {
            assert_eq!(read.held(), b"", "{label}: a failed body holds no bytes");
        }
    }
    if let Err(error) = read.remove() {
        panic!("{label}: {error}");
    }
    if let Some(directory) = directory {
        assert!(!directory.exists(), "{label}: the directory was removed");
    }
    result
}

#[test]
fn bodies() {
    let cases = contract("forms.json");
    let mut failures = Failures::default();
    for case in group(&cases, "body") {
        let label = id(case);
        let body = latin1(&case["body"]);
        let all = divisions(&body);
        let whole = run_body(case, &all[0]);
        for chunks in &all[1..] {
            let divided = run_body(case, chunks);
            if divided != whole {
                failures.push(format!("{label}: {} chunks give {divided}, one chunk {whole}", chunks.len()));
                break;
            }
        }
        for key in ["stage", "raises", "held", "fields", "files", "afterRemoval"] {
            failures.compare(case, key, &whole[key]);
        }
        if case.get("raises").is_none() {
            assert!(whole.get("raises").is_none(), "{label}: failed with {whole}");
        }
        assert!(!string(case, "label").is_empty(), "{label}");
    }
    failures.check("body");
}
