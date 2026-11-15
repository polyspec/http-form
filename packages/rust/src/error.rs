//! The errors of the crate (plan F-2): every failure has a kind that never changes and, when a server answers it, an
//! HTTP status. A failure of the file system also has the kind of the table of F-2 and the `errno` of the system.

use std::fmt;
use std::io;

/// The kind of a failure of the file system, decided by the `io::ErrorKind` of the system error (F-2). `EXDEV` is
/// `io`: only a rename gives it, and the move then copies (F-11), so no kind of its own reaches a caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoKind {
    /// `not_found`: a path that does not exist.
    NotFound,
    /// `permission_denied`: the system refused the access.
    PermissionDenied,
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
    #[cfg(feature = "fs")]
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
    /// `invalid` (400): the body fails with F-20.3, F-21.5, F-22.4 or F-24.5, or a part's `Content-Disposition` has a quoted value
    /// without its end (H-18).
    Invalid,
    /// `header_limit` (400): the header lines of a part are longer than 8192 bytes (F-23.4).
    HeaderLimit,
    /// `no_boundary` (400): a `multipart/form-data` content type without a boundary or with an empty one (H-17).
    NoBoundary,
    /// `malformed_content_type` (400): the `Content-Type` is no media type (spec F-12). It holds the failure of
    /// `polyspec_http_media_type::parse` (MT-4), which has no status because its caller decides it (MT-8.1).
    MalformedContentType(polyspec_http_media_type::Error),
    /// `invalid_content_length` (400): a `Content-Length` that is not one or more digits (RFC 9110 8.6).
    InvalidContentLength,
    /// `body_limit` (413): the body passed the body limit (F-25.1).
    BodyLimit,
    /// `value_limit` (413): the values held as one value passed the value limit (F-25.2).
    ValueLimit,
    /// `part_limit` (413): the body passed the part limit (F-25.3).
    PartLimit,
    /// `file_limit` (413): a file passed the file limit (F-25.4).
    FileLimit,
    /// `not_finished`: the parts of a body whose end the host did not give (H-4).
    NotFinished,
    /// `not_streamed`: the parts of a body that is held as one value (H-4).
    NotStreamed,
    /// `finished`: a chunk after the end of the body (H-4).
    Finished,
    /// `moved`: a move or a read of a file that was moved (F-11).
    Moved,
    /// `exists`: a move to a path that exists (F-11).
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
            Error::MalformedContentType(_) => "malformed_content_type",
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
            | Error::MalformedContentType(_)
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
            | Error::MalformedContentType(_)
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

    /// The kind of the failure of the removal of the new file of `partial_target` (spec F-11), the property
    /// `removalKind` of its error class; `internal` for another kind, whose class has no such property.
    pub fn removal_kind(&self) -> Result<&'static str, Error> {
        match self.removal() {
            Some(removal) => Ok(removal.io_kind().kind()),
            None => Err(Error::Internal("the removal of a failure that is not partial_target")),
        }
    }

    /// The `errno` of the failure of the removal of the new file of `partial_target` (spec F-11), the property
    /// `removalErrno` of its error class: `None` when the system gave none; `internal` for another kind.
    pub fn removal_errno(&self) -> Result<Option<i64>, Error> {
        match self.removal() {
            Some(removal) => Ok(removal.errno().map(i64::from)),
            None => Err(Error::Internal("the removal of a failure that is not partial_target")),
        }
    }
}

/// The kind interface that generated code reads (design I-2 (3), E-3).
impl binding_core::Kinds for Error {
    fn kind(&self) -> &'static str {
        Error::kind(self)
    }

    fn status(&self) -> Option<u16> {
        Error::status(self)
    }

    fn errno(&self) -> Option<i32> {
        Error::errno(self)
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
            Error::Invalid => f.write_str("the body is no valid form (F-20 to F-24)"),
            Error::HeaderLimit => f.write_str("the header lines of a part are longer than 8192 bytes (F-23.4)"),
            Error::NoBoundary => f.write_str("a multipart/form-data content type has no boundary (F-12.3)"),
            Error::MalformedContentType(_) => {
                f.write_str("the Content-Type is not a media type of RFC 9110, section 8.3.1 (MT-4)")
            }
            Error::InvalidContentLength => f.write_str("the Content-Length is not a number of bytes (RFC 9110 8.6)"),
            Error::BodyLimit => f.write_str("the body passed the body limit"),
            Error::ValueLimit => f.write_str("the text values passed the value limit"),
            Error::PartLimit => f.write_str("the body passed the part limit"),
            Error::FileLimit => f.write_str("a file passed the file limit"),
            Error::NotFinished => f.write_str("the end of the body was not read"),
            Error::NotStreamed => f.write_str("the body is held as one value and has no parts"),
            Error::Finished => f.write_str("the end of the body was read"),
            Error::Moved => f.write_str("the file was moved (F-11)"),
            Error::Exists => f.write_str("the path exists (F-11)"),
            Error::Removed => f.write_str("the files of the request were removed (F-10)"),
            Error::Io(failure) => write!(f, "{failure}"),
            Error::SourceLeft(failure) => {
                write!(f, "the file was copied to its new path, and its first path was not removed: {failure}")
            }
            Error::PartialTarget { failure, removal } => {
                write!(f, "the file was not copied ({failure}), and its incomplete copy was not removed ({removal})")
            }
            Error::Internal(what) => write!(f, "an invariant of http-form does not hold: {what}"),
        }
    }
}

impl std::error::Error for Error {
    /// The failure of `polyspec_http_media_type::parse` that `malformed_content_type` holds (spec F-12).
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::MalformedContentType(failure) => Some(failure),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    //! F-2: the kinds of `Io` are the kinds that the core gives, and the contract names them.

    use super::*;

    const EXDEV: i32 = 18;

    /// The kinds of `IoKind`, which the contract names in the row of `Error::Io`.
    const KINDS: [&str; 4] = ["not_found", "permission_denied", "storage_full", "io"];

    #[test]
    fn exdev_is_io_with_its_errno() {
        // EXDEV comes from a rename alone, which takes the copy (F-11), so it reaches no caller with a kind of its own.
        let failure = IoFailure::from(io::Error::from_raw_os_error(EXDEV));
        assert_eq!((failure.io_kind().kind(), failure.errno()), ("io", Some(EXDEV)));
    }

    #[test]
    fn the_contract_names_the_kinds_of_io() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/interface.json");
        let text = std::fs::read_to_string(path).unwrap();
        let contract: serde_json::Value = serde_json::from_str(&text).unwrap();
        let rows = contract["errors"]["kinds"].as_array().unwrap();
        let row = rows.iter().find(|row| row["variant"] == "polyspec_http_form::Error::Io").unwrap();
        assert_eq!(row["kinds"], serde_json::json!(KINDS));
    }
}
