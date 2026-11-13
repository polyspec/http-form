<!-- doc-id: spec-forms -->
# http-form: rules and API

This library reads form values and `multipart/form-data` bodies: the `application/x-www-form-urlencoded` text of a
query or a body, the values and files of a multipart body, a body read in chunks with its limits, and the files of a
request. Its rules are the rules F-n of this document. A server that uses the library, such as hyper, decides what
belongs to it alone and points to these rules: the values and defaults of the limits, the form types of an
application, the order of the checks of a request, its API and its host (section "What the server decides"). A
content type is a media type, whose grammar is the rules MT-1 to MT-9 of the dependency `polyspec-http-media-type`
(RFC 9110, sections 8.3.1 and 5.6.6); this document names them by those identifiers.

`contracts/fields.json` and `contracts/forms.json` hold the cases of these rules; `fields.json` holds the cases of
urlencoded text, multipart bodies, bodies read in chunks and files made from bytes, and `forms.json` the cases of the
other rules. A case follows the case convention of binding (G-5): its keys are the parameter names of the member that
its group names in the interface contract, bytes are `{"hex": ...}`, a content type is a text whose characters are its
bytes, and the limits are the record `limits`. A method of `RequestBody` takes its receiver from the keys of the
constructor, so `request_body.feed` holds the failures of one chunk, `request_body.finish` and `request_body.parts` the
failures of a body that was just made. A scenario of more than one call (`body` of `forms.json`, `streamed` of
`fields.json`) is a case of the core's tests only.

## One implementation

- **F-1** Every rule is in the Rust crate `polyspec-http-form` (`packages/rust/`): the urlencoded reader, the content
  types, the header lines and parameters of a part, the state machine that reads a body in chunks, the value, part and
  file limits, the directory of a request, the writing, reading, moving and removal of its files, and the body of a
  request as its host reads it (`RequestBody`).
- **F-2** A binding converts types and turns an error of the core into the class of its runtime. It holds no rule: no
  parsing, no limit, no file operation and no message of its own. The bindings are generated from the interface
  contract; the hand-written bindings in `php/`, `python/` and `node/` are not built.
- **F-3** What only a runtime can do stays in the server of that runtime: reading the bytes of a request from its host
  and passing them, with the values of `Content-Type` and `Content-Length`, to `RequestBody`.

## Bytes and text

- **F-4** A content type, a `Content-Length`, a boundary and a body are bytes. A request without `Content-Type` gives
  the core no content type (`None`), which is another value than the empty bytes.
- **F-5** Names, values, filenames and content types that the core returns are UTF-8 text, which the core checked
  (F-20.3, F-21.5, F-22.4). The bytes of a file are bytes, also the bytes of a file that an application makes
  (`UploadedFile::from_value`, F-22.2).

## Errors

Every failure is an `Error` with a kind that never changes (plan F-2). A failure of a request body has the status that
a server answers. The category is the error class of the binding (plan F-3).

| Kind | Status | Category | Failure |
|---|---|---|---|
| `invalid` | 400 | value | F-20.3, F-21.5, F-22.4, F-24.5; a quoted parameter value of a `Content-Disposition` without its end (F-6); a part that is no `form-data` part with a name, or that gives a parameter twice (F-16) |
| `header_limit` | 400 | value | header lines of a part longer than 8192 bytes (F-23.4) |
| `no_boundary` | 400 | value | a `multipart/form-data` content type without a boundary or with an empty one (F-7) |
| `malformed_content_type` | 400 | value | a `Content-Type` that is no media type (F-12) |
| `invalid_content_length` | 400 | value | a `Content-Length` that is not one or more digits (RFC 9110, section 8.6) |
| `body_limit` | 413 | value | the body limit (F-25.1) |
| `value_limit` | 413 | value | the value limit (F-25.2) |
| `part_limit` | 413 | value | the part limit (F-25.3) |
| `file_limit` | 413 | value | the file limit (F-25.4) |
| `not_finished` | none | type | `parts()` before `finish()` (F-9) |
| `not_streamed` | none | type | `parts()` of a body held as one value (F-9) |
| `finished` | none | type | a chunk after `finish()` (F-9) |
| `moved` | none | value | a move or a read of a moved file (F-11), message `the file was moved (F-11)` |
| `exists` | none | value | a move to a path that exists (F-11), message `the path exists (F-11)` |
| `removed` | none | value | a move or a read of a stored file after the removal of its directory, a chunk after it (F-10) |
| `not_found`, `permission_denied`, `storage_full`, `io` | none | io | a failure of the file system, by its `io::ErrorKind`, with its `errno` (plan F-2), the `kinds` of the row of `Io` in the contract; `EXDEV` is `io`, because only a rename gives it and the move then copies (F-11); `io` without `errno` for a size check (F-15); `io` with the `errno` `EEXIST` for a new file of the directory of a request whose path exists (F-10) |
| `source_left` | none | io | a copied file whose source was not removed (F-11) |
| `partial_target` | none | io | a failed copy or write whose incomplete target was not removed (F-11) |
| `internal` | none | internal | an invariant of the core does not hold (H-8) |

