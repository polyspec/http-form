//! Runs the cases of `contracts/forms.json`, the cases of the rules of this library (docs/spec/forms.md). A body is
//! read whole, byte by byte and in two chunks divided at every position, and every division must give the result of
//! the case (F-24.6).

mod common;

use common::{
    bytes, content_type, contract, fields_json, files_json, group, id, io, latin1, limits, raised, raises, string,
    text, Arming, Failures,
};
use polyspec_http_form::{
    file_over, is_urlencoded, media_type, multipart_boundary, read_body, read_held_body, Collector, Error, Form,
    Limits, MultipartReader, ReadError, RequestBody,
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
        let kind = content_type(case);
        let kind = kind.as_deref();
        let body = bytes(&case["body"]);
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

/// F-12.1, F-12.2, F-12.3, spec F-12: the media type, the urlencoded type and the boundary of a content type,
/// which `null` leaves out; a case with `raises` fails in all three.
#[test]
fn content_type_cases() {
    let cases = contract("forms.json");
    let mut failures = Failures::default();
    for case in group(&cases, "content_type") {
        let label = id(case);
        let value = content_type(case);
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

/// F-25.6: a body held as one value with the limits of a body.
#[test]
fn held() {
    let cases = contract("forms.json");
    let mut failures = Failures::default();
    for case in group(&cases, "held") {
        let kind = content_type(case);
        let result = form_result(read_held_body(kind.as_deref(), &bytes(&case["body"]), limits(case)));
        for key in ["fields", "files", "raises"] {
            failures.compare(case, key, &result[key]);
        }
        if case.get("raises").is_none() {
            assert!(result.get("raises").is_none(), "{}: failed with {result}", id(case));
        }
    }
    failures.check("held");
}

#[test]
fn files_over_a_limit() {
    let cases = contract("forms.json");
    let mut failures = Failures::default();
    for case in group(&cases, "file_over") {
        let label = id(case);
        let Some(limits) = case["limits"].as_array() else { panic!("{label}: limits") };
        let form = match read_body(content_type(case).as_deref(), &bytes(&case["body"])) {
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

/// The result of a body read in the given chunks: where it failed and with what, or its parts. The faults of a case
/// of `member` are armed for its call: for `request_body.new`, and for `request_body.feed` for the chunks of a
/// division together, so that the result does not depend on the division (F-24.6).
fn run_body(case: &Value, chunks: &[&[u8]], member: Option<&str>) -> Value {
    let label = id(case);
    let arming = |of: &str| match member {
        Some(member) if member == of => Arming::of(case, member),
        Some(_) | None => Arming::of(&json!({"id": label}), of),
    };
    let content_length = match &case["content_length"] {
        Value::Null => None,
        given => Some(latin1(given)),
    };
    let kind = content_type(case);
    let made = arming("request_body.new");
    let made_body = made.run(|| {
        RequestBody::new(io(), kind.as_deref(), content_length.as_deref(), case["stream"] == json!(true), limits(case))
    });
    made.done();
    let mut read = match made_body {
        Ok(read) => read,
        Err(error) => return json!({"stage": "new", "raises": raised(case, &error)}),
    };
    let directory = read.directory();
    let mut result = json!({});
    let fed = arming("request_body.feed");
    let mut failed = fed.run(|| {
        for chunk in chunks {
            if let Err(error) = read.feed(io(), chunk) {
                return Some(("feed", error));
            }
        }
        None
    });
    fed.done();
    if let Some((_, error)) = &failed {
        // F-9, F-25: every later call gives the same error, `finish` and `parts` included.
        assert_eq!(read.feed(io(), b"more"), Err(error.clone()), "{label}");
        assert_eq!(read.finish(io()), Err(error.clone()), "{label}: finish gives the first failure");
        if read.streamed() {
            assert_eq!(read.parts().err(), Some(error.clone()), "{label}: parts gives the first failure");
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
                            let target = std::env::temp_dir().join(format!("http-form-after-{}", std::process::id()));
                            after
                                .push(file.move_to(io(), &target).map(|()| Vec::new()).map_err(|error| raises(&error)));
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
        result["raises"] = raised(case, &error);
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

/// The scenarios of a body read in chunks, `body`, and the cases of one call of RequestBody: `request_body.new` fails
/// before the body, `request_body.feed` at a chunk (its chunk is the body, read in every division), and
/// `request_body.finish` and `request_body.parts` right after the body is made (G-5).
#[test]
fn bodies() {
    let cases = contract("forms.json");
    let mut failures = Failures::default();
    for (name, stage) in [("body", None), ("request_body.new", Some("new")), ("request_body.feed", Some("feed"))] {
        for case in group(&cases, name) {
            let label = id(case);
            let body = match (case.get("body"), case.get("chunk")) {
                (Some(body), _) | (None, Some(body)) => bytes(body),
                (None, None) => Vec::new(),
            };
            let all = divisions(&body);
            let member = stage.map(|stage| format!("request_body.{stage}"));
            let whole = run_body(case, &all[0], member.as_deref());
            for chunks in &all[1..] {
                let divided = run_body(case, chunks, member.as_deref());
                if divided != whole {
                    failures.push(format!("{label}: {} chunks give {divided}, one chunk {whole}", chunks.len()));
                    break;
                }
            }
            for key in ["raises", "fields", "files", "afterRemoval"] {
                failures.compare(case, key, &whole[key]);
            }
            match stage {
                Some(stage) => failures.compare(&json!({"id": label, "stage": stage}), "stage", &whole["stage"]),
                None => failures.compare(case, "stage", &whole["stage"]),
            }
            if let Some(held) = case.get("held") {
                failures.compare(&json!({"id": label, "held": text(&bytes(held))}), "held", &whole["held"]);
            }
            if case.get("raises").is_none() {
                assert!(whole.get("raises").is_none(), "{label}: failed with {whole}");
            }
            assert!(!string(case, "label").is_empty(), "{label}");
        }
    }
    failures.check("body");
}

/// A RequestBody made from the keys of its constructor (G-5).
fn made(case: &Value) -> RequestBody {
    let kind = content_type(case);
    let content_length = match &case["content_length"] {
        Value::Null => None,
        given => Some(latin1(given)),
    };
    match RequestBody::new(
        io(),
        kind.as_deref(),
        content_length.as_deref(),
        case["stream"] == json!(true),
        limits(case),
    ) {
        Ok(read) => read,
        Err(error) => panic!("{}: {error}", id(case)),
    }
}

/// F-24.4, F-9: finish and parts called on a body that was just made; the properties of a new body.
#[test]
fn one_call() {
    let cases = contract("forms.json");
    let mut failures = Failures::default();
    for case in group(&cases, "request_body.finish") {
        let mut read = made(case);
        failures.compare(case, "raises", &failure(read.finish(io()).err()));
        cleanup(read, case);
    }
    for case in group(&cases, "request_body.parts") {
        let read = made(case);
        failures.compare(case, "raises", &failure(read.parts().err()));
        cleanup(read, case);
    }
    for case in group(&cases, "request_body.properties") {
        let read = made(case);
        failures.compare(
            case,
            "held",
            &json!({"hex": read.held().iter().map(|b| format!("{b:02x}")).collect::<String>()}),
        );
        failures.compare(case, "size", &json!(read.size()));
        failures.compare(case, "streamed", &json!(read.streamed()));
        failures.compare(case, "directory", &json!(read.directory().map(|path| path.display().to_string())));
        cleanup(read, case);
    }
    failures.check("one call");
}

fn failure(error: Option<Error>) -> Value {
    match error {
        Some(error) => raises(&error),
        None => Value::Null,
    }
}

/// F-12.3: a multipart type without a boundary, or with an empty one, fails multipartBoundary with no_boundary.
#[test]
fn boundary() {
    let cases = contract("forms.json");
    let mut failures = Failures::default();
    for case in group(&cases, "boundary") {
        failures.compare(case, "raises", &failure(multipart_boundary(content_type(case).as_deref()).err()));
    }
    failures.check("boundary");
}

fn cleanup(mut read: RequestBody, case: &Value) {
    if let Err(error) = read.remove(io()) {
        panic!("{}: {error}", id(case));
    }
}
