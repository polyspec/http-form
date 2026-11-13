//! Form values and `multipart/form-data` bodies (F-20, F-21, F-22, F-27, F-24), read by
//! one implementation that the PHP, Python and Node servers bind.
//!
//! Every rule is here, the files of a request included (F-10): a binding converts types, gives the bytes that
//! its host reads and turns an error into the class of its runtime, and holds no rule of its own. The rules are in
//! `docs/spec/forms.md`.

#![forbid(unsafe_code)]

mod body;
mod content;
mod error;
pub mod fault;
mod fields;
mod header;
mod multipart;
mod store;
mod urlencoded;

pub use body::BodyLimits;
pub use body::RequestBody;
pub use error::{Error, IoFailure, IoKind};
pub use fields::{file_over, Collector, Fields, Files, Form};
pub use multipart::{Event, Limits, MultipartReader, Piece, ReadError, HEADER_LIMIT};
pub use store::UploadedFile;

/// The media type of a content type, its type and subtype in lowercase without parameters, or the empty text for a
/// request without `Content-Type` (F-12.1); a server compares it with its form types. A value that is no media type
/// fails with `malformed_content_type` (spec F-12).
pub fn media_type(content_type: Option<&[u8]>) -> Result<String, Error> {
    content::media_type(content_type)
}

/// True when the media type of the content type is `application/x-www-form-urlencoded` (F-12.2, spec F-12).
pub fn is_urlencoded(content_type: Option<&[u8]>) -> Result<bool, Error> {
    content::is_urlencoded(content_type)
}

/// The boundary of a content type (F-12.3): `None` for a type other than `multipart/form-data` and for a request
/// without `Content-Type`; for that type its boundary, or `no_boundary` (F-7). A value that is no media type fails
/// with `malformed_content_type` (spec F-12).
pub fn multipart_boundary(content_type: Option<&[u8]>) -> Result<Option<Vec<u8>>, Error> {
    content::boundary(content_type)
}

/// Reads `name=value&name=value` (F-20): a query, or an `application/x-www-form-urlencoded` body.
pub fn parse_urlencoded(text: &[u8]) -> Result<Fields, Error> {
    urlencoded::parse(text)
}

/// Reads a whole `multipart/form-data` body without limits, the files as values (F-22, F-24).
pub fn parse_multipart(body: &[u8], boundary: &[u8]) -> Result<Form, Error> {
    parse_multipart_chunks([body], boundary)
}

/// Reads a `multipart/form-data` body given in chunks without limits; the result does not depend on the chunks
/// (F-24.6).
pub fn parse_multipart_chunks<'a>(chunks: impl IntoIterator<Item = &'a [u8]>, boundary: &[u8]) -> Result<Form, Error> {
    read_multipart(chunks, boundary, Limits::default())
}

/// Reads a `multipart/form-data` body in chunks with the limits of a reader.
fn read_multipart<'a>(
    chunks: impl IntoIterator<Item = &'a [u8]>,
    boundary: &[u8],
    limits: Limits,
) -> Result<Form, Error> {
    let mut reader = MultipartReader::new(boundary, limits);
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

/// The fields and the files of a body held as one value, by its content type (F-21): an urlencoded body has
/// fields, a multipart body has fields and files, and a body of another type or without `Content-Type` has neither
/// (F-21.4).
pub fn read_body(content_type: Option<&[u8]>, body: &[u8]) -> Result<Form, Error> {
    if is_urlencoded(content_type)? {
        return Ok(Form { fields: parse_urlencoded(body)?, files: Files::new() });
    }
    match multipart_boundary(content_type)? {
        Some(boundary) => parse_multipart(body, &boundary),
        None => Ok(Form::default()),
    }
}

/// The fields and the files of a body held as one value with the limits of a body (F-25.6): the body limit and the
/// value limit bound its bytes, as they bound a body that `RequestBody` holds (at one byte the body limit comes first),
/// and the part limit, the file limit and the value limit bound the parts, the files and the text values that a
/// `multipart/form-data` body gives. Its files are values.
pub fn read_held_body(content_type: Option<&[u8]>, body: &[u8], limits: BodyLimits) -> Result<Form, Error> {
    let length = multipart::count(body.len())?;
    let over_body = length > limits.limit;
    let over_value = length > limits.value_limit;
    if over_body && (!over_value || limits.limit <= limits.value_limit) {
        return Err(Error::BodyLimit);
    }
    if over_value {
        return Err(Error::ValueLimit);
    }
    if is_urlencoded(content_type)? {
        return Ok(Form { fields: parse_urlencoded(body)?, files: Files::new() });
    }
    match multipart_boundary(content_type)? {
        Some(boundary) => {
            let reader_limits = Limits {
                value: Some(limits.value_limit),
                parts: Some(limits.part_limit),
                file: Some(limits.file_limit),
            };
            read_multipart([body], &boundary, reader_limits)
        }
        None => Ok(Form::default()),
    }
}

/// The form values of a body held as one value: of an urlencoded body, or the text fields of a multipart body; a body
/// of another type has none (F-21).
pub fn fields_from_body(content_type: Option<&[u8]>, body: &[u8]) -> Result<Fields, Error> {
    Ok(read_body(content_type, body)?.fields)
}

/// The files of a body held as one value: of a multipart body, and none for a body of another type (F-22.1).
pub fn files_from_body(content_type: Option<&[u8]>, body: &[u8]) -> Result<Files, Error> {
    match multipart_boundary(content_type)? {
        Some(boundary) => Ok(parse_multipart(body, &boundary)?.files),
        None => Ok(Files::new()),
    }
}
