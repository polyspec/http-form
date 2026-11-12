//! The content type of a request (spec F-12): its media type, read by `polyspec_http_media_type::parse` (MT-4), which
//! gives `mediaType()` (HY-59.11), the comparison of HY-59.7, the urlencoded type (HY-57.10) and the boundary
//! (HY-113.11). A request without `Content-Type` has no media type; a value that is no media type fails with
//! `malformed_content_type`.

use polyspec_http_media_type::{parse, MediaType};

use crate::Error;

/// Reads the value of `Content-Type` (MT-4); a failure is `malformed_content_type`, which holds it (spec F-12).
fn read(content_type: &[u8]) -> Result<MediaType, Error> {
    parse(content_type).map_err(Error::MalformedContentType)
}

/// True when the media type is `type/subtype`; the reader gives both in lowercase (MT-1.2).
fn is(media_type: &MediaType, r#type: &str, subtype: &str) -> bool {
    media_type.r#type == r#type && media_type.subtype == subtype
}

/// The media type of the content type, its type and subtype in lowercase without parameters, or the empty text for a
/// request without `Content-Type` (HY-59.11).
pub fn media_type(content_type: Option<&[u8]>) -> Result<String, Error> {
    match content_type {
        Some(value) => {
            let media_type = read(value)?;
            Ok(format!("{}/{}", media_type.r#type, media_type.subtype))
        }
        None => Ok(String::new()),
    }
}

/// True when the media type is `application/x-www-form-urlencoded` (HY-57.10); a request without `Content-Type` has
/// another type. The type defines no parameter, so its parameters are an ignored input (spec I-7).
pub fn is_urlencoded(content_type: Option<&[u8]>) -> Result<bool, Error> {
    match content_type {
        Some(value) => Ok(is(&read(value)?, "application", "x-www-form-urlencoded")),
        None => Ok(false),
    }
}

/// The boundary of a content type (HY-113.11): `None` for another type or none, and for `multipart/form-data` the
/// value of its `boundary` parameter (MT-3.4), or `no_boundary` when it has none or an empty one (F-7). A parameter
/// given twice fails the media type (MT-3.5); every other parameter is an ignored input, because the type defines
/// `boundary` alone (spec I-7).
pub fn boundary(content_type: Option<&[u8]>) -> Result<Option<Vec<u8>>, Error> {
    let Some(value) = content_type else {
        return Ok(None);
    };
    let media_type = read(value)?;
    if !is(&media_type, "multipart", "form-data") {
        return Ok(None);
    }
    match media_type.parameters.into_iter().find(|parameter| parameter.name == "boundary") {
        Some(parameter) if !parameter.value.is_empty() => Ok(Some(parameter.value)),
        Some(_) | None => Err(Error::NoBoundary),
    }
}
