# http-forms: rules and API

This library reads the form values and the `multipart/form-data` bodies of hyper. The rules are hyper's, and this
document names them by hyper's identifiers rather than repeating them: HY-56 (query and urlencoded values), HY-57
(form values), HY-113 (file parts, header lines and parameters), HY-114 (file limit) and HY-131 (reading in chunks,
limits, the files of a request).

`contracts/fields.json` is a copy of hyper's `contracts/fields.json`. Every case passes in the Rust core and in the
PHP, Python and Node bindings, and the three bindings give the same results (`make conformance`).

## One implementation

- **F-1** Every rule is in the Rust crate `http-forms` (`core/`): the urlencoded reader, the content types, the header
  lines and parameters of a part, the state machine that reads a body in chunks, the value, part and file limits, the
  directory of a request, the writing, reading, moving and removal of its files, and the body of a request as its host
  reads it (`RequestBody`).
- **F-2** A binding converts types and turns an error of the core into the class of its runtime. It holds no rule: no
  parsing, no limit, no file operation and no message of its own. A message that a binding gives is a constant or an
  error of the core.
- **F-3** What only a runtime can do stays in the server of that runtime: reading the bytes of a request from its host
  (`php://input`, `node:http`, WSGI, ASGI) and passing them to `RequestBody.feed`.

## Bytes and text

- **F-4** A content type, a boundary and a body are bytes. PHP passes its strings as they are. Python and Node pass a
  header as text whose characters are its bytes, as their hosts give it; a character above U+00FF fails with a
  `TypeError`. A body is `bytes` in Python and a `Buffer` in Node.
- **F-5** Names, values, filenames and content types that the core returns are UTF-8 text, which the core checked
  (HY-42, HY-113.15).

## Errors

| Core | PHP | Python | Node |
|---|---|---|---|
| A move of a moved file, a read after a move (`the file was moved (HY-131)`) | `InvalidArgumentException` | `ValueError` | `RangeError` |
| A move to a path that exists (`the path exists (HY-131)`) | `InvalidArgumentException` | `ValueError` | `RangeError` |
| A path of another type (`a path is not a string (HY-131)`) | `TypeError` | `TypeError` | `TypeError` |
| A body that fails with HY-42, or passes a limit, in `MultipartReader` | `InvalidArgumentException`, code 400 or 413 | `FormError(code, status, message)` | `Error`, message `<code>: ...` |
| A failure of the file system | `RuntimeException` | `OSError` | `Error` |

The codes of a form error are `invalid` (400), `value_limit`, `part_limit` and `file_limit` (413).

## Writing files in Node

- **F-6** `RequestBody.feed` of Node runs on a thread of the libuv pool and returns a promise of the result; the host
  reads the next chunk after the promise settled. A write of a file therefore does not stop the other requests of the
  server, which HY-131 requires. `feedSync` reads on the calling thread. PHP and Python write a chunk before they read
  the next one, as HY-131 states.
- **F-7** This replaces the sentence of HY-131 that the Node server writes a file through a stream of the file. hyper
  changes that sentence when it adopts this library.

## API

Every binding has the same functions and classes, in the names of its runtime (PHP: namespace `Polyspec\Forms`,
methods in camelCase; Python: module `http_forms`, snake_case; Node: camelCase).

| Core | Purpose |
|---|---|
| `parse_urlencoded(text)` | HY-56: a list of pairs `[name, values]`, or null for text that is not UTF-8 |
| `is_urlencoded(content_type)` | HY-57.6 |
| `multipart_boundary(content_type)` | HY-113.11 |
| `fields_from_body(content_type, body)` | HY-57: the values of a body held as one value, or null |
| `files_from_body(content_type, body)` | HY-113: the files of a body held as one value, or null |
| `parse_multipart_chunks(chunks, boundary)` | HY-131.6: a body given in chunks, as `[fields, files]` or null |
| `MultipartReader(boundary, value, parts, file)` | HY-131: events `field`, `file`, `data`, `end` |
| `RequestBody(type, limit, stream, value, parts, file)` | HY-59, HY-131: `feed`, `exceeded`, `size`, `held`, `streamed`, `parts`, `directory`, `remove` |
| `UploadedFile` | HY-113.3, HY-131.15: `name`, `filename`, `contentType`, `size`, `content`, `moveTo` |

## Verification

- `make test`: the cases of `contracts/fields.json` in the core; a multipart case is also read in two chunks divided at
  every position, and the reader holds between two chunks at most its bound (HY-131.6).
- `make conformance`: the same cases through the PHP extension, the Python module and the Node addon; each result is
  compared with the case and with the other bindings.
- `make differential REFERENCE=<dir>`: random bodies, read whole and in random chunks, through the core and through
  hyper's Python reader; every body must give the same values, files and failure.
