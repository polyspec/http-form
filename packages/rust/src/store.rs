//! The files of a request: a directory below the temporary directory of the system for the user of the server only,
//! the files that the server writes into it, their moves and the removal of the directory (F-10 to F-11).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use bindgen_core::Io;

use crate::multipart::count;
use crate::{Error, IoFailure};

#[cfg(not(any(target_os = "linux", target_vendor = "apple")))]
compile_error!("http-form moves a file without replacing its target on Linux and macOS only (spec F-14)");

/// Creates a new directory below the temporary directory of the system that only its user can open (F-10). The
/// name is `http-form-` and random characters, which `tempfile` draws again when the name exists.
pub(crate) fn create_directory() -> Result<PathBuf, Error> {
    let mut builder = tempfile::Builder::new();
    builder.prefix("http-form-");
    #[cfg(unix)]
    builder.permissions(std::os::unix::fs::PermissionsExt::from_mode(0o700));
    Ok(builder.tempdir()?.keep())
}

/// Opens a new file for writing that only its user can open, and fails with `AlreadyExists` when the path exists.
/// The file is closed on `exec` (`O_CLOEXEC`, which `std` sets on every file it opens).
pub(crate) fn create_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(path)
}

/// The operations of a move, so that a test can make each of them fail.
pub(crate) trait Ops {
    /// Renames without replacing the target: `AlreadyExists` when the target exists.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    fn open(&self, path: &Path) -> io::Result<File>;
    fn create(&self, path: &Path) -> io::Result<File>;
    fn copy(&self, from: &mut File, to: &mut File) -> io::Result<u64>;
    fn write(&self, to: &mut File, bytes: &[u8]) -> io::Result<()>;
    fn sync(&self, file: &File) -> io::Result<()>;
    fn remove(&self, path: &Path) -> io::Result<()>;
}

/// The operations of the system.
pub(crate) struct System;

impl Ops for System {
    /// `renameat2(RENAME_NOREPLACE)` on Linux and `renameatx_np(RENAME_EXCL)` on macOS (spec F-11).
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        use rustix::fs::{renameat_with, RenameFlags, CWD};
        renameat_with(CWD, from, CWD, to, RenameFlags::NOREPLACE).map_err(io::Error::from)
    }

    fn open(&self, path: &Path) -> io::Result<File> {
        File::open(path)
    }

    fn create(&self, path: &Path) -> io::Result<File> {
        create_file(path)
    }

    fn copy(&self, from: &mut File, to: &mut File) -> io::Result<u64> {
        io::copy(from, to)
    }

    fn write(&self, to: &mut File, bytes: &[u8]) -> io::Result<()> {
        to.write_all(bytes)
    }

    fn sync(&self, file: &File) -> io::Result<()> {
        file.sync_all()
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }
}

/// Where the bytes of a file are.
#[derive(Debug)]
pub(crate) enum Content {
    /// The bytes as one value: of a body that the server held as one value, or of a file that an application made.
    Value(Vec<u8>),
    /// A file of the directory of the request.
    Stored(PathBuf),
    /// Nowhere: the file was moved.
    Moved,
    /// Nowhere: the directory of the request was removed.
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
    pub fn from_value(name: String, filename: String, content_type: String, content: Vec<u8>) -> Result<Self, Error> {
        let size = count(content.len())?;
        Ok(UploadedFile::make(name, filename, content_type, size, Content::Value(content)))
    }

    /// A file whose bytes are the file of the path, of the size in bytes.
    pub(crate) fn stored(name: String, filename: String, content_type: String, path: PathBuf, size: u64) -> Self {
        UploadedFile::make(name, filename, content_type, size, Content::Stored(path))
    }

    fn make(name: String, filename: String, content_type: String, size: u64, content: Content) -> Self {
        UploadedFile { shared: Arc::new(Shared { name, filename, content_type, size, content: Mutex::new(content) }) }
    }

    fn lock(&self) -> Result<MutexGuard<'_, Content>, Error> {
        self.shared.content.lock().map_err(|_| Error::Internal("a file was used after a panic in a call on it"))
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

    /// Marks a stored file as removed with the directory of its request (H-16).
    pub(crate) fn mark_removed(&self) -> Result<(), Error> {
        let mut content = self.lock()?;
        if matches!(*content, Content::Stored(_)) {
            *content = Content::Removed;
        }
        Ok(())
    }

