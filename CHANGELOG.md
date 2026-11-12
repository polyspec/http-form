# Changelog

## Unreleased

- The core crate is `polyspec-http-forms` in `packages/rust/`, and the rules are `docs/spec/forms.md`.
- Every failure is an `Error` with a kind, a status and, for the file system, an `errno`; nothing fails as `None`.
- `RequestBody::new` takes the `Content-Length`; `feed` fails with the first failure of the body, by the position of
  its byte, and every later call gives it; the host calls `finish`; `parts` fails with `not_finished`,
  `not_streamed` or the failure of the body.
- A `multipart/form-data` type without a boundary fails with `no_boundary`; a quoted parameter value without its end
  fails; long header lines fail with `header_limit`.
- `Form::file_over(limit)` replaces `largest_file`.
- `UploadedFile` is shared by its clones; a move never replaces a file, copies only to another file system, and
  reports `source_left` and `partial_target`; after `remove` a stored file fails with `removed`.
- The directory of a request is made by `tempfile` and named `http-forms-…`; a body dropped without `remove`
  removes it.
- The hand-written PHP, Python and Node bindings are no longer built.
- `RequestBody::new`, `feed`, `finish`, `remove`, `UploadedFile::content` and `move_to` take the wait token
  `bindgen_core::Io` as their first argument (F-17).
- The `Content-Type` is read as a media type by `polyspec-http-media-type` (F-12): `media_type()` gives `mediaType()`;
  a value that is no media type, such as one with a line feed or the byte A0 around it, an unclosed quote, a parameter
  given twice or the empty value, fails with `malformed_content_type` (400) and holds the failure of the media type;
  spaces before an urlencoded type are ignored; a quoted boundary unquotes its quoted pairs. The content type
  arguments are `Option<&[u8]>`, where `None` is a request without `Content-Type`.
- A part without `Content-Disposition`, of another disposition type or without `name`, and a parameter given twice in
  a `Content-Disposition`, fail the body with `invalid` instead of being ignored (F-16).
- `HEADER_LIMIT` is a `u64`. `Error` implements `bindgen_core::Kinds` and `RequestBody` implements
  `bindgen_core::Cleanup`.
- The interface contract is `packages/rust/interface.json` (F-18).