The kind `invalid_number` (category value) of a limit that is no unsigned integer, and `invalid_path` of a value that
is no path, are the binding's (plan B-2, B-3).

## Form values and files

- **F-20** `parse_urlencoded(text)` reads `application/x-www-form-urlencoded` text, of a query or of a body, without
  nesting.
  1. F-20.1 The text is split at every `&`, and an empty part is skipped (I-6). The name of a part is the text before
     its first `=` and the value is the rest, or the empty text when the part has no `=`. In both, `+` is a space, `%XX`
     with two hexadecimal digits is its byte, and any other `%` stays.
  2. F-20.2 A name with brackets, such as `roles[]` or `a[b]`, is the name as written, and the empty name is a name.
  3. F-20.3 The result is a list of each name, in the order of its first occurrence, with its values in request order
     (`Fields`); a list keeps a decimal name a text in every language. A name or a value that is not UTF-8 fails with
     `invalid`.
- **F-21** The form values of a body.
  1. F-21.1 They are read from the raw body without nesting, as a list like that of F-20.3.
  2. F-21.2 An `application/x-www-form-urlencoded` body (F-12.2) is read by F-20.
  3. F-21.3 A `multipart/form-data` body gives one value for each part whose `Content-Disposition` has a `name`
     parameter and no `filename` parameter (F-16): its name is that parameter and its value the content of the part as
     it is. A part with a `filename` is a file and no value.
  4. F-21.4 A body of another type, and the body of a request without `Content-Type`, has no form values.
  5. F-21.5 A name or a value that is not UTF-8 fails the body with `invalid`.
- **F-22** The files of a `multipart/form-data` body.
  1. F-22.1 A file part is a part whose `Content-Disposition` has both a `name` and a `filename` parameter (F-16).
  2. F-22.2 A file (`UploadedFile`) has its name, which is the `name` parameter; its filename, which is the `filename`
     parameter as sent, with no decoding; its content type, which is the value of its `Content-Type` line as sent, or
     the empty text without one; and its bytes, which are the content of the part as it is. An application may make a
     file from bytes (`UploadedFile::from_value`), whose size is the number of its bytes.
  3. F-22.3 A file with an empty filename is a file.
  4. F-22.4 The name, the filename and the content type are UTF-8, and a part whose are not fails the body with
     `invalid`.
  5. F-22.5 `filename*` (RFC 5987) is not read (I-4).
  6. F-22.6 The files are a list of each name, in the order of its first file, with its files in request order
     (`Files`).
- **F-23** The header lines of a part.
  1. F-23.1 The header lines of a part are separated by CRLF; the line of a name is a line whose text before its first
     `:` is that name, compared without case, and its value is the rest of the line without the spaces and tabs around
     it. F-16 states which lines a part has.
  2. F-23.2 A `Content-Disposition` is `form-data` when its text before its first `;`, without the spaces and tabs
     around it and compared without case, is `form-data`. Each parameter follows a `;` and spaces or tabs as a name up
     to its `=`, compared without case, and a value that is a quoted string without its quotes (F-6) or the text up to
     the next `;`, space or tab. The text inside a quoted string is no parameter, and another blank, such as a no-break
     space, separates nothing.
  3. F-23.3 A part that fails fails the whole body: its form values and its files both fail.
  4. F-23.4 The header lines of a part, from the end of its delimiter line to its empty line, are at most 8192 bytes
     (`HEADER_LIMIT`); a body with longer ones fails with `header_limit` (400).
- **F-27** `file_over(files, limit)` is true when a file has more bytes than `limit`, and false when every file has at
  most `limit` bytes or there is no file. `limit` is a `uint`, so a binding refuses a negative, fractional or boolean
  value with `invalid_number` before the files are compared (plan B-2).

## Content types and parameters

