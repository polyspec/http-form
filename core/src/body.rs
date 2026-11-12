//! The body of a request as its host reads it (HY-59, HY-131): the bytes of a body that is one value, or the parts
//! of a `multipart/form-data` body that is read in chunks, whose files are in a directory of the request.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::store::{create_directory, create_file, FileError};
use crate::{header, Event, Fields, Files, Form, FormError, Limits, MultipartReader, UploadedFile};

/// The file part that is written.
struct Target {
    name: String,
    filename: String,
    content_type: String,
    path: PathBuf,
    file: File,
    size: u64,
}

/// Reads the chunks of a request body.
pub struct RequestBody {
    limit: u64,
    value_limit: u64,
    directory: Option<PathBuf>,
    reader: Option<MultipartReader>,
    chunks: Vec<u8>,
    length: u64,
    over: bool,
    fields: Fields,
    files: Files,
    target: Option<Target>,
    stored: u64,
}

impl RequestBody {
    /// Starts the body of the content type with the body limit. `stream` tells whether a multipart body is read in
    /// chunks, which it is when `multipart/form-data` is a form type of the application (HY-131.1); such a body gets
    /// its directory now. The value limit bounds a body that is one value and the text values of a body read in
    /// chunks, the part limit its parts and the file limit each of its files (HY-131.7 to HY-131.12).
    pub fn new(
        content_type: &[u8],
        limit: u64,
        stream: bool,
        value_limit: u64,
        part_limit: u64,
        file_limit: u64,
    ) -> io::Result<Self> {
        let boundary = if stream { header::boundary(content_type) } else { None };
        let directory = match boundary {
            Some(_) => Some(create_directory()?),
            None => None,
        };
        let limits = Limits { value: Some(value_limit), parts: Some(part_limit), file: Some(file_limit) };
        Ok(RequestBody {
            limit,
            value_limit,
            directory,
            reader: boundary.map(|boundary| MultipartReader::new(&boundary, limits)),
            chunks: Vec::new(),
            length: 0,
            over: false,
            fields: Fields::default(),
            files: Files::default(),
            target: None,
            stored: 0,
        })
    }

    /// Reads the next chunk, and returns false when the body passed a limit, after which it holds no bytes (HY-59.4,
    /// HY-131.11). A body that fails with HY-42 reads on, and `parts` gives nothing for it.
    pub fn feed(&mut self, chunk: &[u8]) -> io::Result<bool> {
        if self.over {
            return Ok(false);
        }
        self.length += chunk.len() as u64;
        if self.length > self.limit || (self.reader.is_none() && self.length > self.value_limit) {
            return Ok(self.passed());
        }
        let Some(reader) = self.reader.as_mut() else {
            self.chunks.extend_from_slice(chunk);
            return Ok(true);
        };
        match reader.feed(chunk) {
            Ok(events) => {
                self.take(events)?;
                Ok(true)
            }
            Err(FormError::Exceeded(_)) => Ok(self.passed()),
            Err(FormError::Invalid) => {
                self.release();
                Ok(true)
            }
        }
    }

    fn take(&mut self, events: Vec<Event>) -> io::Result<()> {
        for event in events {
            match event {
                Event::Field { name, value } => self.fields.push(name, value),
                Event::FileStart { name, filename, content_type } => {
                    let directory = self.directory.as_ref().expect("a body read in chunks has a directory");
                    self.stored += 1;
                    let path = directory.join(self.stored.to_string());
                    let file = create_file(&path).map_err(|error| match error {
                        FileError::Io(error) => error,
                        other => io::Error::other(other.message()),
                    })?;
                    self.target = Some(Target { name, filename, content_type, path, file, size: 0 });
                }
                Event::FileData(bytes) => {
                    if let Some(target) = self.target.as_mut() {
                        target.file.write_all(&bytes)?;
                        target.size += bytes.len() as u64;
                    }
                }
                Event::FileEnd => {
                    if let Some(target) = self.target.take() {
                        drop(target.file);
                        let file = UploadedFile::stored(
                            target.name.clone(),
                            target.filename,
                            target.content_type,
                            target.path,
                            target.size,
                        );
                        self.files.push(target.name, file);
                    }
                }
            }
        }
        Ok(())
    }

    /// Records that the body passed a limit and drops what it holds.
    fn passed(&mut self) -> bool {
        self.over = true;
        self.chunks = Vec::new();
        self.release();
        false
    }

    /// Closes the file of a part whose end was not read.
    fn release(&mut self) {
        self.target = None;
    }

    /// True when the body passed the body limit, the value limit, the part limit or the file limit (HY-131.11).
    pub fn exceeded(&self) -> bool {
        self.over
    }

    /// The number of bytes that were read.
    pub fn size(&self) -> u64 {
        self.length
    }

    /// The body as one value: no bytes for a body read in chunks and for a body that passed a limit.
    pub fn held(&self) -> &[u8] {
        &self.chunks
    }

    /// True when the body is read in chunks.
    pub fn streamed(&self) -> bool {
        self.reader.is_some()
    }

    /// The text fields and the files of a body read in chunks, or `None` when it failed, has not ended or passed a
    /// limit (HY-131.5).
    pub fn parts(&self) -> Option<Form> {
        let reader = self.reader.as_ref()?;
        if self.over || !reader.closed() {
            return None;
        }
        Some(Form::new(self.fields.clone(), self.files.clone()))
    }

    /// The directory of the files of the body, or `None` for a body that is one value.
    pub fn directory(&self) -> Option<&Path> {
        self.directory.as_deref()
    }

    /// Removes the directory with the files that were not moved (HY-131.14).
    pub fn remove(&mut self) -> io::Result<()> {
        self.release();
        match &self.directory {
            Some(directory) if directory.is_dir() => fs::remove_dir_all(directory),
            _ => Ok(()),
        }
    }
}
