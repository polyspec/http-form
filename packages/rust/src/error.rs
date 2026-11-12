//! The errors of the crate (plan F-2): every failure has a kind that never changes and, when a server answers it, an
//! HTTP status. A failure of the file system also has the kind of the table of F-2 and the `errno` of the system.

use std::fmt;
use std::io;

/// The kind of a failure of the file system, decided by the `io::ErrorKind` of the system error (F-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoKind {
    /// `not_found`: a path that does not exist.
    NotFound,
    /// `permission_denied`: the system refused the access.
    PermissionDenied,
    /// `cross_device`: a rename between two file systems.
    CrossDevice,
    /// `storage_full`: no space or no quota left.
    StorageFull,
    /// `io`: any other failure.
    Io,
}

impl IoKind {
    /// The kind text of the failure.
    pub fn kind(self) -> &'static str {
        match self {
            IoKind::NotFound => "not_found",
            IoKind::PermissionDenied => "permission_denied",
            IoKind::CrossDevice => "cross_device",
            IoKind::StorageFull => "storage_full",
            IoKind::Io => "io",
        }
    }
}

/// A failure of the file system: its kind, the `errno` of the system when it gave one, and its message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoFailure {
    kind: IoKind,
    errno: Option<i32>,
    message: String,
}

impl IoFailure {
    /// A failure that the core found without a system error, such as a file whose size is not the size that was
    /// written (H-22). Its kind is `io` and it has no `errno`.
    pub(crate) fn found(message: String) -> Self {
        IoFailure { kind: IoKind::Io, errno: None, message }
    }

    /// The kind of the failure.
    pub fn io_kind(&self) -> IoKind {
        self.kind
    }

    /// The `errno` of the system error, or `None` for a failure that the core found itself.
    pub fn errno(&self) -> Option<i32> {
        self.errno
    }

    /// The message of the system error.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<io::Error> for IoFailure {
    fn from(error: io::Error) -> Self {
        let kind = match error.kind() {
            io::ErrorKind::NotFound => IoKind::NotFound,
            io::ErrorKind::PermissionDenied => IoKind::PermissionDenied,
            io::ErrorKind::CrossesDevices => IoKind::CrossDevice,
            io::ErrorKind::StorageFull | io::ErrorKind::QuotaExceeded => IoKind::StorageFull,
            _ => IoKind::Io,
        };
        IoFailure { kind, errno: error.raw_os_error(), message: error.to_string() }
    }
}

impl fmt::Display for IoFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// A failure of the crate. Every variant has a kind (`kind()`) and, when a server answers the failure of a request
/// with it, a status (`status()`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// `invalid` (400): the body fails with HY-42 or HY-131.5, or a part's `Content-Disposition` has a quoted value
    /// without its end (H-18).
    Invalid,
    /// `header_limit` (400): the header lines of a part are longer than 8192 bytes (HY-113.16).
    HeaderLimit,
    /// `no_boundary` (400): a `multipart/form-data` content type without a boundary or with an empty one (H-17).
    NoBoundary,
    /// `malformed_content_type` (400): the parameters of a `multipart/form-data` content type hold a quoted value
    /// without its end (H-18).
    MalformedContentType,
    /// `invalid_content_length` (400): a `Content-Length` that is not one or more digits (RFC 9110 8.6).
    InvalidContentLength,
    /// `body_limit` (413): the body passed the body limit (HY-59.4, HY-131.11, HY-131.18).
    BodyLimit,
    /// `value_limit` (413): the values held as one value passed the value limit (HY-131.7, HY-131.11).
    ValueLimit,
    /// `part_limit` (413): the body passed the part limit (HY-131.9, HY-131.11).
    PartLimit,
    /// `file_limit` (413): a file passed the file limit (HY-114.7, HY-131.11).
    FileLimit,
    /// `not_finished`: the parts of a body whose end the host did not give (H-4).
    NotFinished,
    /// `not_streamed`: the parts of a body that is held as one value (H-4).
    NotStreamed,
    /// `finished`: a chunk after the end of the body (H-4).
    Finished,
    /// `moved`: a move or a read of a file that was moved (HY-131.16).
    Moved,
    /// `exists`: a move to a path that exists (HY-131.16).
    Exists,
    /// `removed`: a move or a read of a file whose directory was removed, or a chunk after the removal (H-16).
    Removed,
    /// A failure of the file system; its kind is that of [`IoKind`].
    Io(IoFailure),
    /// `source_left` (H-9): the file was copied to its new path, and the removal of its first path failed.
    SourceLeft(IoFailure),
    /// `partial_target` (H-9): the copy or its write to the disk failed, and the removal of the incomplete new file
    /// failed too.
    PartialTarget {
        /// The failure of the copy or of its write to the disk.
        failure: IoFailure,
        /// The failure of the removal of the new file.
        removal: IoFailure,
    },
    /// `internal`: an invariant of the crate does not hold (plan P-4).
    Internal(&'static str),
}