- **F-6** A quoted parameter value of a `Content-Disposition` is the text between its `"` and the next `"`, with every
  backslash kept, because a browser sends a backslash of a filename as it is and a `"` as `%22`; a value that is not
  quoted is the text as it is, also when it holds a `"`. A quoted value without its next `"` fails with `invalid`. A
  browser sends a `"` of a filename as `%22`, so no client of a form sends such a value; reading it to the end of the
  text gave a value that the client did not send (plan P-3). A quoted value of a content type follows MT-3.3 and MT-3.4
  (F-12).
- **F-7** A `multipart/form-data` content type without a `boundary` parameter, or with an empty one, fails with
  `no_boundary` (400). Such a body cannot be read, and reading it as a body of another type dropped it (plan P-4).
- **F-12** The value of `Content-Type` is read by `polyspec_http_media_type::parse` (MT-4), and by no other reader of
  this library. A value that is no media type, the empty value included, fails with `malformed_content_type` (400); the
  error holds the failure of `parse` as its source, whose kind `malformed_media_type` has no status because its caller
  decides it (MT-8.1). A request without `Content-Type` has no media type (F-4).
  1. F-12.1 `media_type(content type)` is the type, `/` and the subtype in lowercase (MT-1.2), without the parameters,
     and the empty text for a request without `Content-Type`. A server compares it with its form types.
  2. F-12.2 `is_urlencoded(content type)` is true when the type and subtype are `application/x-www-form-urlencoded`, and
     false for another type or none.
  3. F-12.3 `multipart_boundary(content type)` is, when the type and subtype are `multipart/form-data`, the value of its
     `boundary` parameter after MT-3.4, or `no_boundary` (F-7); for another type or none it is `None`. A parameter given
     twice, compared without case, fails the media type (MT-3.5), `boundary` included.
  4. F-12.4 A parameter that its type does not define changes nothing (I-7), because it does not change how the body is
     read: `application/x-www-form-urlencoded` defines no parameter, and its parser decodes UTF-8, "only UTF-8 is
     conforming" (WHATWG URL, section 5, 5.1), so a `charset` changes nothing either and a body that is not UTF-8 fails
     with `invalid` (F-21.5); `multipart/form-data` defines `boundary` alone, its optional parameters being "none" (RFC
     7578, section 8).

## Reading a body

- **F-24** A `multipart/form-data` body read in chunks (`RequestBody` with `stream`, `MultipartReader`).
  1. F-24.1 The reader holds neither the body nor the content of a file part as one value: besides one chunk it holds at
     most the delimiter, the header lines of one part (F-23.4), the text values within the value limit and the names,
     filenames and content types of the parts that it read.
  2. F-24.2 The delimiter is `--` and the boundary (F-12.3).
  3. F-24.3 The first delimiter starts the body or follows a CRLF, and the bytes before it are ignored (I-1); every
     later delimiter follows the CRLF that ends the content of a part, and that CRLF is not content.
  4. F-24.4 After a delimiter comes a CRLF, which starts the header lines of a part, or `--`, which ends the body; the
     bytes after that `--` are ignored (I-2).
  5. F-24.5 A body with other bytes after a delimiter, a body that ends before the `--` that ends it, and a part whose
     header lines hold a line that starts with the delimiter before their empty line, fail with `invalid`.
  6. F-24.6 The result does not depend on the chunks: a body read in any division into chunks gives the values, the
     files and the failure of the body read as one chunk (F-8).
- **F-25** The limits of a body (`BodyLimits`); a passed limit fails the body with its kind and status 413.
  1. F-25.1 The body limit bounds the bytes of the body (`body_limit`); a `Content-Length` above it fails before a byte
     is read (F-9).
  2. F-25.2 The value limit bounds the bytes that the reader holds as one value: a body that is not read in chunks, and
     the names and the contents of the text values of a body read in chunks together (`value_limit`).
  3. F-25.3 The part limit bounds the number of parts of a body read in chunks, every part counted (`part_limit`).
  4. F-25.4 The file limit bounds the bytes of each file of a body read in chunks (`file_limit`). It bounds the
     temporary storage of one file (F-10), not its memory. A file is never truncated and never passed in part: a file
     within the limit is passed whole.
  5. F-25.5 The reader stops at the first failure of the body (F-8, F-9).
  6. F-25.6 A body that a server holds as one value, such as the body of a request that an application or a test
     builds from bytes, is read by `read_held_body(content type, body, limits)` with the limits of F-25: the body limit
     and the value limit bound its bytes, as they bound a body that `RequestBody` holds (at one byte the body limit
     comes first), and the part limit, the file limit and the value limit bound the parts, the files and the text
     values of a `multipart/form-data` body as they bound a body read in chunks. Its files are values. A server reads
     a held body so when `multipart/form-data` is one of its form types (F-25 is a rule of the body, not of the way
     the server received it).

