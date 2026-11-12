//! The core side of `conformance/differential.py`: reads one JSON object per line, `{"type": <hex>, "body": <hex>,
//! "cuts": [<index>...]}`, and writes one per line, `{"whole": [fields, files], "chunked": [fields, files] | null}`,
//! where a failure is `{"raises": <kind>}` and a file is `[filename, content type, <hex of its bytes>]`.

use std::io::{self, BufRead, Write};

use polyspec_http_forms::{fields_from_body, files_from_body, multipart_boundary, parse_multipart_chunks, Fields, Files};
use serde_json::{json, Value};

/// The wait token of one call that does blocking I/O (spec F-17), as generated code makes it.
fn io() -> bindgen_core::Io {
    bindgen_core::Io::for_generated_call()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|at| match u8::from_str_radix(&text[at..at + 2], 16) {
            Ok(byte) => byte,
            Err(error) => panic!("{error}"),
        })
        .collect()
}

fn fields(fields: &Fields) -> Value {
    Value::Array(fields.iter().map(|(name, values)| json!([name, values])).collect())
}

fn files(files: &Files) -> Value {
    Value::Array(
        files
            .iter()
            .map(|(name, items)| {
                let items: Vec<Value> = items
                    .iter()
                    .map(|file| match file.content(io()) {
                        Ok(bytes) => json!([file.filename(), file.content_type(), hex(&bytes)]),
                        Err(error) => panic!("{error}"),
                    })
                    .collect();
                json!([name, items])
            })
            .collect(),
    )
}

fn raised(kind: &str) -> Value {
    json!({ "raises": kind })
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        let input: Value = match serde_json::from_str(&line) {
            Ok(input) => input,
            Err(error) => panic!("{error}"),
        };
        let kind = input["type"].as_str().map(unhex);
        let kind = kind.as_deref();
        let Some(body) = input["body"].as_str().map(unhex) else { panic!("body") };
        let Some(cuts) = input["cuts"].as_array() else { panic!("cuts") };
        let whole = json!([
            match fields_from_body(kind, &body) {
                Ok(read) => fields(&read),
                Err(error) => raised(error.kind()),
            },
            match files_from_body(kind, &body) {
                Ok(read) => files(&read),
                Err(error) => raised(error.kind()),
            },
        ]);
        let chunked = match multipart_boundary(kind) {
            Ok(Some(boundary)) => {
                let mut chunks = Vec::new();
                let mut at = 0;
                for cut in cuts {
                    let cut = match cut.as_u64().map(usize::try_from) {
                        Some(Ok(cut)) => cut,
                        Some(Err(_)) | None => panic!("cut {cut}"),
                    };
                    chunks.push(&body[at..cut]);
                    at = cut;
                }
                chunks.push(&body[at..]);
                match parse_multipart_chunks(chunks, &boundary) {
                    Ok(form) => json!([fields(&form.fields), files(&form.files)]),
                    Err(error) => json!([raised(error.kind()), raised(error.kind())]),
                }
            }
            Ok(None) | Err(_) => Value::Null,
        };
        writeln!(stdout, "{}", json!({"whole": whole, "chunked": chunked}))?;
    }
    Ok(())
}