    /// The bytes of the part as one value. A stored file must hold the number of bytes that were written (H-22).
    /// Fails with `moved` after a move (F-11) and with `removed` after the removal of the directory (H-16). It
    /// reads a stored file, so it takes the wait token `Io` (spec F-17).
    pub fn content(&self, _io: Io) -> Result<Vec<u8>, Error> {
        let content = self.lock()?;
        match &*content {
            Content::Value(bytes) => Ok(bytes.clone()),
            Content::Stored(path) => read_stored(path, self.shared.size),
            Content::Moved => Err(Error::Moved),
            Content::Removed => Err(Error::Removed),
        }
    }

    /// Moves the file to the path without replacing a file there (F-11, spec F-11): a stored file by a rename,
    /// or by a copy and a removal when the path is on another file system, and a value by a write. Fails with
    /// `exists` when the path exists, `moved` when the file was moved and `removed` when its directory was removed. It
    /// takes the wait token `Io` (spec F-17).
    pub fn move_to(&self, _io: Io, path: &Path) -> Result<(), Error> {
        self.move_with(&System, path)
    }

    pub(crate) fn move_with(&self, ops: &impl Ops, path: &Path) -> Result<(), Error> {
        let mut content = self.lock()?;
        let result = match &*content {
            Content::Moved => return Err(Error::Moved),
            Content::Removed => return Err(Error::Removed),
            Content::Value(bytes) => write_new(ops, path, bytes),
            Content::Stored(source) => move_stored(ops, source, path, self.shared.size),
        };
        match result {
            Ok(()) => {
                *content = Content::Moved;
                Ok(())
            }
            Err(Error::SourceLeft(failure)) => {
                *content = Content::Moved;
                Err(Error::SourceLeft(failure))
            }
            Err(error) => Err(error),
        }
    }
}

fn mismatch(found: u64, size: u64) -> IoFailure {
    IoFailure::found(format!("the file holds {found} bytes where {size} were written"))
}

fn read_stored(path: &Path, size: u64) -> Result<Vec<u8>, Error> {
    let capacity = usize::try_from(size).map_err(|_| Error::Internal("a size does not fit the memory"))?;
    let mut bytes = Vec::with_capacity(capacity);
    File::open(path)?.read_to_end(&mut bytes)?;
    let found = count(bytes.len())?;
    if found != size {
        return Err(Error::Io(mismatch(found, size)));
    }
    Ok(bytes)
}

/// Creates the target, without replacing a file there.
fn create_target(ops: &impl Ops, path: &Path) -> Result<File, Error> {
    match ops.create(path) {
        Ok(file) => Ok(file),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Err(Error::Exists),
        Err(error) => Err(error.into()),
    }
}

/// Removes the target that a failed write created; the failure is `partial_target` when the removal fails too.
fn undo(ops: &impl Ops, path: &Path, failure: IoFailure) -> Error {
    match ops.remove(path) {
        Ok(()) => Error::Io(failure),
        Err(removal) => Error::PartialTarget { failure, removal: removal.into() },
    }
}

/// Writes the bytes of a value to a new file and to the disk (H-9).
fn write_new(ops: &impl Ops, path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let mut target = create_target(ops, path)?;
    let written = ops.write(&mut target, bytes).and_then(|()| ops.sync(&target));
    drop(target);
    match written {
        Ok(()) => Ok(()),
        Err(error) => Err(undo(ops, path, error.into())),
    }
}

/// Moves a stored file (H-9). The rename does not replace a file at the target. Only `EXDEV`, a target on another
/// file system, takes the copy (plan P-3): the copy opens a new target, copies, writes it to the disk and removes the
/// source.
fn move_stored(ops: &impl Ops, source: &Path, path: &Path, size: u64) -> Result<(), Error> {
    match ops.rename(source, path) {
        Ok(()) => return Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Err(Error::Exists),
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {}
        Err(error) => return Err(error.into()),
    }
    let mut origin = ops.open(source)?;
    let mut target = create_target(ops, path)?;
    let copied = match ops.copy(&mut origin, &mut target) {
        Ok(found) if found == size => ops.sync(&target).map_err(IoFailure::from),
        Ok(found) => Err(mismatch(found, size)),
        Err(error) => Err(error.into()),
    };
    drop(target);
    if let Err(failure) = copied {
        return Err(undo(ops, path, failure));
    }
    match ops.remove(source) {
        Ok(()) => Ok(()),
        Err(error) => Err(Error::SourceLeft(error.into())),
    }
}

#[cfg(test)]
mod tests {
    //! H-9: each step of a move fails in turn; the operations are those of the system but the one that fails.

    use super::*;

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Step {
        None,
        RenameCrossDevice,
        RenameInvalid,
        Copy,
        ShortCopy,
        Sync,
        Write,
        RemoveTarget,
        RemoveSource,
    }

