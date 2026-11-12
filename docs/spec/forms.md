# http-forms: rules and API

This library reads the form values and the `multipart/form-data` bodies of hyper. The rules are hyper's, and this
document names them by hyper's identifiers rather than repeating them: HY-56 (query and urlencoded values), HY-57
(form values), HY-113 (file parts, header lines and parameters), HY-114 (file limit) and HY-131 (reading in chunks,
limits, the files of a request). Where this library changes one of those rules, the change is a rule of this document
(F-n) and the section "Changes to hyper's rules" lists it. A content type is a media type, whose grammar is the rules
MT-1 to MT-9 of the dependency `polyspec-http-media-type` (RFC 9110, sections 8.3.1 and 5.6.6); this document names
them by those identifiers.

`contracts/fields.json` is a copy of hyper's `contracts/fields.json`; `contracts/sources.json` holds its source and
sha256, which `tests/sources.rs` checks. `contracts/forms.json` holds the cases of the rules of this library. A case of
`fields.json` that a rule of this document changes is listed in the group `superseded` of `forms.json` with that rule
and the case that replaces it, which reads the same input; the runner checks that input and runs the replacing case.

## One implementation

- **F-1** Every rule is in the Rust crate `polyspec-http-forms` (`packages/rust/`): the urlencoded reader, the content
  types, the header lines and parameters of a part, the state machine that reads a body in chunks, the value, part
  and file limits, the directory of a request, the writing, reading, moving and removal of its files, and the body of
  a request as its host reads it (`RequestBody`).
- **F-2** A binding converts types and turns an error of the core into the class of its runtime. It holds no rule: no
  parsing, no limit, no file operation and no message of its own. The bindings are generated from the interface
  contract; the hand-written bindings in `php/`, `python/` and `node/` are not built.
- **F-3** What only a runtime can do stays in the server of that runtime: reading the bytes of a request from its host
  and passing them, with the values of `Content-Type` and `Content-Length`, to `RequestBody`.

## Bytes and text

- **F-4** A content type, a `Content-Length`, a boundary and a body are bytes. A request without `Content-Type` gives
  the core no content type (`None`), which is another value than the empty bytes.
- **F-5** Names, values, filenames and content types that the core returns are UTF-8 text, which the core checked
  (HY-42, HY-113.5). The bytes of a file are bytes, also the bytes of a file that an application makes
  (`UploadedFile::from_value`, HY-113.3).

## Errors

Every failure is an `Error` with a kind that never changes (plan F-2). A failure of a request body has the status that
a server answers. The category is the error class of the binding (plan F-3).

| Kind | Status | Category | Failure |
|---|---|---|---|
| `invalid` | 400 | value | HY-42, HY-113.5, HY-131.5; a quoted parameter value of a `Content-Disposition` without its end (F-6); a part that is no `form-data` part with a name, or that gives a parameter twice (F-16) |
| `header_limit` | 400 | value | header lines of a part longer than 8192 bytes (HY-113.16) |
| `no_boundary` | 400 | value | a `multipart/form-data` content type without a boundary or with an empty one (F-7) |
| `malformed_content_type` | 400 | value | a `Content-Type` that is no media type (F-12) |
| `invalid_content_length` | 400 | value | a `Content-Length` that is not one or more digits (RFC 9110, section 8.6) |
| `body_limit` | 413 | value | the body limit (HY-59.4, HY-131.11, HY-131.18) |
| `value_limit` | 413 | value | the value limit (HY-131.7) |
| `part_limit` | 413 | value | the part limit (HY-131.9, HY-131.12) |
| `file_limit` | 413 | value | the file limit (HY-114.7) |
| `not_finished` | none | type | `parts()` before `finish()` (F-9) |
| `not_streamed` | none | type | `parts()` of a body held as one value (F-9) |
| `finished` | none | type | a chunk after `finish()` (F-9) |
| `moved` | none | value | a move or a read of a moved file (HY-131.16), message `the file was moved (HY-131)` |
| `exists` | none | value | a move to a path that exists (HY-131.16), message `the path exists (HY-131)` |
| `removed` | none | value | a move or a read of a stored file after the removal of its directory, a chunk after it (F-10) |
| `not_found`, `permission_denied`, `cross_device`, `storage_full`, `io` | none | io | a failure of the file system, by its `io::ErrorKind`, with its `errno` (plan F-2); `io` without `errno` for a size check (F-12) |
| `source_left` | none | io | a copied file whose source was not removed (F-11) |
| `partial_target` | none | io | a failed copy or write whose incomplete target was not removed (F-11) |
| `internal` | none | internal | an invariant of the core does not hold (H-8) |

