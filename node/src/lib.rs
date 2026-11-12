//! The Node binding of http-forms. It converts types and turns an error of the core into a JavaScript error; every
//! rule is in the core. `RequestBody.feed` reads a chunk and writes its files on a thread of the libuv pool and returns
//! a promise, so that a write does not stop the other requests of the server (HY-131).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use http_forms as core;
use napi::bindgen_prelude::*;
use napi::{Env, Task};
use napi_derive::napi;

/// The bytes of a text whose characters are bytes, as `node:http` gives a header.
fn bytes_of(env: &Env, text: &str) -> Result<Vec<u8>> {
    match text.chars().map(|c| u8::try_from(u32::from(c))).collect::<std::result::Result<Vec<u8>, _>>() {
        Ok(bytes) => Ok(bytes),
        Err(_) => Err(type_error(env, "a header has a character above U+00FF")),
    }
}

fn text_of(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}

fn pending() -> Error {
    Error::new(Status::PendingException, "")
}

fn type_error(env: &Env, message: &str) -> Error {
    match env.throw_type_error(message, None) {
        Ok(()) => pending(),
        Err(error) => error,
    }
}

fn range_error(env: &Env, message: &str) -> Error {
    match env.throw_range_error(message, None) {
        Ok(()) => pending(),
        Err(error) => error,
    }
}

fn form_error(error: core::FormError) -> Error {
    Error::new(Status::GenericFailure, format!("{}: {}", error.code(), error))
}

fn file_error(env: &Env, error: core::FileError) -> Error {
    match error {
        core::FileError::Io(error) => Error::from_reason(error.to_string()),
        other => range_error(env, &other.message()),
    }
}

fn io_error(error: std::io::Error) -> Error {
    Error::from_reason(error.to_string())
}

fn fields_array<'e>(env: &'e Env, fields: &core::Fields) -> Result<Array<'e>> {
    let mut pairs = env.create_array(fields.len() as u32)?;
    for (index, (name, values)) in fields.entries().iter().enumerate() {
        let mut pair = env.create_array(2)?;
        pair.set(0, name.as_str())?;
        pair.set(1, values.clone())?;
        pairs.set(index as u32, pair)?;
    }
    Ok(pairs)
}

fn files_array<'e>(env: &'e Env, files: core::Files) -> Result<Array<'e>> {
    let mut pairs = env.create_array(files.len() as u32)?;
    for (index, (name, items)) in files.into_entries().into_iter().enumerate() {
        let mut list = env.create_array(items.len() as u32)?;
        for (position, file) in items.into_iter().enumerate() {
            list.set(position as u32, UploadedFile { inner: file }.into_instance(env)?)?;
        }
        let mut pair = env.create_array(2)?;
        pair.set(0, name)?;
        pair.set(1, list)?;
        pairs.set(index as u32, pair)?;
    }
    Ok(pairs)
}

fn form_array<'e>(env: &'e Env, form: core::Form) -> Result<Array<'e>> {
    let mut pair = env.create_array(2)?;
    pair.set(0, fields_array(env, &form.fields)?)?;
    pair.set(1, files_array(env, form.files)?)?;
    Ok(pair)
}

/// Reads `name=value&name=value` (HY-56); null when a name or a value is not UTF-8.
#[napi(js_name = "parseUrlencoded", ts_return_type = "Array<[string, string[]]> | null")]
pub fn parse_urlencoded<'e>(env: &'e Env, text: Buffer) -> Result<Option<Array<'e>>> {
    core::parse_urlencoded(&text).ok().map(|fields| fields_array(env, &fields)).transpose()
}

/// True when the content type is `application/x-www-form-urlencoded` (HY-57.6).
#[napi(js_name = "isUrlencoded")]
pub fn is_urlencoded(env: Env, content_type: String) -> Result<bool> {
    Ok(core::is_urlencoded(&bytes_of(&env, &content_type)?))
}

/// The boundary of a `multipart/form-data` content type, or null (HY-113.11).
#[napi(js_name = "multipartBoundary")]
pub fn multipart_boundary(env: Env, content_type: String) -> Result<Option<String>> {
    Ok(core::multipart_boundary(&bytes_of(&env, &content_type)?).map(|b| text_of(&b)))
}