- **F-8** The first failure of a body is the failure whose byte comes first (F-24.6). The byte of a failure is the first
  byte after which the bytes read so far determine it:

  | Failure | Its byte |
  |---|---|
  | `body_limit` | the byte after `limit` bytes |
  | `value_limit` of a body held as one value | the byte after `valueLimit` bytes |
  | `part_limit` | the LF of the CRLF after the delimiter of the part past the limit |
  | `value_limit` of a name | the last byte of the empty line that ends the header lines of the part |
  | `value_limit`, `file_limit` of content | the content byte past the limit |
  | `invalid` of a value that is not UTF-8 | the byte after which the content is no start of UTF-8 text, or the end of the content for a sequence that the end cuts off |
  | `invalid` after a delimiter | the first byte that is neither the `-` nor the CR of `--` and CRLF, or the second byte that does not complete them |
  | `invalid` of header lines (F-6, F-16, F-22.4, F-24.5) | the last byte of the empty line that ends them |
  | `header_limit` | the byte after which no empty line can start within 8192 bytes of the header lines |
  | `invalid` of a body without its end | the end of the body (`finish()`) |

  A content byte that may start a delimiter (`CRLF--` and the boundary) is held until a later byte shows that it does
  not; it is content, and counts for the limits and the UTF-8 of a value, from that later byte. When two failures have
  one byte, a limit (413) comes before a failure of the bytes (400), and `body_limit` before the other limits. A chunk
  that passes the body limit is read up to the limit, and `body_limit` is its failure only when those bytes do not fail.
  The result therefore does not depend on the chunks, also when the body fails.
- **F-9** `RequestBody::new(content type, Content-Length, stream, limits)` starts a body; `limits` is the record
  `BodyLimits` of the body limit, the value limit, the part limit and the file limit. A `Content-Length` that is no
  number fails with `invalid_content_length`, and one above the limit fails with `body_limit` before a byte is read
  (F-25.1); a number of more digits than 64 bits hold is above every limit. A body read in chunks (`stream` and the type
  `multipart/form-data`) needs its boundary (F-6, F-7) and gets its directory. `feed(chunk)` reads the next chunk; its
  first failure (F-8) is the error of that call and of every later call, and the body then holds no bytes and no open
  file (F-25.5). The host calls `finish()` after the last chunk: a body read in chunks that did not end fails with
  `invalid` (F-24.5); a second `finish()` gives the result of the first, and a chunk after it fails with `finished`.
  `parts()` gives the fields and the files of a body read in chunks; it fails with `not_streamed` for a body held as one
  value, with the first failure of the body, and with `not_finished` before `finish()`. A body held as one value gives
  its bytes as `held()`, which the host reads with `read_body`, `fields_from_body` and `files_from_body`.
- **F-16** Every part of a `multipart/form-data` body has one `Content-Disposition` line whose type is `form-data` and
  which has a `name` parameter (RFC 7578, section 4.2), at most one `Content-Type` line, no header line without `:` (RFC
  5322, section 2.2: a header field is a name, a colon and a body), and no parameter of its `Content-Disposition` is
  given twice, its name compared without case (RFC 6266, section 4.1: such a value is invalid). A part without
  `Content-Disposition`, with two `Content-Disposition` or two `Content-Type` lines (no rule says which one counts),
  with a line without `:`, of another disposition type, without `name` or with a parameter given twice fails the body
  with `invalid` at the byte of its header lines (F-8). It is no ignored input: ignoring it dropped a part that the
  client sent (plan P-4). A part that fails so has no name, so it counts for no value limit.
- **F-13** A failure of the sink of a `MultipartReader`, such as a write of a file, is the failure of that call; the
  reader then reads nothing and every later call gives `SinkFailed`. It is not a failure of the body (H-6).

## Files

- **F-10** The directory of a request is created below the temporary directory of the system by `tempfile` with the name
  `http-form-` and random characters, which `tempfile` draws again when the name exists, and with the mode `0700`; the
  content of a file part is written to a file of it as it arrives. Its files have the mode `0600` and are closed on
  `exec`. Each file is created as a new file whose name is the number of the file part in the body. The core itself
  never makes one name twice, but the directory is on a file system that another process of the same user can write, so
  a path that exists is a condition of the environment and no broken invariant of the core: the chunk fails with the
  failure of the file system, `io` with the `errno` `EEXIST`, and the file that exists is not opened, written or removed
  by the failed call. `RequestBody::remove()` removes the directory with the files that were not moved; the directory
  belongs to the request alone, so a directory that is gone fails with `not_found`, and after a removal a second call
  does nothing. After the removal every stored file of the body, through every `UploadedFile` that `parts()` gave, fails
  with `removed`, and so does a chunk. A body dropped without `remove()` removes its directory; a failure of that
  removal reaches no caller, and the binding warns that `remove()` was not called (plan B-10).
