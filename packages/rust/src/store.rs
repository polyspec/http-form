//! The files of a request (F-22.2): a file whose bytes are a value, or, with the feature `fs`, a file that the server
//! writes into a directory of the request and its moves (F-10 to F-11, F-28), which are in the module `files`.

#[cfg(feature = "fs")]
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use binding_core::Io;

use crate::fault;
use crate::multipart::count;
use crate::Error;

#[cfg(feature = "fs")]
mod files;

#[cfg(feature = "fs")]
pub(crate) use files::{create_directory, create_file};

/// Where the bytes of a file are.
#[derive(Debug)]
pub(crate) enum Content {
    /// The bytes as one value: of a body that the server held as one value, or of a file that an application made.
    Value(Vec<u8>),
    /// A file of the directory of the request.
    #[cfg(feature = "fs")]
    Stored(PathBuf),
    /// Nowhere: the file was moved.
    #[cfg(feature = "fs")]
    Moved,
    /// Nowhere: the directory of the request was removed.
    #[cfg(feature = "fs")]
    Removed,
}

#[derive(Debug)]
struct Shared {
    name: String,
    filename: String,
    content_type: String,
    size: u64,
    content: Mutex<Content>,
}

/// A file that a request sent in a file part (F-22.2): its name, its filename, its content type and its bytes,
/// which are a value or a file of the directory of the request. A clone is the same file: every clone sees a move
/// and the removal of the directory (H-16).
#[derive(Debug, Clone)]
pub struct UploadedFile {
    shared: Arc<Shared>,
}

impl UploadedFile {
    /// A file whose bytes are a value, as an application makes one (F-22.2); its size is the number of its bytes.
    pub fn from_value(name: &str, filename: &str, content_type: &str, content: &[u8]) -> Result<Self, Error> {
        UploadedFile::from_owned(name.to_owned(), filename.to_owned(), content_type.to_owned(), content.to_vec())
    }

    /// A file of the bytes that the reader of a body collected, which it owns already, so they are not copied.
    pub(crate) fn from_owned(
        name: String,
        filename: String,
        content_type: String,
        content: Vec<u8>,
    ) -> Result<Self, Error> {
        let size = count(content.len())?;
        Ok(UploadedFile::make(name, filename, content_type, size, Content::Value(content)))
    }

    fn make(name: String, filename: String, content_type: String, size: u64, content: Content) -> Self {
        UploadedFile { shared: Arc::new(Shared { name, filename, content_type, size, content: Mutex::new(content) }) }
    }

    /// The content of the file. A lock that a panic left poisoned is `internal` (H-8); the point `internal` imitates
    /// it (X-1).
    #[allow(
        clippy::map_err_ignore,
        reason = "H-8: the poisoned lock holds the content, not a cause, so the kind states all"
    )]
    fn lock(&self) -> Result<MutexGuard<'_, Content>, Error> {
        const POISONED: &str = "a file was used after a panic in a call on it";
        if fault::broken(fault::INTERNAL) {
            return Err(Error::Internal(POISONED));
        }
        self.shared.content.lock().map_err(|_| Error::Internal(POISONED))
    }

    /// The `name` parameter of the part.
    pub fn name(&self) -> String {
        self.shared.name.clone()
    }

    /// The `filename` parameter of the part, as it was sent (F-6).
    pub fn filename(&self) -> String {
        self.shared.filename.clone()
    }

    /// The `Content-Type` header of the part as it was sent, or the empty text.
    pub fn content_type(&self) -> String {
        self.shared.content_type.clone()
    }

    /// The number of bytes of the part (F-11).
    pub fn size(&self) -> u64 {
        self.shared.size
    }

    /// The bytes of the part as one value. A stored file must hold the number of bytes that were written (H-22).
    /// Fails with `moved` after a move (F-11) and with `removed` after the removal of the directory (H-16). It
    /// reads a stored file, so it takes the wait token `Io` (spec F-17).
    pub fn content(&self, _io: Io) -> Result<Vec<u8>, Error> {
        self.bytes()
    }

    /// The bytes of the part as one value in a build without the feature `fs` (F-28): every file there holds its
    /// bytes as a value, so the read does no I/O and takes no wait token (spec F-17). The WASM binding calls it as
    /// `content`.
    #[cfg(not(feature = "fs"))]
    pub fn held_content(&self) -> Result<Vec<u8>, Error> {
        self.bytes()
    }

    fn bytes(&self) -> Result<Vec<u8>, Error> {
        let content = self.lock()?;
        match &*content {
            Content::Value(bytes) => Ok(bytes.clone()),
            #[cfg(feature = "fs")]
            Content::Stored(path) => files::read_stored(path, self.shared.size),
            #[cfg(feature = "fs")]
            Content::Moved => Err(Error::Moved),
            #[cfg(feature = "fs")]
            Content::Removed => Err(Error::Removed),
        }
    }
}

#[cfg(all(test, not(feature = "fs")))]
mod tests {
    //! F-28: without the feature `fs` a file holds its bytes as a value, and `held_content` gives them.

    use super::*;

    #[test]
    fn a_file_without_fs_gives_its_bytes() {
        let file = UploadedFile::from_value("f", "f.txt", "text/plain", b"abc").unwrap();
        assert_eq!(file.held_content(), Ok(b"abc".to_vec()));
        assert_eq!(file.held_content(), file.content(Io::for_generated_call()));
        assert_eq!(
            (file.name(), file.filename(), file.content_type(), file.size()),
            ("f".into(), "f.txt".into(), "text/plain".into(), 3)
        );
    }
}