The kind `invalid_number` (category value) of a limit that is no unsigned integer, and `invalid_path` of a value that
is no path, are the binding's (plan B-2, B-3).

## Content types and parameters

- **F-6** A quoted parameter value of a `Content-Disposition` is the text between its `"` and the next `"` (HY-113.14).
  A quoted value without its next `"` fails with `invalid`. A browser sends a `"` of a filename as `%22`, so no client
  of a form sends such a value; reading it to the end of the text gave a value that the client did not send (plan
  P-3). A quoted value of a content type follows MT-3.3 and MT-3.4 (F-12).
- **F-7** A `multipart/form-data` content type without a `boundary` parameter, or with an empty one, fails with
  `no_boundary` (400). Such a body cannot be read, and reading it as a body of another type dropped it (plan P-4).
- **F-12** The value of `Content-Type` is read by `polyspec_http_media_type::parse` (MT-4), and by no other reader of
  this library. A value that is no media type, the empty value included, fails with `malformed_content_type` (400);
  the error holds the failure of `parse` as its source, whose kind `malformed_media_type` has no status because its
  caller decides it (MT-8.1). A request without `Content-Type` has no media type (F-4).
  1. `media_type(content type)` is the type, `/` and the subtype in lowercase (MT-1.2), without the parameters, and the
     empty text for a request without `Content-Type` (HY-59.11). HY-59.7 compares it with the form types.
  2. `is_urlencoded(content type)` is true when the type and subtype are `application/x-www-form-urlencoded`, and false
     for another type or none (HY-57.10).
  3. `multipart_boundary(content type)` is, when the type and subtype are `multipart/form-data`, the value of its
     `boundary` parameter after MT-3.4, or `no_boundary` (F-7); for another type or none it is `None` (HY-113.11). A
     parameter given twice, compared without case, fails the media type (MT-3.5), `boundary` included.
  4. A parameter that its type does not define changes nothing (I-7), because it does not change how the body is
     read: `application/x-www-form-urlencoded` defines no parameter, and its parser decodes UTF-8, "only UTF-8 is
     conforming" (WHATWG URL, section 5, 5.1), so a `charset` changes nothing either and a body that is not UTF-8 fails
     with `invalid` (HY-42); `multipart/form-data` defines `boundary` alone, its optional parameters being "none"
     (RFC 7578, section 8).

## Reading a body

- **F-8** The first failure of a body is the failure whose byte comes first (HY-131.6). The byte of a failure is the
  first byte after which the bytes read so far determine it:

  | Failure | Its byte |
  |---|---|
  | `body_limit` | the byte after `limit` bytes |
  | `value_limit` of a body held as one value | the byte after `valueLimit` bytes |
  | `part_limit` | the LF of the CRLF after the delimiter of the part past the limit |
  | `value_limit` of a name | the last byte of the empty line that ends the header lines of the part |
  | `value_limit`, `file_limit` of content | the content byte past the limit |
  | `invalid` of a value that is not UTF-8 | the byte after which the content is no start of UTF-8 text, or the end of the content for a sequence that the end cuts off |
  | `invalid` after a delimiter | the first byte that is neither the `-` nor the CR of `--` and CRLF, or the second byte that does not complete them |
  | `invalid` of header lines (HY-113.5, HY-131.5, F-6, F-16) | the last byte of the empty line that ends them |
  | `header_limit` | the byte after which no empty line can start within 8192 bytes of the header lines |
  | `invalid` of a body without its end | the end of the body (`finish()`) |

  A content byte that may start a delimiter (`CRLF--` and the boundary) is held until a later byte shows that it does
  not; it is content, and counts for the limits and the UTF-8 of a value, from that later byte. When two failures have
  one byte, a limit (413) comes before a failure of HY-42 (400), and `body_limit` before the other limits. A chunk that
  passes the body limit is read up to the limit, and `body_limit` is its failure only when those bytes do not fail.
  The result therefore does not depend on the chunks, also when the body fails.
