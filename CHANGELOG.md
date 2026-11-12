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