impl Error {
    /// The kind of the failure, which never changes.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::Invalid => "invalid",
            Error::HeaderLimit => "header_limit",
            Error::NoBoundary => "no_boundary",
            Error::MalformedContentType => "malformed_content_type",
            Error::InvalidContentLength => "invalid_content_length",
            Error::BodyLimit => "body_limit",
            Error::ValueLimit => "value_limit",
            Error::PartLimit => "part_limit",
            Error::FileLimit => "file_limit",
            Error::NotFinished => "not_finished",
            Error::NotStreamed => "not_streamed",
            Error::Finished => "finished",
            Error::Moved => "moved",
            Error::Exists => "exists",
            Error::Removed => "removed",
            Error::Io(failure) => failure.io_kind().kind(),
            Error::SourceLeft(_) => "source_left",
            Error::PartialTarget { .. } => "partial_target",
            Error::Internal(_) => "internal",
        }
    }

    /// The status that a server answers for the failure of a request, or `None` for a failure that is not one of the
    /// body of a request.
    pub fn status(&self) -> Option<u16> {
        match self {
            Error::Invalid
            | Error::HeaderLimit
            | Error::NoBoundary
            | Error::MalformedContentType
            | Error::InvalidContentLength => Some(400),
            Error::BodyLimit | Error::ValueLimit | Error::PartLimit | Error::FileLimit => Some(413),
            Error::NotFinished
            | Error::NotStreamed
            | Error::Finished
            | Error::Moved
            | Error::Exists
            | Error::Removed
            | Error::Io(_)
            | Error::SourceLeft(_)
            | Error::PartialTarget { .. }
            | Error::Internal(_) => None,
        }
    }

    /// The `errno` of a failure of the file system: of the failure itself, of the removal that `source_left` reports,
    /// and of the copy that `partial_target` reports (its removal is [`Error::removal`]).
    pub fn errno(&self) -> Option<i32> {
        match self {
            Error::Io(failure) | Error::SourceLeft(failure) | Error::PartialTarget { failure, .. } => failure.errno(),
            Error::Invalid
            | Error::HeaderLimit
            | Error::NoBoundary
            | Error::MalformedContentType
            | Error::InvalidContentLength
            | Error::BodyLimit
            | Error::ValueLimit
            | Error::PartLimit
            | Error::FileLimit
            | Error::NotFinished
            | Error::NotStreamed
            | Error::Finished
            | Error::Moved
            | Error::Exists
            | Error::Removed
            | Error::Internal(_) => None,
        }
    }

    /// The failure of the removal of the new file of `partial_target`.
    pub fn removal(&self) -> Option<&IoFailure> {
        match self {
            Error::PartialTarget { removal, .. } => Some(removal),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(IoFailure::from(error))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Invalid => f.write_str("the form fails with HY-42"),
            Error::HeaderLimit => f.write_str("the header lines of a part are longer than 8192 bytes (HY-113.16)"),
            Error::NoBoundary => f.write_str("a multipart/form-data content type has no boundary (HY-113.11)"),
            Error::MalformedContentType => {
                f.write_str("a parameter of the content type is a quoted string without its end (HY-113.14)")
            }
            Error::InvalidContentLength => f.write_str("the Content-Length is not a number of bytes (RFC 9110 8.6)"),
            Error::BodyLimit => f.write_str("the body passed the body limit"),
            Error::ValueLimit => f.write_str("the text values passed the value limit"),
            Error::PartLimit => f.write_str("the body passed the part limit"),
            Error::FileLimit => f.write_str("a file passed the file limit"),
            Error::NotFinished => f.write_str("the end of the body was not read"),
            Error::NotStreamed => f.write_str("the body is held as one value and has no parts"),
            Error::Finished => f.write_str("the end of the body was read"),
            Error::Moved => f.write_str("the file was moved (HY-131)"),
            Error::Exists => f.write_str("the path exists (HY-131)"),
            Error::Removed => f.write_str("the files of the request were removed (HY-131)"),
            Error::Io(failure) => write!(f, "{failure}"),
            Error::SourceLeft(failure) => {
                write!(f, "the file was copied to its new path, and its first path was not removed: {failure}")
            }
            Error::PartialTarget { failure, removal } => {
                write!(f, "the file was not copied ({failure}), and its incomplete copy was not removed ({removal})")
            }
            Error::Internal(what) => write!(f, "an invariant of http-forms does not hold: {what}"),
        }
    }
}

impl std::error::Error for Error {}