- **F-9** `RequestBody::new(content type, Content-Length, stream, limits)` starts a body; `limits` is the record
  `BodyLimits` of the body limit, the value limit, the part limit and the file limit. A `Content-Length` that is no number fails with `invalid_content_length`, and one above the limit fails with
  `body_limit` before a byte is read (HY-131.18); a number of more digits than 64 bits hold is above every limit. A
  body read in chunks (`stream` and the type `multipart/form-data`) needs its boundary (F-6, F-7) and gets its
  directory. `feed(chunk)` reads the next chunk; its first failure (F-8) is the error of that call and of every later
  call, and the body then holds no bytes and no open file (HY-131.11). The host calls `finish()` after the last chunk:
  a body read in chunks that did not end fails with `invalid` (HY-131.5); a second `finish()` gives the result of the
  first, and a chunk after it fails with `finished`. `parts()` gives the fields and the files of a body read in
  chunks; it fails with `not_streamed` for a body held as one value, with the first failure of the body, and with
  `not_finished` before `finish()`. A body held as one value gives its bytes as `held()`, which the host reads with
  `read_body`, `fields_from_body` and `files_from_body`.
- **F-16** Every part of a `multipart/form-data` body has one `Content-Disposition` line whose type is `form-data`
  and which has a `name` parameter (RFC 7578, section 4.2), at most one `Content-Type` line, no header line without
  `:` (RFC 5322, section 2.2: a header field is a name, a colon and a body), and no parameter of its `Content-Disposition` is given twice, its name
  compared without case (RFC 6266, section 4.1: such a value is invalid). A part without `Content-Disposition`, with
  two `Content-Disposition` or two `Content-Type` lines (no rule says which one counts), with a line without `:`, of another disposition type, without `name` or with
  a parameter given twice fails the body with `invalid` at the byte
  of its header lines (F-8). It is no ignored input: ignoring it dropped a part that the client sent (plan P-4). A part
  that fails so has no name, so it counts for no value limit.
- **F-13** A failure of the sink of a `MultipartReader`, such as a write of a file, is the failure of that call; the
  reader then reads nothing and every later call gives `SinkFailed`. It is not a failure of the body (H-6).

## Files

- **F-10** The directory of a request is created below the temporary directory of the system by `tempfile` with the
  name `http-forms-` and random characters, which `tempfile` draws again when the name exists, and with the mode
  `0700` (HY-131.13). Its files have the mode `0600` and are closed on `exec`. `RequestBody::remove()` removes the
  directory (HY-131.14); the directory belongs to the request alone, so a directory that is gone fails with
  `not_found`, and after a removal a second call does nothing. After the removal every stored file of the body,
  through every `UploadedFile` that `parts()` gave, fails with `removed`, and so does a chunk. A body dropped without
  `remove()` removes its directory; a failure of that removal reaches no caller, and the binding warns that `remove()`
  was not called (plan B-10).
- **F-11** `UploadedFile::move_to(path)` moves a file without replacing a file at `path` (HY-131.15). A stored file is
  renamed by `renameat2` with `RENAME_NOREPLACE` on Linux and by `renameatx_np` with `RENAME_EXCL` on macOS (through
  `rustix::fs::renameat_with`); `EEXIST` fails with `exists`. Only `EXDEV`, a path on another file system, takes
  another way, because only that errno tells that a rename cannot move the file (plan P-3): the copy opens a new file
  at `path` (`EEXIST` fails with `exists`), copies, writes it to the disk (`sync_all`) and removes the source. When the
  copy, its size or the write to the disk fails, the copy removes the file that it created and gives the failure; when
  that removal fails too the error is `partial_target` with both failures. When only the removal of the source fails,
  the file counts as moved and the error is `source_left`. Every other errno, `EINVAL` of a file system without the
  flag and `ENOSYS` included, is the error of the move. A file whose bytes are a value is written to a new file and to
  the disk, and a failed write removes that file likewise. Every clone of an `UploadedFile`, and every file that
  `parts()` gives for one part, is the same file: a move through one is a move of all (H-16).
- **F-15** `std` does not report a failure of `close(2)`, so the core checks sizes instead (H-22): the size of a
  written file before it is closed, and the number of bytes that `content()` reads, are the number of bytes that were
  written; another number fails with `io` without `errno`. Every number of bytes is converted with a range check.
- **F-17** A public function that does blocking I/O takes the wait token `bindgen_core::Io` of the dependency
  `polyspec-bindgen-core` as its first argument (bindgen design I-2 (4), plan B-5 (a)): `RequestBody::new` (it creates
  the directory), `feed` (it writes files), `finish` (it closes the file of a failed body), `remove`,
  `UploadedFile::content` and `UploadedFile::move_to`. Only generated code makes a token, through the feature
  `construct`, which the tests of this crate enable as a dev-dependency. No other public function does I/O. `Drop` of
  `RequestBody` removes the directory of a body whose host did not call `remove` (F-10) without a token, because
  `Drop` takes no argument; it is the safety net of a missing cleanup call, which the binding reports (plan B-10).
