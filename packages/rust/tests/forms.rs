//! Runs the cases of `contracts/forms.json`, the cases of the rules of this library (docs/spec/forms.md). A body is
//! read whole, byte by byte and in two chunks divided at every position, and every division must give the result of
//! the case (HY-131.6).

mod common;

use common::{contract, io, fields_json, files_json, group, id, latin1, number, raises, string, text, Failures};
use polyspec_http_forms::{
    file_over, is_urlencoded, media_type, multipart_boundary, read_body, BodyLimits, Collector, Error, Form, Limits,
    MultipartReader, ReadError, RequestBody,
};
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
    let mut form = Collector::default();
    for chunk in chunks {
        match reader.feed_with(chunk, |piece| form.take(piece)) {
            Ok(()) => {}
            Err(ReadError::Form(error) | ReadError::Sink(error)) => return Err(error),
            Err(ReadError::SinkFailed) => return Err(Error::Internal("a sink failed twice")),
        }
    }
    match reader.finish() {
        Ok(()) => form.into_form(),
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

/// HY-59.11, HY-57.10, HY-113.11, spec F-12: the media type, the urlencoded type and the boundary of a content type,
/// which `null` leaves out; a case with `raises` fails in all three.
#[test]
fn content_type() {
    let cases = contract("forms.json");
    let mut failures = Failures::default();
    for case in group(&cases, "content_type") {
        let label = id(case);
        let value = match &case["type"] {
            Value::Null => None,
            other => Some(latin1(other)),
        };
        let value = value.as_deref();
        let results = [
            ("mediaType", media_type(value).map(Value::from)),
            ("urlencoded", is_urlencoded(value).map(Value::from)),
            ("boundary", multipart_boundary(value).map(|boundary| Value::from(boundary.map(|b| text(&b))))),
        ];
        for (key, result) in results {
            match (result, case.get("raises")) {
                (Ok(actual), None) => failures.compare(case, key, &actual),
                (Err(error), Some(_)) => failures.compare(case, "raises", &raises(&error)),
                (Ok(actual), Some(_)) => failures.push(format!("{label}: {key} gives {actual} and does not fail")),
                (Err(error), None) => failures.push(format!("{label}: {key} fails with {}", error.kind())),
            }
        }
    }
    failures.check("content_type");
}

#[test]
fn files_over_a_limit() {
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
                Some(limit) => file_over(&form.files, limit),
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
        io(),
        Some(&latin1(&case["type"])),
        content_length.as_deref(),
        case["stream"] == json!(true),
        BodyLimits {
            limit: number(case, "limit"),
            value_limit: number(case, "valueLimit"),
            part_limit: number(case, "partLimit"),
            file_limit: number(case, "fileLimit"),
        },
    ) {
        Ok(read) => read,
        Err(error) => return json!({"stage": "new", "raises": raises(&error)}),
    };
    let directory = read.directory();
    let mut result = json!({});
    let mut failed = None;
    for chunk in chunks {
        if let Err(error) = read.feed(io(), chunk) {
            // HY-131.11: every later call gives the same error.
            assert_eq!(read.feed(io(), b"more"), Err(error.clone()), "{label}");
            failed = Some(("feed", error));
            break;
        }
    }
    if failed.is_none() && case.get("finish") != Some(&json!(false)) {
        if let Err(error) = read.finish(io()) {
            failed = Some(("finish", error));
        }
    }
    result["held"] = json!(text(&read.held()));
    if failed.is_none() && (read.streamed() || case.get("parts") == Some(&json!(true))) {
        match read.parts() {
            Ok(form) => {
                result["fields"] = fields_json(&form.fields);
                result["files"] = files_json(&form.files, true);
                if case.get("afterRemoval").is_some() {
                    if let Err(error) = read.remove(io()) {
                        panic!("{label}: {error}");
                    }
                    let mut after = Vec::new();
                    for (_, files) in &form.files {
                        for file in files {
                            after.push(file.content(io()).map_err(|error| raises(&error)));
                            let target = std::env::temp_dir().join(format!("http-forms-after-{}", std::process::id()));
                            after.push(file.move_to(io(), &target).map(|()| Vec::new()).map_err(|error| raises(&error)));
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
    if let Err(error) = read.remove(io()) {
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
