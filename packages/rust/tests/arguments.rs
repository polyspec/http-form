//! The arguments of the core functions (spec F-18.1): the generated code passes a `text` as `&str`, `bytes` as `&[u8]`
//! and a record as a reference (binding GN-6), and the contract has no form of an argument that is passed by value, so
//! each function of the contract takes its arguments borrowed. These calls are the calls of the generated code; a core
//! function that takes one of them by value does not compile here.
use binding_core::Io;
use polyspec_http_form::{read_held_body, BodyLimits, RequestBody, UploadedFile};

/// The wait token of one call that does blocking I/O (spec F-17), as generated code makes it.
fn io() -> Io {
    Io::for_generated_call()
}

const LIMITS: BodyLimits = BodyLimits { limit: 1 << 20, value_limit: 1 << 20, part_limit: 16, file_limit: 1 << 20 };

#[test]
fn read_held_body_takes_the_limits_by_reference() {
    let limits = LIMITS;
    let form = match read_held_body(Some(b"application/x-www-form-urlencoded"), b"a=1", &limits) {
        Ok(form) => form,
        Err(error) => panic!("read_held_body failed: {error:?}"),
    };
    assert_eq!(form.fields, vec![("a".to_owned(), vec!["1".to_owned()])]);
    assert_eq!(limits, LIMITS, "the caller keeps its limits");
}

#[test]
fn start_takes_the_limits_by_reference() {
    let limits = LIMITS;
    let mut body = match RequestBody::start(io(), Some(b"application/x-www-form-urlencoded"), None, false, &limits) {
        Ok(body) => body,
        Err(error) => panic!("RequestBody::start failed: {error:?}"),
    };
    if let Err(error) = body.remove(io()) {
        panic!("remove failed: {error:?}");
    }
    assert_eq!(limits, LIMITS, "the caller keeps its limits");
}

#[test]
fn from_value_borrows_its_texts_and_its_bytes() {
    let (name, filename, content_type, content) = ("f", "f.txt", "text/plain", b"abc".as_slice());
    let file = match UploadedFile::from_value(name, filename, content_type, content) {
        Ok(file) => file,
        Err(error) => panic!("UploadedFile::from_value failed: {error:?}"),
    };
    assert_eq!(
        (file.name(), file.filename(), file.content_type(), file.size()),
        (name.to_owned(), filename.to_owned(), content_type.to_owned(), 3)
    );
    match file.content(io()) {
        Ok(bytes) => assert_eq!(bytes, content),
        Err(error) => panic!("content failed: {error:?}"),
    }
}