- **F-14** Moving without replacing needs `renameat2` or `renameatx_np`, so the crate builds for Linux and macOS only
  (plan T-2).

## Ignored inputs (plan P-4)

| Id | Rule | Bytes | Basis | Case |
|---|---|---|---|---|
| I-1 | HY-131.3 | the bytes before the first delimiter | RFC 2046, section 5.1.1: the preamble is to be ignored | `fields.json` multipart `the_bytes_before_the_first_delimiter_are_ignored` |
| I-2 | HY-131.4 | the bytes after the `--` that ends the body | RFC 2046, section 5.1.1: the epilogue is to be ignored | `fields.json` multipart `the_bytes_after_the_closing_delimiter_are_ignored` |
| I-4 | HY-113.7 | the parameter `filename*` of a `Content-Disposition` | RFC 7578, section 4.2: `filename*` must not be used, and the `filename` parameter alone names the file | `forms.json` multipart `a_filename_star_is_ignored` |
| I-5 | HY-113.12 | header lines of a name other than `Content-Disposition` and `Content-Type` | RFC 7578, section 4.8: other header fields must be ignored | `fields.json` multipart `a_part_header_of_8192_bytes_is_read`, `forms.json` multipart `other_header_lines_are_ignored` |
| I-6 | HY-56 | an empty part between two `&` | WHATWG URL, application/x-www-form-urlencoded parsing: an empty sequence is skipped | `fields.json` urlencoded `empty_parts_are_skipped` |
| I-7 | F-12 | the parameters that a form type does not define: every parameter of `application/x-www-form-urlencoded`, and those of `multipart/form-data` other than `boundary` | WHATWG URL, section 5: the type has no parameter and its parser decodes UTF-8; RFC 7578, section 8: optional parameters "none" | `forms.json` multipart `the_charset_of_an_urlencoded_type_is_ignored`, `another_parameter_of_an_urlencoded_type_is_ignored`, `fields.json` multipart `a_boundary_inside_a_quoted_string_is_no_boundary` |

The reader of a media type ignores the inputs of the table of http-media-type (MT-1.3, MT-3.2, MT-3.4, MT-2.2):
whitespace around the value and around `;`, empty parameters, the quotes of a quoted value and the case of names.
I-3, a part that is no `form-data` part with a name, is no row: such a part fails (F-16).

The code marks each place with its id.

## Changes to hyper's rules

These rules of hyper change with this library; hyper changes its sentences and cases when it adopts it (plan S-3b):

- HY-113.11 and HY-57.4 (F-7): a `multipart/form-data` type without a boundary or with an empty one is a 400 with
  `no_boundary`, not a body without values. Cases: `a_quoted_boundary_of_no_character_is_no_boundary`,
  `a_multipart_type_without_a_boundary_has_no_values`.
- HY-113.14 (F-6): "or the rest of the text when there is no next one" becomes a failure. Case:
  `a_quoted_string_without_its_end_is_the_rest_of_the_text`.
- HY-113.16 (H-19): the failure of long header lines is `header_limit`, still 400.
- HY-131.6, HY-131.11, HY-131.18 (F-8, F-9): the first failure is the one whose byte comes first; a body read as one
  chunk that has an invalid byte before the body limit is now a 400, not a 413; a `Content-Length` that is no number is
  a 400. The host passes the `Content-Length` to the core and calls `finish()`; `feed` fails instead of returning a
  boolean, and `parts()` fails instead of returning null (H-4).
- HY-131.16 (F-10): a stored file fails with `removed` after the removal of its directory.
- HY-131.21 (plan H-12): replaced by the Node host rule of plan H-12.
- HY-59.11, HY-59.7, HY-57.10, HY-113.11 (F-12): the media type of `Content-Type` is read by MT-4. `mediaType()` is a
  function of this library. A `Content-Type` that is no media type fails with `malformed_content_type` (400), where
  HY-59.7 answered 415 and HY-57.10 and HY-113.11 gave a body of another type: a line feed or the byte A0 around the
  value, a byte that is no token character in a name, an unclosed quoted value, a parameter given twice, and the empty
  value. SP and HTAB before the media type are ignored also for HY-57.10, which refused them. A quoted value of a
  content type unquotes its quoted pairs (MT-3.4), so a backslash of a quoted boundary is no part of it. Cases: the
  group `content_type` and the replacements of `an_urlencoded_type_followed_by_a_line_feed_is_another_type` and
  `a_body_without_a_type_has_no_values`.
