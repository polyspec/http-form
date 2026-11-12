//! The Python binding of http-forms. It converts types and turns an error of the core into a Python exception; every
//! rule is in the core.

use std::path::PathBuf;

use http_forms as core;
use pyo3::create_exception;
use pyo3::exceptions::{PyOSError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList, PyString, PyTuple};

create_exception!(_http_forms, FormError, PyValueError, "A form that fails: args are the code, the status and the message.");

fn form_error(error: core::FormError) -> PyErr {
    FormError::new_err((error.code(), error.status(), error.to_string()))
}

fn file_error(error: core::FileError) -> PyErr {
    match error {
        core::FileError::Io(error) => PyOSError::new_err(error.to_string()),
        other => PyValueError::new_err(other.message()),
    }
}

/// The bytes of a text whose characters are bytes, as a host gives a header.
fn bytes_of(text: &str) -> PyResult<Vec<u8>> {
    text.chars()
        .map(|c| u8::try_from(u32::from(c)).map_err(|_| PyTypeError::new_err("a header has a character above U+00FF")))
        .collect()
}

fn text_of(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}

fn fields_list<'py>(py: Python<'py>, fields: &core::Fields) -> PyResult<Bound<'py, PyList>> {
    let pairs = fields
        .entries()
        .iter()
        .map(|(name, values)| PyTuple::new(py, [name.into_pyobject(py)?.into_any(), PyList::new(py, values)?.into_any()]))
        .collect::<PyResult<Vec<_>>>()?;
    PyList::new(py, pairs)
}

fn files_list<'py>(py: Python<'py>, files: core::Files) -> PyResult<Bound<'py, PyList>> {
    let mut pairs = Vec::new();
    for (name, items) in files.into_entries() {
        let items = items.into_iter().map(|file| Py::new(py, UploadedFile { inner: file })).collect::<PyResult<Vec<_>>>()?;
        pairs.push(PyTuple::new(py, [name.into_pyobject(py)?.into_any(), PyList::new(py, items)?.into_any()])?);
    }
    PyList::new(py, pairs)
}

/// Reads ``name=value&name=value`` (HY-56); None when a name or a value is not UTF-8.
#[pyfunction]
fn parse_urlencoded<'py>(py: Python<'py>, text: &[u8]) -> PyResult<Option<Bound<'py, PyList>>> {
    core::parse_urlencoded(text).ok().map(|fields| fields_list(py, &fields)).transpose()
}

/// True when the content type is ``application/x-www-form-urlencoded`` (HY-57.6).
#[pyfunction]
fn is_urlencoded(content_type: &str) -> PyResult<bool> {
    Ok(core::is_urlencoded(&bytes_of(content_type)?))
}

/// The boundary of a ``multipart/form-data`` content type, or None (HY-113.11).
#[pyfunction]
fn multipart_boundary(content_type: &str) -> PyResult<Option<String>> {
    Ok(core::multipart_boundary(&bytes_of(content_type)?).map(|b| text_of(&b)))
}

/// The form values of a body as pairs, or None when the body fails with HY-42 (HY-57).
#[pyfunction]
fn fields_from_body<'py>(py: Python<'py>, content_type: &str, body: &[u8]) -> PyResult<Option<Bound<'py, PyList>>> {
    core::fields_from_body(&bytes_of(content_type)?, body).ok().map(|fields| fields_list(py, &fields)).transpose()
}

/// The files of a body as pairs, or None when the body fails with HY-42 (HY-113).
#[pyfunction]
fn files_from_body<'py>(py: Python<'py>, content_type: &str, body: &[u8]) -> PyResult<Option<Bound<'py, PyList>>> {
    core::files_from_body(&bytes_of(content_type)?, body).ok().map(|files| files_list(py, files)).transpose()
}

/// Reads a multipart body given in chunks as ``(fields, files)``, or None when it fails (HY-131.6).
#[pyfunction]
fn parse_multipart_chunks<'py>(py: Python<'py>, chunks: Vec<Vec<u8>>, boundary: &str) -> PyResult<Option<Bound<'py, PyTuple>>> {
    let Ok(form) = core::parse_multipart_chunks(chunks.iter().map(Vec::as_slice), &bytes_of(boundary)?) else {
        return Ok(None);
    };
    let fields = fields_list(py, &form.fields)?.into_any();
    let files = files_list(py, form.files)?.into_any();
    Ok(Some(PyTuple::new(py, [fields, files])?))
}

/// A file that a request sent in a file part (HY-113.3, HY-131.15).
#[pyclass(module = "http_forms")]
struct UploadedFile {
    inner: core::UploadedFile,
}

