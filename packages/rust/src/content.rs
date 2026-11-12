//! The content type of a request (HY-57.10, HY-113.11).
//!
//! The grammar of a media type (RFC 9110, sections 8.3.1 and 5.6.6) belongs to the library `http-media-type`, which
//! this crate is to read every content type with (H-28, spec F-12). Until then these functions keep the reading of
//! HY-57.10 and HY-113.11 (`header.rs`), and no `mediaType()` (HY-59.11) is part of this crate.

use crate::{header, Error};

/// True when the content type is `application/x-www-form-urlencoded` (HY-57.10); a request without `Content-Type`
/// has another type.
pub fn is_urlencoded(content_type: Option<&[u8]>) -> Result<bool, Error> {
    match content_type {
        Some(value) => Ok(header::is_urlencoded(value)),
        None => Ok(false),
    }
}

/// The boundary of a content type (HY-113.11): `None` for another type or none, and for `multipart/form-data` its
/// `boundary` parameter, or `no_boundary` (H-17) or `malformed_content_type` (H-18).
pub fn boundary(content_type: Option<&[u8]>) -> Result<Option<Vec<u8>>, Error> {
    match content_type {
        Some(value) => header::boundary(value),
        None => Ok(None),
    }
}
