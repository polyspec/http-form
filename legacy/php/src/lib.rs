//! The PHP binding of http-forms. It converts types and turns an error of the core into a PHP exception; every rule
//! is in the core. A PHP string is bytes, so a content type and a body pass to the core as they are.

#![cfg_attr(windows, feature(abi_vectorcall))]

use std::path::PathBuf;

use ext_php_rs::binary::Binary;
use ext_php_rs::binary_slice::BinarySlice;
use ext_php_rs::boxed::ZBox;
use ext_php_rs::convert::IntoZval;
use ext_php_rs::exception::PhpException;
use ext_php_rs::prelude::*;
use ext_php_rs::types::{ZendClassObject, ZendHashTable, Zval};
use ext_php_rs::zend::{ce, ClassEntry};
use http_forms as core;

fn invalid_argument() -> &'static ClassEntry {
    ClassEntry::try_find("InvalidArgumentException").unwrap_or_else(ce::exception)
}

fn runtime() -> &'static ClassEntry {
    ClassEntry::try_find("RuntimeException").unwrap_or_else(ce::exception)
}

fn file_error(error: core::FileError) -> PhpException {
    match error {
        core::FileError::Io(error) => PhpException::new(error.to_string(), 0, runtime()),
        other => PhpException::new(other.message(), 0, invalid_argument()),
    }
}

fn io_error(error: std::io::Error) -> PhpException {
    PhpException::new(error.to_string(), 0, runtime())
}

fn form_error(error: core::FormError) -> PhpException {
    PhpException::new(format!("{}: {}", error.code(), error), i32::from(error.status()), invalid_argument())
}

fn fields_table(fields: &core::Fields) -> PhpResult<ZBox<ZendHashTable>> {
    let mut pairs = ZendHashTable::new();
    for (name, values) in fields.entries() {
        let mut pair = ZendHashTable::new();
        pair.push(name.as_str())?;
        pair.push(values.clone())?;
        pairs.push(pair)?;
    }
    Ok(pairs)
}

fn files_table(files: core::Files) -> PhpResult<ZBox<ZendHashTable>> {
    let mut pairs = ZendHashTable::new();
    for (name, items) in files.into_entries() {
        let mut list = ZendHashTable::new();
        for file in items {
            list.push(ZendClassObject::new(UploadedFile { inner: file }).into_zval(false)?)?;
        }
        let mut pair = ZendHashTable::new();
        pair.push(name)?;
        pair.push(list)?;
        pairs.push(pair)?;
    }
    Ok(pairs)
}

fn form_table(form: core::Form) -> PhpResult<ZBox<ZendHashTable>> {
    let mut pair = ZendHashTable::new();
    pair.push(fields_table(&form.fields)?)?;
    pair.push(files_table(form.files)?)?;
    Ok(pair)
}

/// Reads `name=value&name=value` (HY-56) as a list of pairs; null when a name or a value is not UTF-8.
#[php_function]
#[php(name = "Polyspec\\Forms\\parse_urlencoded")]
pub fn parse_urlencoded(text: BinarySlice<u8>) -> PhpResult<Option<ZBox<ZendHashTable>>> {
    core::parse_urlencoded(&text).ok().map(|fields| fields_table(&fields)).transpose()
}

/// True when the content type is `application/x-www-form-urlencoded` (HY-57.6).
#[php_function]
#[php(name = "Polyspec\\Forms\\is_urlencoded")]
pub fn is_urlencoded(content_type: BinarySlice<u8>) -> bool {
    core::is_urlencoded(&content_type)
}

/// The boundary of a `multipart/form-data` content type, or null (HY-113.11).
#[php_function]
#[php(name = "Polyspec\\Forms\\multipart_boundary")]
pub fn multipart_boundary(content_type: BinarySlice<u8>) -> Option<Binary<u8>> {
    core::multipart_boundary(&content_type).map(Binary::from)
}

/// The form values of a body as pairs, or null when the body fails with HY-42 (HY-57).
#[php_function]
#[php(name = "Polyspec\\Forms\\fields_from_body")]
pub fn fields_from_body(content_type: BinarySlice<u8>, body: BinarySlice<u8>) -> PhpResult<Option<ZBox<ZendHashTable>>> {
    core::fields_from_body(&content_type, &body).ok().map(|fields| fields_table(&fields)).transpose()
}

/// The files of a body as pairs, or null when the body fails with HY-42 (HY-113).
#[php_function]
#[php(name = "Polyspec\\Forms\\files_from_body")]
pub fn files_from_body(content_type: BinarySlice<u8>, body: BinarySlice<u8>) -> PhpResult<Option<ZBox<ZendHashTable>>> {
    core::files_from_body(&content_type, &body).ok().map(files_table).transpose()
}

/// Reads a multipart body given in chunks as `[fields, files]`, or null when it fails (HY-131.6).
#[php_function]
#[php(name = "Polyspec\\Forms\\parse_multipart_chunks")]
pub fn parse_multipart_chunks(chunks: Vec<Binary<u8>>, boundary: BinarySlice<u8>) -> PhpResult<Option<ZBox<ZendHashTable>>> {
    let chunks: Vec<Vec<u8>> = chunks.into_iter().map(Vec::from).collect();
    match core::parse_multipart_chunks(chunks.iter().map(Vec::as_slice), &boundary) {
        Ok(form) => Ok(Some(form_table(form)?)),
        Err(_) => Ok(None),
    }
}