    struct Faulty {
        fail: Vec<Step>,
    }

    impl Faulty {
        fn fails(&self, step: Step) -> bool {
            self.fail.contains(&step)
        }
    }

    fn failure(errno: i32) -> io::Error {
        io::Error::from_raw_os_error(errno)
    }

    const EIO: i32 = 5;
    const EXDEV: i32 = 18;
    const EINVAL: i32 = 22;
    const ENOSPC: i32 = 28;
    const EACCES: i32 = 13;

    impl Ops for Faulty {
        fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
            if self.fails(Step::RenameCrossDevice) {
                return Err(failure(EXDEV));
            }
            if self.fails(Step::RenameInvalid) {
                return Err(failure(EINVAL));
            }
            System.rename(from, to)
        }
        fn open(&self, path: &Path) -> io::Result<File> {
            System.open(path)
        }
        fn create(&self, path: &Path) -> io::Result<File> {
            System.create(path)
        }
        fn copy(&self, from: &mut File, to: &mut File) -> io::Result<u64> {
            if self.fails(Step::Copy) {
                to.write_all(b"pa")?;
                return Err(failure(ENOSPC));
            }
            let copied = System.copy(from, to)?;
            if self.fails(Step::ShortCopy) {
                return Ok(copied - 1);
            }
            Ok(copied)
        }
        fn write(&self, to: &mut File, bytes: &[u8]) -> io::Result<()> {
            if self.fails(Step::Write) {
                return Err(failure(ENOSPC));
            }
            System.write(to, bytes)
        }
        fn sync(&self, file: &File) -> io::Result<()> {
            if self.fails(Step::Sync) {
                return Err(failure(EIO));
            }
            System.sync(file)
        }
        fn remove(&self, path: &Path) -> io::Result<()> {
            let target = path.file_name().is_some_and(|name| name == "target");
            if (target && self.fails(Step::RemoveTarget)) || (!target && self.fails(Step::RemoveSource)) {
                return Err(failure(EACCES));
            }
            System.remove(path)
        }
    }

    struct Place {
        directory: PathBuf,
    }

    impl Place {
        fn new(name: &str) -> Place {
            let directory = std::env::temp_dir().join(format!("http-form-store-{}-{name}", std::process::id()));
            match fs::remove_dir_all(&directory) {
                Ok(()) => {}
                Err(error) => assert_eq!(error.kind(), io::ErrorKind::NotFound),
            }
            fs::create_dir(&directory).unwrap();
            Place { directory }
        }
        fn stored(&self) -> UploadedFile {
            let source = self.directory.join("source");
            fs::write(&source, b"payload").unwrap();
            UploadedFile::stored("f".into(), "f.txt".into(), String::new(), source, 7)
        }
        fn target(&self) -> PathBuf {
            self.directory.join("target")
        }
        fn exists(&self, name: &str) -> bool {
            self.directory.join(name).exists()
        }
    }

    impl Drop for Place {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.directory).unwrap();
        }
    }

    fn faulty(fail: &[Step]) -> Faulty {
        Faulty { fail: fail.to_vec() }
    }

    #[test]
    fn a_rename_moves_once_and_never_replaces() {
        let place = Place::new("rename");
        let file = place.stored();
        fs::write(place.target(), b"taken").unwrap();
        assert_eq!(file.move_with(&faulty(&[Step::None]), &place.target()), Err(Error::Exists));
        fs::remove_file(place.target()).unwrap();
        file.move_with(&faulty(&[Step::None]), &place.target()).unwrap();
        assert_eq!(fs::read(place.target()).unwrap(), b"payload");
        assert!(!place.exists("source"));
        assert_eq!(file.move_to(Io::for_generated_call(), &place.directory.join("again")), Err(Error::Moved));
        assert_eq!(file.content(Io::for_generated_call()), Err(Error::Moved));
    }

    #[test]
    fn exdev_alone_copies_and_removes_the_source() {
        let place = Place::new("exdev");
        let file = place.stored();
        file.move_with(&faulty(&[Step::RenameCrossDevice]), &place.target()).unwrap();
        assert_eq!(fs::read(place.target()).unwrap(), b"payload");
        assert!(!place.exists("source"));
        assert_eq!(file.content(Io::for_generated_call()), Err(Error::Moved));
    }

    #[test]
    fn exdev_to_a_path_that_exists_fails_with_exists() {
        let place = Place::new("exdev-exists");
        let file = place.stored();
        fs::write(place.target(), b"taken").unwrap();
        assert_eq!(file.move_with(&faulty(&[Step::RenameCrossDevice]), &place.target()), Err(Error::Exists));
        assert_eq!(fs::read(place.target()).unwrap(), b"taken");
        assert_eq!(file.content(Io::for_generated_call()).unwrap(), b"payload");
    }

    #[test]
    fn another_errno_of_the_rename_is_its_error_and_no_copy() {
        let place = Place::new("einval");
        let file = place.stored();
        let error = file.move_with(&faulty(&[Step::RenameInvalid]), &place.target()).unwrap_err();
        assert_eq!((error.kind(), error.errno()), ("io", Some(EINVAL)));
        assert!(!place.exists("target"));
        assert_eq!(file.content(Io::for_generated_call()).unwrap(), b"payload");
    }

    #[test]
    fn a_failed_copy_removes_its_target_and_keeps_the_source() {
        for step in [Step::Copy, Step::ShortCopy, Step::Sync] {
            let place = Place::new("copy");
            let file = place.stored();
            let error = file.move_with(&faulty(&[Step::RenameCrossDevice, step]), &place.target()).unwrap_err();
            let expected = match step {
                Step::Copy => ("storage_full", Some(ENOSPC)),
                Step::Sync => ("io", Some(EIO)),
                _ => ("io", None),
            };
            assert_eq!((error.kind(), error.errno()), expected, "{step:?}");
            assert!(!place.exists("target"), "{step:?}");
            assert_eq!(file.content(Io::for_generated_call()).unwrap(), b"payload", "{step:?}");
        }
    }

    #[test]
    fn a_failed_copy_whose_target_stays_is_partial_target() {
        let place = Place::new("partial");
        let file = place.stored();
        let steps = [Step::RenameCrossDevice, Step::Sync, Step::RemoveTarget];
        let error = file.move_with(&faulty(&steps), &place.target()).unwrap_err();
        assert_eq!((error.kind(), error.errno()), ("partial_target", Some(EIO)));
        assert_eq!(error.removal().map(IoFailure::errno), Some(Some(EACCES)));
        // F-11: the properties of the class of partial_target, which another kind does not have.
        assert_eq!(error.removal_kind(), Ok("permission_denied".to_owned()));
        assert_eq!(error.removal_errno(), Ok(Some(i64::from(EACCES))));
        assert!(matches!(Error::Exists.removal_kind(), Err(Error::Internal(_))));
        assert!(matches!(Error::Exists.removal_errno(), Err(Error::Internal(_))));
        assert_eq!(file.content(Io::for_generated_call()).unwrap(), b"payload");
    }

    #[test]
    fn a_copy_whose_source_stays_is_moved_with_source_left() {
        let place = Place::new("source-left");
        let file = place.stored();
        let error = file.move_with(&faulty(&[Step::RenameCrossDevice, Step::RemoveSource]), &place.target());
        let error = error.unwrap_err();
        assert_eq!((error.kind(), error.errno()), ("source_left", Some(EACCES)));
        assert_eq!(fs::read(place.target()).unwrap(), b"payload");
        assert_eq!(file.content(Io::for_generated_call()), Err(Error::Moved));
    }

    #[test]
    fn a_failed_write_of_a_value_removes_its_target() {
        let place = Place::new("value");
        let file = UploadedFile::from_value("f".into(), "f".into(), String::new(), b"abc".to_vec()).unwrap();
        let error = file.move_with(&faulty(&[Step::Write]), &place.target()).unwrap_err();
        assert_eq!(error.kind(), "storage_full");
        assert!(!place.exists("target"));
        let error = file.move_with(&faulty(&[Step::Sync, Step::RemoveTarget]), &place.target()).unwrap_err();
        assert_eq!(error.kind(), "partial_target");
        fs::remove_file(place.target()).unwrap();
        file.move_with(&faulty(&[Step::None]), &place.target()).unwrap();
        assert_eq!(fs::read(place.target()).unwrap(), b"abc");
    }

    #[test]
    fn a_stored_file_of_another_size_fails_its_read() {
        let place = Place::new("size");
        let file = place.stored();
        fs::write(place.directory.join("source"), b"pay").unwrap();
        let error = file.content(Io::for_generated_call()).unwrap_err();
        assert_eq!((error.kind(), error.errno()), ("io", None));
    }

    #[test]
    fn a_clone_is_the_same_file() {
        let place = Place::new("clone");
        let file = place.stored();
        let other = file.clone();
        file.move_to(Io::for_generated_call(), &place.target()).unwrap();
        assert_eq!(other.content(Io::for_generated_call()), Err(Error::Moved));
        assert_eq!(other.move_to(Io::for_generated_call(), &place.directory.join("again")), Err(Error::Moved));
    }
}