- **F-11** `UploadedFile::move_to(path)` moves a file without replacing a file at `path`, by a rename or, to another
  file system, by a copy and a removal; after a move, `move_to` fails with `moved` and `content` fails likewise, and
  `size` is the number of bytes of the file. A stored file is renamed by `renameat2` with `RENAME_NOREPLACE` on Linux
  and by `renameatx_np` with `RENAME_EXCL` on macOS (through `rustix::fs::renameat_with`); `EEXIST` fails with `exists`.
  Only `EXDEV`, a path on another file system, takes another way, because only that errno tells that a rename cannot
  move the file (plan P-3): the copy opens a new file at `path` (`EEXIST` fails with `exists`), copies, writes it to the
  disk (`sync_all`) and removes the source. When the copy, its size or the write to the disk fails, the copy removes the
  file that it created and gives the failure; when that removal fails too the error is `partial_target` with both
  failures. The `errno` of `partial_target` is that of the failed copy, size check or write to the disk, which is the
  cause of the error, and none when the size check failed (F-15); the kind and the `errno` of the failed removal are the
  properties `removalKind` and `removalErrno` of its error class. The `errno` of `source_left` is that of the failed
  removal of the source. When only the removal of the source fails, the file counts as moved and the error is
  `source_left`. Every other errno, `EINVAL` of a file system without the flag and `ENOSYS` included, is the error of
  the move. A file whose bytes are a value is written to a new file and to the disk, and a failed write removes that
  file likewise. Every clone of an `UploadedFile`, and every file that `parts()` gives for one part, is the same file: a
  move through one is a move of all (H-16).
- **F-15** `std` does not report a failure of `close(2)`, so the core checks sizes instead (H-22): the size of a written
  file before it is closed, and the number of bytes that `content()` reads, are the number of bytes that were written;
  another number fails with `io` without `errno`. Every number of bytes is converted with a range check.
- **F-17** A public function that does blocking I/O takes the wait token `binding_core::Io` of the dependency
  `polyspec-binding-core` as its first argument (binding design I-2 (4), plan B-5 (a)): `RequestBody::new` (it creates
  the directory), `feed` (it writes files), `finish` (it closes the file of a failed body), `remove`,
  `UploadedFile::content` and `UploadedFile::move_to`. Only generated code makes a token, through the feature
  `construct`, which the tests of this crate enable as a dev-dependency. No other public function does I/O. `Drop` of
  `RequestBody` removes the directory of a body whose host did not call `remove` (F-10) without a token, because `Drop`
  takes no argument; it is the safety net of a missing cleanup call, which the binding reports (plan B-10).
- **F-28** A body read in chunks and its files have no WASM binding: they keep their files in a directory below the
  temporary directory of the system (F-10) and wait on file I/O (F-17), which WebAssembly in a browser cannot do
  (binding Q-4). `RequestBody` and `UploadedFile` are classes of PHP, Python, Node and Go. The core holds that file
  I/O in its Cargo feature `fs`, which its default features turn on: `RequestBody`, the directory and the files of a
  request, the stored content of an `UploadedFile` and `UploadedFile::move_to`, with the dependencies `rustix` and
  `tempfile`, which do not build for `wasm32-unknown-unknown`. Without `fs` the core keeps the functions of a body
  held as one value and `UploadedFile` with its bytes as a value. The contract builds the WASM binding with the
  features of `core.features.wasm`, which are none, so the generated WASM crate depends on the core with
  `default-features = false` (binding GN-4a); every other target builds the default features.
- **F-14** Moving without replacing needs `renameat2` or `renameatx_np`, so the crate with the feature `fs` builds for
  Linux and macOS only (plan T-2); the check is in the module of that feature, so the core without it builds for
  another target, such as `wasm32-unknown-unknown` (F-28).

## Fault injection points

The core calls `binding_core::fault::point` of the dependency `polyspec-binding-core` at each point of the table below
(`packages/rust/src/fault.rs`, whose constants name the points), and the contract declares the same points in `faults`
(binding G-5). A point does nothing without the feature `fault-test` of this crate, which turns on that of
`polyspec-binding-core`: a binding crate turns it on through its own feature `fault-test`, the tests of this crate
through the dev-dependency, and no release build has it. A case that arms points is a case of one call (G-5): the arming
holds for the call of its member alone.

