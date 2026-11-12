//! Form values and `multipart/form-data` bodies of hyper (HY-56, HY-57, HY-113, HY-114.9, HY-131), read by
//! one implementation that the PHP, Python and Node servers bind.
//!
//! Every rule is here, the files of a request included (HY-131.13): a binding converts types, gives the bytes that
//! its host reads and turns an error into the class of its runtime, and holds no rule of its own. The rules are in
//! `docs/spec/forms.md`.

#![forbid(unsafe_code)]

mod body;
mod content;
mod error;
mod fields;
mod header;
mod multipart;
mod store;
mod urlencoded;

pub use body::RequestBody;
pub use error::{Error, IoFailure, IoKind};
pub use body::BodyLimits;
pub use fields::{file_over, Collector, Fields, Files, Form};
pub use multipart::{Event, Limits, MultipartReader, Piece, ReadError, HEADER_LIMIT};
pub use store::UploadedFile;

/// The media type of a content type, its type and subtype in lowercase without parameters, or the empty text for a
/// request without `Content-Type` (HY-59.11); HY-59.7 compares it with the form types. A value that is no media type
/// fails with `malformed_content_type` (spec F-12).
pub fn media_type(content_type: Option<&[u8]>) -> Result<String, Error> {
    content::media_type(content_type)
}

/// True when the media type of the content type is `application/x-www-form-urlencoded` (HY-57.10, spec F-12).
pub fn is_urlencoded(content_type: Option<&[u8]>) -> Result<bool, Error> {
    content::is_urlencoded(content_type)
}

/// The boundary of a content type (HY-113.11): `None` for a type other than `multipart/form-data` and for a request
/// without `Content-Type`; for that type its boundary, or `no_boundary` (F-7). A value that is no media type fails
/// with `malformed_content_type` (spec F-12).
pub fn multipart_boundary(content_type: Option<&[u8]>) -> Result<Option<Vec<u8>>, Error> {
    content::boundary(content_type)
}

/// Reads `name=value&name=value` (HY-56): a query, or an `application/x-www-form-urlencoded` body.
pub fn parse_urlencoded(text: &[u8]) -> Result<Fields, Error> {
    urlencoded::parse(text)
}

/// Reads a whole `multipart/form-data` body without limits, the files as values (HY-113, HY-131).
pub fn parse_multipart(body: &[u8], boundary: &[u8]) -> Result<Form, Error> {
    parse_multipart_chunks([body], boundary)
}

/// Reads a `multipart/form-data` body given in chunks without limits; the result does not depend on the chunks
/// (HY-131.6).
pub fn parse_multipart_chunks<'a>(
    chunks: impl IntoIterator<Item = &'a [u8]>,
    boundary: &[u8],
) -> Result<Form, Error> {
    let mut reader = MultipartReader::new(boundary, Limits::default());
    let mut form = Collector::default();
    for chunk in chunks {
        let result = reader.feed_with(chunk, |piece| form.take(piece));
        match result {
            Ok(()) => {}
            Err(ReadError::Form(error) | ReadError::Sink(error)) => return Err(error),
            Err(ReadError::SinkFailed) => return Err(Error::Internal("the body read on after a failure")),
        }
    }
    match reader.finish() {
        Ok(()) => form.into_form(),
        Err(ReadError::Form(error)) => Err(error),
        Err(ReadError::Sink(never)) => match never {},
        Err(ReadError::SinkFailed) => Err(Error::Internal("the body ended after a failure")),
    }
}

/// The fields and the files of a body held as one value, by its content type (HY-57): an urlencoded body has
/// fields, a multipart body has fields and files, and a body of another type or without `Content-Type` has neither
/// (HY-57.4).
pub fn read_body(content_type: Option<&[u8]>, body: &[u8]) -> Result<Form, Error> {
    if is_urlencoded(content_type)? {
        return Ok(Form { fields: parse_urlencoded(body)?, files: Files::new() });
    }
    match multipart_boundary(content_type)? {
        Some(boundary) => parse_multipart(body, &boundary),
        None => Ok(Form::default()),
    }
}

/// The form values of a body held as one value: of an urlencoded body, or the text fields of a multipart body; a body
/// of another type has none (HY-57).
pub fn fields_from_body(content_type: Option<&[u8]>, body: &[u8]) -> Result<Fields, Error> {
    Ok(read_body(content_type, body)?.fields)
}

/// The files of a body held as one value: of a multipart body, and none for a body of another type (HY-113.1).
pub fn files_from_body(content_type: Option<&[u8]>, body: &[u8]) -> Result<Files, Error> {
    match multipart_boundary(content_type)? {
        Some(boundary) => Ok(parse_multipart(body, &boundary)?.files),
        None => Ok(Files::new()),
    }
}