- HY-113.6, HY-113.12, HY-113.13 (F-16): a part without `Content-Disposition`, with two `Content-Disposition` or
  `Content-Type` lines or a line without `:`, of another disposition type or without `name`, and a parameter given
  twice fail the body with `invalid`, where they were ignored and the first line and the first parameter of a name
  counted. Cases: the replacements in the group `superseded` whose rule is F-16.

## API

| Core | Purpose |
|---|---|
| `parse_urlencoded(text)` | HY-56: the fields, or `invalid` |
| `media_type(content type)` | HY-59.11, F-12 |
| `is_urlencoded(content type)` | HY-57.10, F-12 |
| `multipart_boundary(content type)` | HY-113.11, F-7, F-12 |
| `read_body(content type, body)`, `fields_from_body`, `files_from_body` | HY-57, HY-113: a body held as one value |
| `parse_multipart(body, boundary)`, `parse_multipart_chunks(chunks, boundary)`, `Collector`: `take`, `collect`, `into_form` | HY-131.6 |
| `MultipartReader::new(boundary, limits)`: `feed`, `feed_with`, `finish`, `closed`, `buffered`, `parts` | HY-131, F-8, F-13 |
| `RequestBody`: `new`, `feed`, `finish`, `parts`, `held`, `size`, `streamed`, `directory`, `remove`; `BodyLimits` | F-9, F-10, F-17 |
| `Form`: `fields`, `files`; `Fields`, `Files`: lists of a name and its values (HY-56.7) | HY-57, HY-113.2 |
| `file_over(files, limit)` | HY-114.9: true when a file has more than `limit` bytes |
| `UploadedFile`: `from_value`, `name`, `filename`, `content_type`, `size`, `content`, `move_to` | HY-113.3, HY-131.15, F-11, F-17 |
| `Error`: `kind`, `status`, `errno`, `removal`, `removal_kind`, `removal_errno` | the table of Errors, F-11 |

## Interface contract

- **F-18** The interface contract is `packages/rust/interface.json` (bindgen design I-1), in the schema of
  `@polyspec/bindgen`. It declares the members of the table above that a host uses (the functions, `Form`,
  `RequestBody`, `UploadedFile`), the error classes `FormsValueError`, `FormsTypeError`, `FormsIoError`,
  `FormsStoreError`, `FormsBusyError` and `FormsInternalError` of the categories of the table of Errors (design E-2),
  the error class `PartialTargetError` of `partial_target`, and the differences from the earlier bindings in `changes`.
  `MultipartReader`, `parse_multipart` and `parse_multipart_chunks` stay Rust API (F-3). `malformed_content_type` is a
  kind of this library, with the status 400 that a request with such a `Content-Type` gets; its error holds the
  failure of http-media-type (F-12), so the contract names no kind of that library and forwards no property of it.
  `RequestBody` implements `bindgen_core::Cleanup` (its cleanup call is `remove`) and `Error` implements
  `bindgen_core::Kinds`.
  1. The core functions take and return the Rust types of the contract types (bindgen design I-2 (1), T-1, T-2): a
     result is an owned value (`String`, `Vec<u8>`, `PathBuf`, `Vec`), `Fields` and `Files` are the lists
     `list<tuple<text, list<…>>>`, and `Form` and `BodyLimits` are records (design T-4), so `file_over` is a function
     of the files and not a member of `Form`. Every member of the contract is a function of the core, the properties
     `removalKind` and `removalErrno` of `PartialTargetError` included, which fail with `internal` on another kind.

## Verification

- `make test`: the cases of `contracts/fields.json` and `contracts/forms.json` in the core; every multipart case and
  every body case is also read byte by byte and in two chunks divided at every position, and the reader holds between
  two chunks at most its bound (HY-131.6). Random bodies with random limits give the result of the body read byte by
  byte in every division (`HTTP_FORMS_ROUNDS`, 3000 by default). A move to another file system needs one at
  `HTTP_FORMS_OTHER_FS` (`/dev/shm` by default). The failures of each step of a move are tested with operations that
  fail on purpose.
- `make clippy`: the lints of plan R-1 and `as_conversions`.
- The interface contract passes `checkContract` of `@polyspec/bindgen` with the contract of `polyspec-http-media-type`
  as its dependency, with no finding.
- `make differential HYPER=<dir> PYTHONPATH=...`: random bodies through the core and through hyper's Python reader. It
  reads a module of hyper that is not public, so its result is a reference only until hyper exports the reader (plan
  H-25); a difference that F-6 or F-7 explains is counted apart.