A failure of the file system (the variants `Io`, `SourceLeft` and `PartialTarget`) comes from the environment, and no
argument of a member reaches it. A point is one call of the system in the core. An injection of the kind `io` makes that
call fail with the given `errno`; the failure takes its kind from that `errno` by the table of Errors, like a failure of
the system, so an `io` injection fills the four kinds of the variant `Io` (`not_found`, `permission_denied`,
`storage_full`, `io`) unless its row states another result. The row of the variant `Io` in `errors.kinds` names the four
in `kinds`, and the `fills` of a point with an `io` injection name the four, because a point fills only the kinds of that
table (binding CK-3). An injection of the kind `short` makes a read, a write or a copy handle only the given number
of bytes and return, which the size check then finds as `io` without `errno` (F-15). An injection of the kind `broken`
at `http_form.internal` treats the lock of a file as poisoned (H-8). An armed point holds for the call of the member
that the case tests, on the thread that runs it; the core starts no thread, so every point takes the arming of its call.

| Id | Place (function) | Inject | Fills | Rule of the condition | Takes the arming |
|---|---|---|---|---|---|
| `http_form.create_directory` | `create_directory` of `RequestBody::new` | `io` | the kinds of `Io` | F-10 | yes |
| `http_form.create_file` | the new file of a file part, `Parts::take` of `feed` | `io` | the kinds of `Io` | F-10 | yes |
| `http_form.write_file` | the write of the content of a file part, `Parts::take` of `feed` | `io`, `short` | the kinds of `Io` | F-10, F-15 | yes |
| `http_form.file_size` | the size of a written file before it is closed, `Parts::take` of `feed` | `io` | the kinds of `Io` | F-15 | yes |
| `http_form.remove_directory` | `remove_dir_all` of `RequestBody::remove` | `io` | the kinds of `Io` | F-10 | yes |
| `http_form.read` | the open and the read of a stored file, `read_stored` of `UploadedFile::content` | `io`, `short` | the kinds of `Io` | F-11, F-15 | yes |
| `http_form.rename` | `Ops::rename` of `move_to` | `io` | the kinds of `Io`, but `EXDEV` takes the copy and fills nothing itself; `EEXIST` gives `exists` | F-11 | yes |
| `http_form.open` | `Ops::open` of the source of a copy | `io` | the kinds of `Io` | F-11 | yes |
| `http_form.create` | `Ops::create` of the target of a copy or of a value | `io` | the kinds of `Io`; `EEXIST` gives `exists` | F-11 | yes |
| `http_form.copy` | `Ops::copy` | `io`, `short` | the kinds of `Io`; `partial_target` with `http_form.remove` | F-11, F-15 | yes |
| `http_form.write` | `Ops::write` of a value | `io` | the kinds of `Io`; `partial_target` with `http_form.remove` | F-11 | yes |
| `http_form.sync` | `Ops::sync` of the target | `io` | the kinds of `Io`; `partial_target` with `http_form.remove` | F-11 | yes |
| `http_form.remove` | `Ops::remove`: of the target after a failed copy, write or sync, and else of the source of a copy | `io` | `partial_target` for the target, `source_left` for the source | F-11 | yes |
| `http_form.internal` | the lock of an `UploadedFile`, of `content` and `move_to`: armed as `broken`, the lock is poisoned | `broken` | `internal` | X-1, H-8 | yes |

`partial_target` needs three injections in one case, because the copy is reached only after `EXDEV` of the rename and
the kind only after the failed removal of the target: `http_form.rename` with `EXDEV`, `http_form.copy` (or
`http_form.sync`) with the failure of the copy, and `http_form.remove` with the failure of the removal. Its `errno` is
that of the copy and its `removalErrno` that of the removal (F-11). `EEXIST` of `http_form.create_file` gives `io`
(F-10), and `EEXIST` of `http_form.rename` and `http_form.create` gives `exists`, which an input reaches as well: a
target path that exists.

`finish` and `parts` do no call of the system that can fail: they give the first failure of the body again (F-9), so
their failures of the file system are those that an earlier `feed` met.