/// The largest header of a part in bytes (HY-113.16).
#[php_function]
#[php(name = "Polyspec\\Forms\\header_limit")]
pub fn header_limit() -> i64 {
    core::HEADER_LIMIT as i64
}

/// A file that a request sent in a file part (HY-113.3, HY-131.15).
#[php_class]
#[php(name = "Polyspec\\Forms\\UploadedFile")]
pub struct UploadedFile {
    inner: core::UploadedFile,
}

#[php_impl]
impl UploadedFile {
    pub fn name(&self) -> String {
        self.inner.name().to_owned()
    }
    pub fn filename(&self) -> String {
        self.inner.filename().to_owned()
    }
    pub fn content_type(&self) -> String {
        self.inner.content_type().to_owned()
    }
    pub fn size(&self) -> i64 {
        self.inner.size() as i64
    }
    pub fn content(&self) -> PhpResult<Binary<u8>> {
        self.inner.content().map(Binary::from).map_err(file_error)
    }
    pub fn move_to(&mut self, path: &Zval) -> PhpResult<()> {
        let Some(path) = path.str() else {
            return Err(PhpException::new(core::NOT_A_STRING.to_owned(), 0, ce::type_error()));
        };
        self.inner.move_to(&PathBuf::from(path)).map_err(file_error)
    }
}

/// Reads a `multipart/form-data` body in chunks (HY-131); events are `['field', name, value]`,
/// `['file', name, filename, contentType]`, `['data', bytes]` and `['end']`.
#[php_class]
#[php(name = "Polyspec\\Forms\\MultipartReader")]
pub struct MultipartReader {
    inner: core::MultipartReader,
}

#[php_impl]
impl MultipartReader {
    pub fn __construct(
        boundary: BinarySlice<u8>,
        value_limit: Option<i64>,
        part_limit: Option<i64>,
        file_limit: Option<i64>,
    ) -> Self {
        let limits = core::Limits {
            value: value_limit.map(|v| v.max(0) as u64),
            parts: part_limit.map(|v| v.max(0) as u64),
            file: file_limit.map(|v| v.max(0) as u64),
        };
        MultipartReader { inner: core::MultipartReader::new(&boundary, limits) }
    }

    pub fn feed(&mut self, chunk: BinarySlice<u8>) -> PhpResult<ZBox<ZendHashTable>> {
        let events = self.inner.feed(&chunk).map_err(form_error)?;
        let mut items = ZendHashTable::new();
        for event in events {
            let mut item = ZendHashTable::new();
            match event {
                core::Event::Field { name, value } => {
                    item.push("field")?;
                    item.push(name)?;
                    item.push(value)?;
                }
                core::Event::FileStart { name, filename, content_type } => {
                    item.push("file")?;
                    item.push(name)?;
                    item.push(filename)?;
                    item.push(content_type)?;
                }
                core::Event::FileData(bytes) => {
                    item.push("data")?;
                    item.push(Binary::from(bytes))?;
                }
                core::Event::FileEnd => item.push("end")?,
            }
            items.push(item)?;
        }
        Ok(items)
    }

    pub fn finish(&mut self) -> PhpResult<()> {
        self.inner.finish().map_err(form_error)
    }

    pub fn closed(&self) -> bool {
        self.inner.closed()
    }

    pub fn buffered(&self) -> i64 {
        self.inner.buffered() as i64
    }
}

/// The body of a request as its host reads it (HY-59, HY-131).
#[php_class]
#[php(name = "Polyspec\\Forms\\RequestBody")]
pub struct RequestBody {
    inner: core::RequestBody,
}

#[php_impl]
impl RequestBody {
    pub fn __construct(
        content_type: BinarySlice<u8>,
        limit: i64,
        stream: bool,
        value_limit: i64,
        part_limit: i64,
        file_limit: i64,
    ) -> PhpResult<Self> {
        let inner = core::RequestBody::new(
            &content_type,
            limit.max(0) as u64,
            stream,
            value_limit.max(0) as u64,
            part_limit.max(0) as u64,
            file_limit.max(0) as u64,
        )
        .map_err(io_error)?;
        Ok(RequestBody { inner })
    }

    pub fn feed(&mut self, chunk: BinarySlice<u8>) -> PhpResult<bool> {
        self.inner.feed(&chunk).map_err(io_error)
    }

    pub fn exceeded(&self) -> bool {
        self.inner.exceeded()
    }

    pub fn size(&self) -> i64 {
        self.inner.size() as i64
    }

    pub fn held(&self) -> Binary<u8> {
        Binary::from(self.inner.held().to_vec())
    }

    pub fn streamed(&self) -> bool {
        self.inner.streamed()
    }

    pub fn parts(&self) -> PhpResult<Option<ZBox<ZendHashTable>>> {
        self.inner.parts().map(form_table).transpose()
    }

    pub fn directory(&self) -> Option<String> {
        self.inner.directory().map(|path| path.to_string_lossy().into_owned())
    }

    pub fn remove(&mut self) -> PhpResult<()> {
        self.inner.remove().map_err(io_error)
    }
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .class::<UploadedFile>()
        .class::<MultipartReader>()
        .class::<RequestBody>()
        .function(wrap_function!(parse_urlencoded))
        .function(wrap_function!(is_urlencoded))
        .function(wrap_function!(multipart_boundary))
        .function(wrap_function!(fields_from_body))
        .function(wrap_function!(files_from_body))
        .function(wrap_function!(parse_multipart_chunks))
        .function(wrap_function!(header_limit))
}