#[pymethods]
impl UploadedFile {
    fn name(&self) -> &str {
        self.inner.name()
    }
    fn filename(&self) -> &str {
        self.inner.filename()
    }
    fn content_type(&self) -> &str {
        self.inner.content_type()
    }
    fn size(&self) -> u64 {
        self.inner.size()
    }
    fn content<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        self.inner.content().map(|bytes| PyBytes::new(py, &bytes)).map_err(file_error)
    }
    fn move_to(&mut self, path: &Bound<'_, PyAny>) -> PyResult<()> {
        let Ok(path) = path.cast::<PyString>() else {
            return Err(PyTypeError::new_err(core::NOT_A_STRING));
        };
        self.inner.move_to(&PathBuf::from(path.to_str()?)).map_err(file_error)
    }
}

/// Reads a ``multipart/form-data`` body in chunks (HY-131); events are tuples ``("field", name, value)``,
/// ``("file", name, filename, content_type)``, ``("data", bytes)`` and ``("end",)``.
#[pyclass(module = "http_forms")]
struct MultipartReader {
    inner: core::MultipartReader,
}

#[pymethods]
impl MultipartReader {
    #[new]
    #[pyo3(signature = (boundary, value_limit=None, part_limit=None, file_limit=None))]
    fn new(boundary: &str, value_limit: Option<u64>, part_limit: Option<u64>, file_limit: Option<u64>) -> PyResult<Self> {
        let limits = core::Limits { value: value_limit, parts: part_limit, file: file_limit };
        Ok(MultipartReader { inner: core::MultipartReader::new(&bytes_of(boundary)?, limits) })
    }

    fn feed<'py>(&mut self, py: Python<'py>, chunk: &[u8]) -> PyResult<Bound<'py, PyList>> {
        let events = self.inner.feed(chunk).map_err(form_error)?;
        let mut items = Vec::with_capacity(events.len());
        for event in events {
            items.push(match event {
                core::Event::Field { name, value } => ("field", name, value).into_pyobject(py)?.into_any(),
                core::Event::FileStart { name, filename, content_type } => {
                    ("file", name, filename, content_type).into_pyobject(py)?.into_any()
                }
                core::Event::FileData(bytes) => ("data", PyBytes::new(py, &bytes)).into_pyobject(py)?.into_any(),
                core::Event::FileEnd => ("end",).into_pyobject(py)?.into_any(),
            });
        }
        PyList::new(py, items)
    }

    fn finish(&mut self) -> PyResult<()> {
        self.inner.finish().map_err(form_error)
    }

    fn closed(&self) -> bool {
        self.inner.closed()
    }

    fn buffered(&self) -> usize {
        self.inner.buffered()
    }
}

/// The body of a request as its host reads it (HY-59, HY-131).
#[pyclass(module = "http_forms")]
struct RequestBody {
    inner: core::RequestBody,
}

#[pymethods]
impl RequestBody {
    #[new]
    fn new(content_type: &str, limit: u64, stream: bool, value_limit: u64, part_limit: u64, file_limit: u64) -> PyResult<Self> {
        let inner = core::RequestBody::new(&bytes_of(content_type)?, limit, stream, value_limit, part_limit, file_limit)
            .map_err(|error| PyOSError::new_err(error.to_string()))?;
        Ok(RequestBody { inner })
    }

    fn feed(&mut self, chunk: &[u8]) -> PyResult<bool> {
        self.inner.feed(chunk).map_err(|error| PyOSError::new_err(error.to_string()))
    }

    fn exceeded(&self) -> bool {
        self.inner.exceeded()
    }

    fn size(&self) -> u64 {
        self.inner.size()
    }

    fn held<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.held())
    }

    fn streamed(&self) -> bool {
        self.inner.streamed()
    }

    fn parts<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyTuple>>> {
        let Some(form) = self.inner.parts() else { return Ok(None) };
        let fields = fields_list(py, &form.fields)?.into_any();
        let files = files_list(py, form.files)?.into_any();
        Ok(Some(PyTuple::new(py, [fields, files])?))
    }

    fn directory(&self) -> Option<String> {
        self.inner.directory().map(|path| path.to_string_lossy().into_owned())
    }

    fn remove(&mut self) -> PyResult<()> {
        self.inner.remove().map_err(|error| PyOSError::new_err(error.to_string()))
    }
}

#[pymodule]
fn _http_forms(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_urlencoded, m)?)?;
    m.add_function(wrap_pyfunction!(is_urlencoded, m)?)?;
    m.add_function(wrap_pyfunction!(multipart_boundary, m)?)?;
    m.add_function(wrap_pyfunction!(fields_from_body, m)?)?;
    m.add_function(wrap_pyfunction!(files_from_body, m)?)?;
    m.add_function(wrap_pyfunction!(parse_multipart_chunks, m)?)?;
    m.add_class::<UploadedFile>()?;
    m.add_class::<MultipartReader>()?;
    m.add_class::<RequestBody>()?;
    m.add("FormError", m.py().get_type::<FormError>())?;
    m.add("HEADER_LIMIT", core::HEADER_LIMIT)?;
    Ok(())
}