The cases of the contract that arm points are in the groups of one call: `request_body.new` (`create_directory`),
`request_body.feed` (`create_file`, `write_file` with `io` and with `short`, `file_size`), `request_body.remove`
(`remove_directory`), `uploaded_file.move_to` (`create`, `write` and `sync` of a file that is a value) and
`uploaded_file.content` (`internal`, the one case of X-1). A case of one call makes its receiver with the constructor of
its unit from the keys of the case (binding G-5, T-5), so its receiver is a body that was just made or a file that is a
value: no case of one call makes a body that ended, failed or was removed, or a file that is stored or was moved.
`tests/faults.rs` arms the points that only a stored file reaches (`read`, `rename`, `open`, `create`, `copy`, `sync`,
`remove`, and `internal` of a move) in calls of the core. The kinds that only such a receiver reaches therefore have no
case of the contract, and the contract check reports each (CK-22): `finished` and `removed` of `feed`; the failures of a
body that `finish` and `parts` give again (F-9) and `removed` of `finish`; `moved`, `removed` and `io` of `content`;
`moved`, `removed` and `source_left` of `move_to`. `partial_target` of `move_to` has no case either: a file that is a
value reaches it with `http_form.write` and `http_form.remove`, but its target then stays at the path of the case, which
a binding test takes relative to the directory that runs it. The tests of the core reach each of these kinds:
`tests/forms.rs` `bodies` calls `finish` and `parts` after each failure of a chunk, `tests/behavior.rs` gives `finished`
and `removed` of a body, `tests/fields.rs` `streamed` and `tests/forms.rs` `bodies` give `moved` and `removed` of a
file, and `tests/faults.rs` gives `io` of `content`, `source_left` and `partial_target`.

## Ignored inputs (plan P-4)

| Id | Rule | Bytes | Basis | Case |
|---|---|---|---|---|
| I-1 | F-24.3 | the bytes before the first delimiter | RFC 2046, section 5.1.1: the preamble is to be ignored | `fields.json` multipart `the_bytes_before_the_first_delimiter_are_ignored` |
| I-2 | F-24.4 | the bytes after the `--` that ends the body | RFC 2046, section 5.1.1: the epilogue is to be ignored | `fields.json` multipart `the_bytes_after_the_closing_delimiter_are_ignored` |
| I-4 | F-22.5 | the parameter `filename*` of a `Content-Disposition` | RFC 7578, section 4.2: `filename*` must not be used, and the `filename` parameter alone names the file | `forms.json` multipart `a_filename_star_is_ignored` |
| I-5 | F-23.1 | header lines of a name other than `Content-Disposition` and `Content-Type` | RFC 7578, section 4.8: other header fields must be ignored | `fields.json` multipart `a_part_header_of_8192_bytes_is_read`, `forms.json` multipart `other_header_lines_are_ignored` |
| I-6 | F-20.1 | an empty part between two `&` | WHATWG URL, application/x-www-form-urlencoded parsing: an empty sequence is skipped | `fields.json` urlencoded `empty_parts_are_skipped` |
| I-7 | F-12 | the parameters that a form type does not define: every parameter of `application/x-www-form-urlencoded`, and those of `multipart/form-data` other than `boundary` | WHATWG URL, section 5: the type has no parameter and its parser decodes UTF-8; RFC 7578, section 8: optional parameters "none" | `forms.json` multipart `the_charset_of_an_urlencoded_type_is_ignored`, `another_parameter_of_an_urlencoded_type_is_ignored`, `fields.json` multipart `a_boundary_inside_a_quoted_string_is_no_boundary` |

The reader of a media type ignores the inputs of the table of http-media-type (MT-1.3, MT-3.2, MT-3.4, MT-2.2):
whitespace around the value and around `;`, empty parameters, the quotes of a quoted value and the case of names.
I-3, a part that is no `form-data` part with a name, is no row: such a part fails (F-16).

The code marks each place with its id.

## What the server decides

These are the decisions of a server that uses the library, which no rule of this document makes:

- the values of the limits of F-25 and their defaults, and the check that an application declares valid limits;
- the form types of an application, whether a `multipart/form-data` body is read in chunks (`stream` of F-9), the status
  of a body of a type that is no form type, and what follows a `malformed_content_type` (F-12);
- the order of the checks of a request around reading the body, and the response text of a status;
- the API of its request: the raw query of a request target, the methods that return the values of F-20 to F-22 and the
  last value or file of a name, and how a decimal text becomes an integer;
- when the directory of a request is removed (F-10), and how its hosts read the chunks of a body (F-3).

## API