/// The form values of a body as pairs, or null when the body fails with HY-42 (HY-57).
#[napi(js_name = "fieldsFromBody", ts_return_type = "Array<[string, string[]]> | null")]
pub fn fields_from_body<'e>(env: &'e Env, content_type: String, body: Buffer) -> Result<Option<Array<'e>>> {
    let kind = bytes_of(env, &content_type)?;
    core::fields_from_body(&kind, &body).ok().map(|fields| fields_array(env, &fields)).transpose()
}

/// The files of a body as pairs, or null when the body fails with HY-42 (HY-113).
#[napi(js_name = "filesFromBody", ts_return_type = "Array<[string, UploadedFile[]]> | null")]
pub fn files_from_body<'e>(env: &'e Env, content_type: String, body: Buffer) -> Result<Option<Array<'e>>> {
    let kind = bytes_of(env, &content_type)?;
    core::files_from_body(&kind, &body).ok().map(|files| files_array(env, files)).transpose()
}

/// Reads a multipart body given in chunks as `[fields, files]`, or null when it fails (HY-131.6).
#[napi(js_name = "parseMultipartChunks", ts_return_type = "[Array<[string, string[]]>, Array<[string, UploadedFile[]]>] | null")]
pub fn parse_multipart_chunks<'e>(env: &'e Env, chunks: Vec<Buffer>, boundary: String) -> Result<Option<Array<'e>>> {
    let boundary = bytes_of(env, &boundary)?;
    match core::parse_multipart_chunks(chunks.iter().map(|chunk| &chunk[..]), &boundary) {
        Ok(form) => Ok(Some(form_array(env, form)?)),
        Err(_) => Ok(None),
    }
}

/// The largest header of a part in bytes (HY-113.16).
#[napi(js_name = "HEADER_LIMIT")]
pub const HEADER_LIMIT: u32 = core::HEADER_LIMIT as u32;

/// A file that a request sent in a file part (HY-113.3, HY-131.15).
#[napi]
pub struct UploadedFile {
    inner: core::UploadedFile,
}

#[napi]
impl UploadedFile {
    #[napi]
    pub fn name(&self) -> String {
        self.inner.name().to_owned()
    }
    #[napi]
    pub fn filename(&self) -> String {
        self.inner.filename().to_owned()
    }
    #[napi(js_name = "contentType")]
    pub fn content_type(&self) -> String {
        self.inner.content_type().to_owned()
    }
    #[napi]
    pub fn size(&self) -> f64 {
        self.inner.size() as f64
    }
    #[napi]
    pub fn content(&self, env: Env) -> Result<Buffer> {
        self.inner.content().map(Buffer::from).map_err(|error| file_error(&env, error))
    }
    #[napi(js_name = "moveTo")]
    pub fn move_to(&mut self, env: Env, path: Unknown) -> Result<()> {
        if path.get_type()? != ValueType::String {
            return Err(type_error(&env, core::NOT_A_STRING));
        }
        let path: String = unsafe { path.cast()? };
        self.inner.move_to(&PathBuf::from(path)).map_err(|error| file_error(&env, error))
    }
}

/// Reads a `multipart/form-data` body in chunks (HY-131); events are `["field", name, value]`,
/// `["file", name, filename, contentType]`, `["data", bytes]` and `["end"]`.
#[napi]
pub struct MultipartReader {
    inner: core::MultipartReader,
}

#[napi]
impl MultipartReader {
    #[napi(constructor)]
    pub fn new(
        env: Env,
        boundary: String,
        value_limit: Option<f64>,
        part_limit: Option<f64>,
        file_limit: Option<f64>,
    ) -> Result<Self> {
        let limits = core::Limits {
            value: value_limit.map(|v| v as u64),
            parts: part_limit.map(|v| v as u64),
            file: file_limit.map(|v| v as u64),
        };
        Ok(MultipartReader { inner: core::MultipartReader::new(&bytes_of(&env, &boundary)?, limits) })
    }

