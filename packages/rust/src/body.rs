//! The body of a request as its host reads it (F-24, F-25): the bytes of a body that is one value, or the parts
//! of a `multipart/form-data` body that is read in chunks, whose files are in a directory of the request. The module
//! is of the feature `fs` (F-28).

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use binding_core::Io;

use crate::fault::{self, Fault};
use crate::fields::Named;
use crate::multipart::count;
use crate::store::{create_directory, create_file};
use crate::{content, BodyLimits, Error, Form, IoFailure, Limits, MultipartReader, Piece, ReadError, UploadedFile};

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
    fields: Named<String>,
    files: Named<UploadedFile>,
    target: Option<Target>,
    stored: u64,
}

impl Parts {
    /// Takes a piece of the reader: a value, or the start, the data or the end of a file, which it writes to a new
    /// file of the directory (F-10). A piece out of the order of a reader is `internal` (H-8).
    fn take(&mut self, piece: Piece<'_>) -> Result<(), Error> {
        match piece {
            Piece::Field { name, value } => self.fields.push(name, value),
            Piece::FileStart { name, filename, content_type } => {
                if self.target.is_some() {
                    return Err(Error::Internal("a file started before the end of the file before it"));
                }
                self.stored = self.stored.checked_add(1).ok_or(Error::Internal("files"))?;
                let path = self.directory.join(self.stored.to_string());
                // F-10: a path that exists is a failure of the file system (`io`, `EEXIST`). The core numbers the
                // files, but another process of the same user can create one in the directory.
                fault::fail(fault::CREATE_FILE)?;
                let file = create_file(&path)?;
                self.target = Some(Target { name, filename, content_type, path, file, size: 0 });
            }
            Piece::FileData(bytes) => {
                let Some(target) = self.target.as_mut() else {
                    return Err(Error::Internal("data of a file came without its start"));
                };
                // F-15: a short write is found by the size check at the end of the file.
                match fault::system(fault::WRITE_FILE) {
                    Some(Fault::Fail(error)) => return Err(error.into()),
                    Some(Fault::Short(short)) => target.file.write_all(fault::prefix(bytes, short))?,
                    None => target.file.write_all(bytes)?,
                }
                target.size = target.size.checked_add(count(bytes.len())?).ok_or(Error::Internal("size"))?;
            }
            Piece::FileEnd => {
                let Some(target) = self.target.take() else {
                    return Err(Error::Internal("the end of a file came without its start"));
                };
                // H-22: `std` does not report a failure of close(2), so the size of the file is checked before it is
                // closed.
                fault::fail(fault::FILE_SIZE)?;
                let written = target.file.metadata()?.len();
                if written != target.size {
                    return Err(Error::Io(IoFailure::found(format!(
                        "the file holds {written} bytes where {} were written",
                        target.size
                    ))));
                }
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
        Ok(())
    }
}

/// The value of a `Content-Length` (RFC 9110 8.6): one or more digits. A number beyond 64 bits is beyond any limit.
fn parse_content_length(value: &[u8]) -> Result<Option<u64>, Error> {
    if value.is_empty() || !value.iter().all(u8::is_ascii_digit) {
        return Err(Error::InvalidContentLength);
    }
    let mut number: u64 = 0;
    for &digit in value {
        let next = number.checked_mul(10).and_then(|tens| tens.checked_add(u64::from(digit - b'0')));
        match next {
            Some(next) => number = next,
            None => return Ok(None),
        }
    }
    Ok(Some(number))
}

/// Reads the chunks of a request body.
pub struct RequestBody {
    limit: u64,
    value_limit: u64,
    reader: Option<MultipartReader>,
    parts: Option<Parts>,
    chunks: Vec<u8>,
    length: u64,
    failure: Option<Error>,
    finished: bool,
    removed: bool,
}

impl RequestBody {
    /// Starts the body of the content type with its limits.
    ///
    /// `content_type` is the value of `Content-Type`, or `None` without one. `content_length` is the value of the `Content-Length` field as the host received it, or `None` without one; a
    /// value that is not a number fails with `invalid_content_length`, and a number above the limit fails with
    /// `body_limit` before any byte is read (F-25.1). `stream` tells whether a multipart body is read in chunks,
    /// which it is when `multipart/form-data` is a form type of the application (F-24.1); such a body needs its
    /// boundary (H-17, H-18) and gets its directory now. The limits are those of [`BodyLimits`]. It creates the
    /// directory, so it takes the wait token `Io` (spec F-17).
    pub fn new(
        _io: Io,
        content_type: Option<&[u8]>,
        content_length: Option<&[u8]>,
        stream: bool,
        limits: BodyLimits,
    ) -> Result<Self, Error> {
        let BodyLimits { limit, value_limit, part_limit, file_limit } = limits;
        if let Some(value) = content_length {
            match parse_content_length(value)? {
                Some(length) if length <= limit => {}
                Some(_) | None => return Err(Error::BodyLimit),
            }
        }
        let boundary = if stream { content::boundary(content_type)? } else { None };
        let (reader, parts) = match boundary {
            Some(boundary) => {
                let limits = Limits { value: Some(value_limit), parts: Some(part_limit), file: Some(file_limit) };
                let parts = Parts {
                    directory: create_directory()?,
                    fields: Named::default(),
                    files: Named::default(),
                    target: None,
                    stored: 0,
                };
                (Some(MultipartReader::new(&boundary, limits)), Some(parts))
            }
            None => (None, None),
        };
        Ok(RequestBody {
            limit,
            value_limit,
            reader,
            parts,
            chunks: Vec::new(),
            length: 0,
            failure: None,
            finished: false,
            removed: false,
        })
    }

    /// Reads the next chunk (H-5). The first failure of the body is its error and the error of every later call, and
    /// the body then holds no bytes and no open file (F-25): `invalid` and `header_limit` (F-21.5, F-23.4,
    /// F-24.5), the limits, and a failure of the file system. The first failure is the one whose byte comes first
    /// (spec F-8); the body limit is passed at the byte after `limit` bytes, and a chunk that passes it is read up to
    /// the limit first, so that a failure before that byte is the error. It writes the files of the body, so it takes
    /// the wait token `Io` (spec F-17).
    pub fn feed(&mut self, _io: Io, chunk: &[u8]) -> Result<(), Error> {
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        if self.removed {
            return Err(Error::Removed);
        }
        if self.finished {
            return Err(Error::Finished);
        }
        let result = self.read(chunk);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
            self.chunks = Vec::new();
            self.release();
        }
        result
    }

    fn read(&mut self, chunk: &[u8]) -> Result<(), Error> {
        let start = self.length;
        self.length = start.checked_add(count(chunk.len())?).ok_or(Error::BodyLimit)?;
        // The bytes of the chunk before the body limit.
        let within = match usize::try_from(self.limit.saturating_sub(start)) {
            Ok(within) if within < chunk.len() => Some(within),
            Ok(_) | Err(_) => None,
        };
        let (Some(reader), Some(parts)) = (self.reader.as_mut(), self.parts.as_mut()) else {
            // A body held as one value: the body limit and the value limit (F-25.2); at one byte the body limit
            // comes first.
            let value = match usize::try_from(self.value_limit.saturating_sub(start)) {
                Ok(value) if value < chunk.len() => Some(value),
                Ok(_) | Err(_) => None,
            };
            return match (within, value) {
                (Some(within), Some(value)) if value < within => Err(Error::ValueLimit),
                (Some(_), _) => Err(Error::BodyLimit),
                (None, Some(_)) => Err(Error::ValueLimit),
                (None, None) => {
                    self.chunks.extend_from_slice(chunk);
                    Ok(())
                }
            };
        };
        let read = match within {
            Some(within) => &chunk[..within],
            None => chunk,
        };
        let result = reader.feed_with(read, |piece| parts.take(piece));
        match result {
            Ok(()) => {}
            Err(ReadError::Form(error) | ReadError::Sink(error)) => return Err(error),
            Err(ReadError::SinkFailed) => return Err(Error::Internal("the body read on after a failure")),
        }
        match within {
            Some(_) => Err(Error::BodyLimit),
            None => Ok(()),
        }
    }

    /// Ends the body, as its host calls it after the last chunk (H-4): a body read in chunks that did not end with
    /// the `--` after a delimiter fails with `invalid` (F-24.5). A second call gives the result of the first. It
    /// closes the file of a body that failed, so it takes the wait token `Io` (spec F-17).
    pub fn finish(&mut self, _io: Io) -> Result<(), Error> {
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        if self.removed {
            return Err(Error::Removed);
        }
        self.finished = true;
        let Some(reader) = self.reader.as_mut() else {
            return Ok(());
        };
        let result = match reader.finish() {
            Ok(()) => Ok(()),
            Err(ReadError::Form(error)) => Err(error),
            Err(ReadError::Sink(never)) => match never {},
            Err(ReadError::SinkFailed) => Err(Error::Internal("the body ended after a failure")),
        };
        if let Err(error) = &result {
            self.failure = Some(error.clone());
            self.release();
        }
        result
    }

    /// Closes the file of a part whose end was not read.
    fn release(&mut self) {
        if let Some(parts) = self.parts.as_mut() {
            parts.target = None;
        }
    }

    /// The number of bytes that were given.
    pub fn size(&self) -> u64 {
        self.length
    }

    /// The body as one value: no bytes for a body read in chunks, and none after a failure.
    pub fn held(&self) -> Vec<u8> {
        self.chunks.clone()
    }

    /// True when the body is read in chunks.
    pub fn streamed(&self) -> bool {
        self.reader.is_some()
    }

    /// The text fields and the files of a body read in chunks (H-4). Fails with `not_streamed` for a body held as one
    /// value, with the first failure of the body, and with `not_finished` before `finish`. Every call gives the same
    /// files, which share their state (H-16).
    pub fn parts(&self) -> Result<Form, Error> {
        let Some(parts) = self.parts.as_ref() else {
            return Err(Error::NotStreamed);
        };
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        if !self.finished {
            return Err(Error::NotFinished);
        }
        Ok(Form { fields: parts.fields.entries().to_vec(), files: parts.files.entries().to_vec() })
    }

    /// The directory of the files of a body read in chunks, or `None` for a body held as one value (F-10).
    pub fn directory(&self) -> Option<PathBuf> {
        self.parts.as_ref().map(|parts| parts.directory.clone())
    }

    /// Removes the directory with the files that were not moved (F-10, H-20). The directory belongs to this
    /// request only, so a directory that is gone fails with `not_found`; after a removal a second call does nothing.
    /// A stored file of the body is then `removed` (H-16), and a later chunk fails with `removed`. It takes the wait
    /// token `Io` (spec F-17).
    pub fn remove(&mut self, _io: Io) -> Result<(), Error> {
        self.release();
        let Some(parts) = self.parts.as_ref() else {
            return Ok(());
        };
        if self.removed {
            return Ok(());
        }
        fault::fail(fault::REMOVE_DIRECTORY)?;
        fs::remove_dir_all(&parts.directory)?;
        self.removed = true;
        for (_, files) in parts.files.entries() {
            for file in files {
                file.mark_removed()?;
            }
        }
        Ok(())
    }
}

impl Drop for RequestBody {
    /// Removes the directory of a body whose host did not call `remove` (plan B-10). A failure here has no caller to
    /// reach (spec F-10); the binding reports the missing call. This blocking I/O takes no wait token, because `Drop`
    /// takes no argument (spec F-17).
    fn drop(&mut self) {
        if self.removed {
            return;
        }
        if let Some(parts) = self.parts.as_mut() {
            parts.target = None;
            match fs::remove_dir_all(&parts.directory) {
                Ok(()) => {}
                Err(error) => drop(error),
            }
        }
    }
}

/// `remove` is the cleanup call of a body (design U-1 `cleanup`, plan B-10, spec F-10).
impl binding_core::Cleanup for RequestBody {}