| Core | Purpose |
|---|---|
| `parse_urlencoded(text)` | F-20 |
| `media_type(content type)` | F-12.1 |
| `is_urlencoded(content type)` | F-12.2 |
| `multipart_boundary(content type)` | F-7, F-12.3 |
| `read_body(content type, body)`, `fields_from_body`, `files_from_body` | F-21, F-22: a body held as one value |
| `read_held_body(content type, body, limits)` | F-25.6 |
| `parse_multipart(body, boundary)`, `parse_multipart_chunks(chunks, boundary)`, `Collector`: `take`, `collect`, `into_form` | F-24 |
| `MultipartReader::new(boundary, limits)`: `feed`, `feed_with`, `finish`, `closed`, `buffered`, `parts` | F-24, F-25, F-8, F-13 |
| `RequestBody`: `new`, `feed`, `finish`, `parts`, `held`, `size`, `streamed`, `directory`, `remove`; `BodyLimits` | F-9, F-10, F-17 |
| `Form`: `fields`, `files`; `Fields`, `Files`: lists of a name and its values | F-20.3, F-22.6 |
| `file_over(files, limit)` | F-27 |
| `UploadedFile`: `from_value`, `name`, `filename`, `content_type`, `size`, `content`, `move_to` | F-22.2, F-11, F-17 |
| `Error`: `kind`, `status`, `errno`, `removal`, `removal_kind`, `removal_errno` | the table of Errors, F-11 |
| `fault`: the constants of the fault injection points | Fault injection points |

## Interface contract

- **F-18** The interface contract is `packages/rust/interface.json` (binding design I-1), in the schema of
  `@polyspec/binding`. It declares the members of the table above that a host uses (the functions, `Form`,
  `RequestBody`, `UploadedFile`), the error classes `HttpFormValueError`, `HttpFormTypeError`, `HttpFormIoError`,
  `HttpFormStoreError`, `HttpFormBusyError` and `HttpFormInternalError` of the categories of the table of Errors
  (design E-2), the error class `PartialTargetError` of `partial_target`, and the differences from the earlier bindings
  in `changes`, and the fault injection points of the section "Fault injection points" in `faults`.
  `MultipartReader`, `parse_multipart` and `parse_multipart_chunks` stay Rust API (F-3). `malformed_content_type` is a
  kind of this library, with the status 400 that a request with such a `Content-Type` gets; its error holds the failure
  of http-media-type (F-12), so the contract names no kind of that library and forwards no property of it. `RequestBody`
  implements `binding_core::Cleanup` (its cleanup call is `remove`) and `Error` implements `binding_core::Kinds`.
  1. The core functions take and return the Rust types of the contract types (binding design I-2 (1), T-1, T-2): a
     result is an owned value (`String`, `Vec<u8>`, `PathBuf`, `Vec`), `Fields` and `Files` are the lists
     `list<tuple<text, list<…>>>`, and `Form` and `BodyLimits` are records (design T-4), so `file_over` is a function of
     the files and not a member of `Form`. Every member of the contract is a function of the core, the properties
     `removalKind` and `removalErrno` of `PartialTargetError` included, which fail with `internal` on another kind.

## Verification

- `make test`: the cases of `contracts/fields.json` and `contracts/forms.json` in the core; every multipart case and
  every body case is also read byte by byte and in two chunks divided at every position, and the reader holds between
  two chunks at most its bound (F-24.1, F-24.6). A case that arms fault injection points runs with them, and every
  armed fault must fire; `tests/faults.rs` arms the points of a stored file. Random bodies with random limits give the
  result of the body read byte by byte in every division (`HTTP_FORM_ROUNDS`, 3000 by default). A move to another
  file system needs one at `HTTP_FORM_OTHER_FS` (`/dev/shm` by default). The failures of each step of a move are
  tested with operations that fail on purpose.
- `make clippy`: the lints of plan R-1 and `as_conversions`.
- `make wasm-core`: the core for `wasm32-unknown-unknown` without `fs`, with and without `fault-test`, under clippy
  (F-28); `make check` runs it after `make fmt`, `make clippy` and `make test`.
- `make bindings-check BINDING=<cli>`: the interface contract passes the contract check of `@polyspec/binding` with
  the contract of `polyspec-http-media-type` as its dependency, and the binding packages `packages/ts`,
  `packages/ts/wasm`, `packages/python`, `packages/php-ext` and `packages/go` are what `binding scaffold` writes
  (binding design L-2). `make clippy`,
  `make fmt` and `make test` name the core crate, and `make lint-bindings` and `make test-bindings` check the binding
  crates of PHP, Python and Node after `make bindings`; no make target builds or tests the Go and WASM bindings yet.
- `make differential HYPER=<dir> PYTHONPATH=...`: random bodies through the core and through hyper's Python reader. It
  reads a module of hyper that is not public, so its result is a reference only until hyper exports the reader (plan
  H-25); a difference that F-6 or F-7 explains is counted apart.
