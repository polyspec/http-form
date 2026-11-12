//! Form values and `multipart/form-data` bodies of hyper (HY-56, HY-57, HY-113, HY-131), read by one implementation
//! that the PHP, Python and Node servers bind.
//!
//! Every rule is here, the files of a request included (HY-131.13): a binding converts types, gives the bytes that
//! its host reads and turns an error into the class of its runtime, and holds no rule of its own.

mod body;
mod fields;
mod header;
mod multipart;
mod store;
mod urlencoded;

pub use body::RequestBody;
pub use fields::{Fields, Files, Form, Named};
pub use multipart::{Event, Limits, MultipartReader, Piece, ReadError, HEADER_LIMIT};
pub use store::{FileError, UploadedFile, EXISTS, MOVED, NOT_A_STRING};

use std::fmt;

/// The limit that a body passed (HY-131.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limit {
    Value,
    Parts,
    File,
}

/// A failure to read a form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormError {
    /// The body fails with HY-42: a name, a value, a filename or a content type that is not UTF-8, or a multipart
    /// body of another form (HY-113.15, HY-131.5). A server answers 400.
    Invalid,
    /// The body passed a limit (HY-131.11). A server answers 413.
    Exceeded(Limit),
}

impl FormError {
    /// The stable code of the error, which every binding gives as it is.
    pub fn code(&self) -> &'static str {
        match self {
            FormError::Invalid => "invalid",
            FormError::Exceeded(Limit::Value) => "value_limit",
            FormError::Exceeded(Limit::Parts) => "part_limit",
            FormError::Exceeded(Limit::File) => "file_limit",
        }
    }

    /// The status that a server answers.
    pub fn status(&self) -> u16 {
        match self {
            FormError::Invalid => 400,
            FormError::Exceeded(_) => 413,
        }
    }
}

impl fmt::Display for FormError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormError::Invalid => write!(f, "the form fails with HY-42"),
            FormError::Exceeded(Limit::Value) => write!(f, "the text values passed the value limit"),
            FormError::Exceeded(Limit::Parts) => write!(f, "the body passed the part limit"),
            FormError::Exceeded(Limit::File) => write!(f, "a file passed the file limit"),
        }
    }
}

impl std::error::Error for FormError {}

/// Reads `name=value&name=value` (HY-56): a query, or an `application/x-www-form-urlencoded` body.
pub fn parse_urlencoded(text: &[u8]) -> Result<Fields, FormError> {
    urlencoded::parse(text)
}

/// True when the content type is `application/x-www-form-urlencoded` (HY-57.6).
pub fn is_urlencoded(content_type: &[u8]) -> bool {
    header::is_urlencoded(content_type)
}

/// The boundary of a `multipart/form-data` content type, or `None` (HY-113.11).
pub fn multipart_boundary(content_type: &[u8]) -> Option<Vec<u8>> {
    header::boundary(content_type)
}

/// Reads a whole `multipart/form-data` body without limits, the files as values (HY-113, HY-131).
pub fn parse_multipart(body: &[u8], boundary: &[u8]) -> Result<Form, FormError> {
    parse_multipart_chunks([body], boundary)
}

/// The form values of a body: of an urlencoded body, or the text fields of a multipart body; a body of another type
/// has none (HY-57).
pub fn fields_from_body(content_type: &[u8], body: &[u8]) -> Result<Fields, FormError> {
    if is_urlencoded(content_type) {
        return parse_urlencoded(body);
    }
    match multipart_boundary(content_type) {
        Some(boundary) => Ok(parse_multipart(body, &boundary)?.fields),
        None => Ok(Fields::default()),
    }
}

/// The files of a multipart body, and none for a body of another type (HY-113).
pub fn files_from_body(content_type: &[u8], body: &[u8]) -> Result<Files, FormError> {
    match multipart_boundary(content_type) {
        Some(boundary) => Ok(parse_multipart(body, &boundary)?.files),
        None => Ok(Files::default()),
    }
}

/// Reads a `multipart/form-data` body given in chunks without limits; the result does not depend on the chunks
/// (HY-131.6).
pub fn parse_multipart_chunks<'a>(
    chunks: impl IntoIterator<Item = &'a [u8]>,
    boundary: &[u8],
) -> Result<Form, FormError> {
    let mut reader = MultipartReader::new(boundary, Limits::default());
    let mut form = Form::default();
    for chunk in chunks {
        reader
            .feed_with(chunk, |piece| {
                form.take(piece);
                Ok::<(), std::convert::Infallible>(())
            })
            .map_err(|error| match error {
                ReadError::Form(error) => error,
                ReadError::Sink(never) => match never {},
            })?;
    }
    reader.finish()?;
    Ok(form)
}

/// The fields and the files of a body by its content type: an urlencoded body has fields, a multipart body has fields
/// and files, and a body of another type has neither (HY-57).
pub fn read_body(content_type: &[u8], body: &[u8]) -> Result<Form, FormError> {
    if is_urlencoded(content_type) {
        return Ok(Form::new(parse_urlencoded(body)?, Files::default()));
    }
    match multipart_boundary(content_type) {
        Some(boundary) => parse_multipart(body, &boundary),
        None => Ok(Form::default()),
    }
}
