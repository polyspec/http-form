//! The files of a request: a directory below the temporary directory of the system for the user of the server only,
//! the files that the server writes into it, their moves and the removal of the directory (HY-131.13 to HY-131.16).

use std::collections::hash_map::RandomState;
use std::fs::{self, File, OpenOptions};
use std::hash::{BuildHasher, Hasher};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

/// The message of a move or a read after a move (HY-131.16).
pub const MOVED: &str = "the file was moved (HY-131)";
/// The message of a move to a path that exists (HY-131.15).
pub const EXISTS: &str = "the path exists (HY-131)";
/// The message of a move to a path that is not a string, which a binding gives for a value of another type.
pub const NOT_A_STRING: &str = "a path is not a string (HY-131)";

/// A failure of a file operation of an application.
#[derive(Debug)]
pub enum FileError {
    /// The file was moved: a second move or a read after a move. Every binding gives it as the class of HY-46.2.
    Moved,
    /// The target path exists. Every binding gives it as the class of HY-46.2.
    Exists,
    /// Another failure of the file system.
    Io(io::Error),
}

impl FileError {
    /// The message of the failure, which every binding gives as it is.
    pub fn message(&self) -> String {
        match self {
            FileError::Moved => MOVED.to_owned(),
            FileError::Exists => EXISTS.to_owned(),
            FileError::Io(error) => error.to_string(),
        }
    }
}

impl From<io::Error> for FileError {
    fn from(error: io::Error) -> Self {
        FileError::Io(error)
    }
}

fn random() -> u64 {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(std::process::id() as u64);
    hasher.finish()
}

/// Creates a new directory below the temporary directory of the system that only its user can open.
pub fn create_directory() -> io::Result<PathBuf> {
    let base = std::env::temp_dir();
    loop {
        let path = base.join(format!("hyper-{:016x}", random()));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        match builder.create(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
}

/// Opens a new file for writing that only its user can open, and fails when the path exists.
pub fn create_file(path: &Path) -> Result<File, FileError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(path).map_err(|error| match error.kind() {
        io::ErrorKind::AlreadyExists => FileError::Exists,
        _ => FileError::Io(error),
    })
}

/// Where the bytes of a file are.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Content {
    /// The bytes as one value, of a body that the server held as one value.
    Value(Vec<u8>),
    /// A file of the directory of the request.
    Stored(PathBuf),
    /// Nowhere: the file was moved.
    Moved,
}

/// A file that a request sent in a file part (HY-113.3): its name, its filename, its content type and its bytes,
/// which are a value or a file of the directory of the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadedFile {
    name: String,
    filename: String,
    content_type: String,
    size: u64,
    content: Content,
}

impl UploadedFile {
    /// A file whose bytes are a value.
    pub fn from_value(name: String, filename: String, content_type: String, content: Vec<u8>) -> Self {
        UploadedFile { name, filename, content_type, size: content.len() as u64, content: Content::Value(content) }
    }

    /// A file whose bytes are the file of the path, of the size in bytes.
    pub fn stored(name: String, filename: String, content_type: String, path: PathBuf, size: u64) -> Self {
        UploadedFile { name, filename, content_type, size, content: Content::Stored(path) }
    }

    /// The `name` parameter of the part.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The `filename` parameter of the part, as it was sent (HY-113.14).
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// The `Content-Type` header of the part as it was sent, or the empty text.
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// The number of bytes of the part (HY-131.15).
    pub fn size(&self) -> u64 {
        self.size
    }

    /// The bytes of the part as one value; fails after a move (HY-131.16).
    pub fn content(&self) -> Result<Vec<u8>, FileError> {
        match &self.content {
            Content::Value(bytes) => Ok(bytes.clone()),
            Content::Stored(path) => {
                let mut bytes = Vec::with_capacity(self.size as usize);
                File::open(path)?.read_to_end(&mut bytes)?;
                Ok(bytes)
            }
            Content::Moved => Err(FileError::Moved),
        }
    }

    /// Moves the file to the path: a stored file by a second name and the removal of the first, or by a copy when
    /// the file system gives no second name, as between two file systems; fails when the path exists or when the
    /// file was moved (HY-131.15, HY-131.16).
    pub fn move_to(&mut self, path: &Path) -> Result<(), FileError> {
        match &self.content {
            Content::Moved => return Err(FileError::Moved),
            Content::Value(bytes) => {
                create_file(path)?.write_all(bytes)?;
            }
            Content::Stored(source) => {
                match fs::hard_link(source, path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Err(FileError::Exists),
                    Err(_) => {
                        let mut origin = File::open(source)?;
                        let mut target = create_file(path)?;
                        io::copy(&mut origin, &mut target)?;
                    }
                }
                fs::remove_file(source)?;
            }
        }
        self.content = Content::Moved;
        Ok(())
    }
}
