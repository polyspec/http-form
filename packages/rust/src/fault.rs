//! The fault injection points of the core (spec "Fault injection points", binding G-5, X-1). Each constant names one
//! call of the system, or the invariant of `internal`, as `http_form.<id>`; the contract names the same points in
//! `faults`. `binding_core::fault::point` does nothing without the feature `fault-test`, which only tests turn on.

#[cfg(feature = "fs")]
use std::io;

use binding_core::fault::{point, Injected};

/// `create_directory` of `RequestBody::new` (F-10).
#[doc(hidden)]
pub const CREATE_DIRECTORY: &str = "http_form.create_directory";
/// The new file of a file part, in `feed` (F-10).
#[doc(hidden)]
pub const CREATE_FILE: &str = "http_form.create_file";
/// The write of the content of a file part, in `feed` (F-10, F-15).
#[doc(hidden)]
pub const WRITE_FILE: &str = "http_form.write_file";
/// The size of a written file before it is closed, in `feed` (F-15).
#[doc(hidden)]
pub const FILE_SIZE: &str = "http_form.file_size";
/// The removal of the directory of a request, in `RequestBody::remove` (F-10).
#[doc(hidden)]
pub const REMOVE_DIRECTORY: &str = "http_form.remove_directory";
/// The open and the read of a stored file, in `UploadedFile::content` (F-11, F-15).
#[doc(hidden)]
pub const READ: &str = "http_form.read";
/// The rename of a stored file, in `move_to` (F-11).
#[doc(hidden)]
pub const RENAME: &str = "http_form.rename";
/// The open of the source of a copy, in `move_to` (F-11).
#[doc(hidden)]
pub const OPEN: &str = "http_form.open";
/// The new target of a copy or of a value, in `move_to` (F-11).
#[doc(hidden)]
pub const CREATE: &str = "http_form.create";
/// The copy of a stored file to another file system, in `move_to` (F-11, F-15).
#[doc(hidden)]
pub const COPY: &str = "http_form.copy";
/// The write of the bytes of a value, in `move_to` (F-11).
#[doc(hidden)]
pub const WRITE: &str = "http_form.write";
/// The write of a target to the disk, in `move_to` (F-11).
#[doc(hidden)]
pub const SYNC: &str = "http_form.sync";
/// The removal of the target after a failed copy, write or sync, or else of the source of a copy, in `move_to` (F-11).
#[doc(hidden)]
pub const REMOVE: &str = "http_form.remove";
/// The invariant of the lock of a file: armed as `broken`, the lock is treated as poisoned (X-1, H-8).
#[doc(hidden)]
pub const INTERNAL: &str = "http_form.internal";

/// What an armed point of a call of the system makes the call do.
#[cfg(feature = "fs")]
pub(crate) enum Fault {
    /// The call fails with this error of the system.
    Fail(io::Error),
    /// The read, write or copy handles this many bytes and returns.
    Short(u64),
}

/// The fault armed at the point `name` of a call of the system, if any. An `io` injection is the failure of the
/// system with its `errno`, or a failure without one; `broken` is no injection of such a point, and the contract
/// names no point of a call of the system that takes it.
#[cfg(feature = "fs")]
pub(crate) fn system(name: &'static str) -> Option<Fault> {
    match point(name)? {
        Injected::Io { errno: Some(errno) } => Some(Fault::Fail(io::Error::from_raw_os_error(errno))),
        Injected::Io { errno: None } => Some(Fault::Fail(io::Error::other(format!("the fault of {name}")))),
        Injected::Short { bytes } => Some(Fault::Short(bytes)),
        Injected::Broken => None,
    }
}

/// Fails the call of the point `name` when an `io` fault is armed there; a point that takes no `short` uses it.
#[cfg(feature = "fs")]
pub(crate) fn fail(name: &'static str) -> io::Result<()> {
    match system(name) {
        Some(Fault::Fail(error)) => Err(error),
        Some(Fault::Short(_)) | None => Ok(()),
    }
}

/// True when the invariant of the point `name` is armed as broken.
pub(crate) fn broken(name: &'static str) -> bool {
    matches!(point(name), Some(Injected::Broken))
}

/// The first `bytes` of `data`, for a `short` fault of a write.
#[cfg(feature = "fs")]
pub(crate) fn prefix(data: &[u8], bytes: u64) -> &[u8] {
    match usize::try_from(bytes) {
        Ok(bytes) if bytes < data.len() => &data[..bytes],
        Ok(_) | Err(_) => data,
    }
}
