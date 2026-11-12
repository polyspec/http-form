//! The body of a request as its host reads it (HY-59, HY-131): the bytes of a body that is one value, or the parts
//! of a `multipart/form-data` body that is read in chunks, whose files are in a directory of the request.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::store::{create_directory, create_file, FileError};
use crate::{header, Fields, Files, Form, FormError, Limits, MultipartReader, Piece, ReadError, UploadedFile};

/// The file part that is written.
struct Target {
    name: String,
    filename: String,
    content_type: String,
    path: PathBuf,
    file: File,
    size: u64,
}

/// The parts of a body read in chunks, with the file that is written.
struct Parts {
    directory: PathBuf,
    fields: Fields,
    files: Files,
    target: Option<Target>,
    stored: u64,
}

impl Parts {
    /// Takes a piece of the reader: a value, or the start, the data or the end of a file, which it writes to a new
    /// file of the directory (HY-131.13).
    fn take(&mut self, piece: Piece<'_>) -> io::Result<()> {
        match piece {
            Piece::Field { name, value } => self.fields.push(name, value),
            Piece::FileStart { name, filename, content_type } => {
                self.stored += 1;
                let path = self.directory.join(self.stored.to_string());
                let file = create_file(&path).map_err(|error| match error {
                    FileError::Io(error) => error,
                    other => io::Error::other(other.message()),
                })?;
                self.target = Some(Target { name, filename, content_type, path, file, size: 0 });
            }
            Piece::FileData(bytes) => {
                if let Some(target) = self.target.as_mut() {
                    target.file.write_all(bytes)?;
                    target.size += bytes.len() as u64;
                }
            }
            Piece::FileEnd => {
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
        Ok(())
    }
}

/// Reads the chunks of a request body.
pub struct RequestBody {
    limit: u64,
    value_limit: u64,
    reader: Option<MultipartReader>,
    parts: Option<Parts>,
    chunks: Vec<u8>,
    length: u64,
    over: bool,
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
        let parts = match boundary {
            Some(_) => Some(Parts {
                directory: create_directory()?,
                fields: Fields::default(),
                files: Files::default(),
                target: None,
                stored: 0,
            }),
            None => None,
        };
        let limits = Limits { value: Some(value_limit), parts: Some(part_limit), file: Some(file_limit) };
        Ok(RequestBody {
            limit,
            value_limit,
            reader: boundary.map(|boundary| MultipartReader::new(&boundary, limits)),
            parts,
            chunks: Vec::new(),
            length: 0,
            over: false,
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
        let (Some(reader), Some(parts)) = (self.reader.as_mut(), self.parts.as_mut()) else {
            self.chunks.extend_from_slice(chunk);
            return Ok(true);
        };
        match reader.feed_with(chunk, |piece| parts.take(piece)) {
            Ok(()) => Ok(true),
            Err(ReadError::Form(FormError::Exceeded(_))) => Ok(self.passed()),
            Err(ReadError::Form(FormError::Invalid)) => {
                self.release();
                Ok(true)
            }
            Err(ReadError::Sink(error)) => {
                self.release();
                Err(error)
            }
        }
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
        if let Some(parts) = self.parts.as_mut() {
            parts.target = None;
        }
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
        let (reader, parts) = (self.reader.as_ref()?, self.parts.as_ref()?);
        if self.over || !reader.closed() {
            return None;
        }
        Some(Form::new(parts.fields.clone(), parts.files.clone()))
    }

    /// The directory of the files of the body, or `None` for a body that is one value.
    pub fn directory(&self) -> Option<&Path> {
        self.parts.as_ref().map(|parts| parts.directory.as_path())
    }

    /// Removes the directory with the files that were not moved (HY-131.14).
    pub fn remove(&mut self) -> io::Result<()> {
        self.release();
        match &self.parts {
            Some(parts) if parts.directory.is_dir() => fs::remove_dir_all(&parts.directory),
            _ => Ok(()),
        }
    }
}