    #[napi(ts_return_type = "Array<['field', string, string] | ['file', string, string, string] | ['data', Buffer] | ['end']>")]
    pub fn feed<'e>(&mut self, env: &'e Env, chunk: Buffer) -> Result<Array<'e>> {
        let events = self.inner.feed(&chunk).map_err(form_error)?;
        let mut items = env.create_array(events.len() as u32)?;
        for (index, event) in events.into_iter().enumerate() {
            let mut item = env.create_array(0)?;
            match event {
                core::Event::Field { name, value } => {
                    item.insert("field")?;
                    item.insert(name)?;
                    item.insert(value)?;
                }
                core::Event::FileStart { name, filename, content_type } => {
                    item.insert("file")?;
                    item.insert(name)?;
                    item.insert(filename)?;
                    item.insert(content_type)?;
                }
                core::Event::FileData(bytes) => {
                    item.insert("data")?;
                    item.insert(Buffer::from(bytes))?;
                }
                core::Event::FileEnd => item.insert("end")?,
            }
            items.set(index as u32, item)?;
        }
        Ok(items)
    }

    #[napi]
    pub fn finish(&mut self) -> Result<()> {
        self.inner.finish().map_err(form_error)
    }

    #[napi]
    pub fn closed(&self) -> bool {
        self.inner.closed()
    }

    #[napi]
    pub fn buffered(&self) -> u32 {
        self.inner.buffered() as u32
    }
}

/// Reads one chunk of a body on a thread of the libuv pool.
pub struct Feed {
    body: Arc<Mutex<core::RequestBody>>,
    chunk: Vec<u8>,
}

impl Task for Feed {
    type Output = bool;
    type JsValue = bool;

    fn compute(&mut self) -> Result<bool> {
        let mut body = self.body.lock().map_err(|_| Error::from_reason("a body failed on another thread"))?;
        body.feed(&self.chunk).map_err(io_error)
    }

    fn resolve(&mut self, _: Env, output: bool) -> Result<bool> {
        Ok(output)
    }
}

/// The body of a request as its host reads it (HY-59, HY-131).
#[napi]
pub struct RequestBody {
    inner: Arc<Mutex<core::RequestBody>>,
}

impl RequestBody {
    fn with<T>(&self, work: impl FnOnce(&mut core::RequestBody) -> T) -> Result<T> {
        let mut body = self.inner.lock().map_err(|_| Error::from_reason("a body failed on another thread"))?;
        Ok(work(&mut body))
    }
}

#[napi]
impl RequestBody {
    #[napi(constructor)]
    pub fn new(
        env: Env,
        content_type: String,
        limit: f64,
        stream: bool,
        value_limit: f64,
        part_limit: f64,
        file_limit: f64,
    ) -> Result<Self> {
        let kind = bytes_of(&env, &content_type)?;
        let body = core::RequestBody::new(&kind, limit as u64, stream, value_limit as u64, part_limit as u64, file_limit as u64)
            .map_err(io_error)?;
        Ok(RequestBody { inner: Arc::new(Mutex::new(body)) })
    }

    /// Reads the next chunk on the libuv pool; the promise is false when the body passed a limit.
    #[napi]
    pub fn feed(&self, chunk: Buffer) -> AsyncTask<Feed> {
        AsyncTask::new(Feed { body: Arc::clone(&self.inner), chunk: chunk.to_vec() })
    }

    /// Reads the next chunk on the calling thread.
    #[napi(js_name = "feedSync")]
    pub fn feed_sync(&self, chunk: Buffer) -> Result<bool> {
        self.with(|body| body.feed(&chunk))?.map_err(io_error)
    }

    #[napi]
    pub fn exceeded(&self) -> Result<bool> {
        self.with(|body| body.exceeded())
    }

    #[napi]
    pub fn size(&self) -> Result<f64> {
        self.with(|body| body.size() as f64)
    }

    #[napi]
    pub fn held(&self) -> Result<Buffer> {
        self.with(|body| Buffer::from(body.held().to_vec()))
    }

    #[napi]
    pub fn streamed(&self) -> Result<bool> {
        self.with(|body| body.streamed())
    }

    #[napi(ts_return_type = "[Array<[string, string[]]>, Array<[string, UploadedFile[]]>] | null")]
    pub fn parts<'e>(&self, env: &'e Env) -> Result<Option<Array<'e>>> {
        match self.with(|body| body.parts())? {
            Some(form) => Ok(Some(form_array(env, form)?)),
            None => Ok(None),
        }
    }

    #[napi]
    pub fn directory(&self) -> Result<Option<String>> {
        self.with(|body| body.directory().map(|path| path.to_string_lossy().into_owned()))
    }

    #[napi]
    pub fn remove(&self) -> Result<()> {
        self.with(|body| body.remove())?.map_err(io_error)
    }
}
