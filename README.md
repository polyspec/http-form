<!-- doc-id: readme -->
# http-form

Form values and `multipart/form-data` bodies, read by one Rust implementation that PHP, Python,
Node, the browser (WASM) and Go bind, the five targets that every contract names; the body read in chunks and its
files are not in the WASM binding (F-28). A binding converts types; every rule is in the core, so the three servers
run the same code and give the same results.

```
packages/rust/   the Rust crate polyspec-http-form: every rule, and interface.json, the interface contract
packages/ts/ packages/ts/wasm/ packages/python/ packages/php-ext/ packages/go/   the thin binding packages that
                 binding scaffold writes
docs/spec/       forms.md: the rules, the errors and the API
contracts/       fields.json and forms.json: the cases of the rules
conformance/     the differential test against hyper's Python reader
legacy/          the hand-written bindings of an earlier API and their runners; not built
```

## Use

```rust
// `io()` is the wait token `binding_core::Io` of one blocking call, which generated code makes (F-17).
let limits = BodyLimits { limit, value_limit, part_limit, file_limit };
let mut body = RequestBody::start(io(), content_type, content_length, true, limits)?;
for chunk in chunks {
    body.feed(io(), chunk)?; // the first failure of the body, with its kind and status
}
body.finish(io())?;
let form = body.parts()?;
// ...
body.remove(io())?;
```

## Checks

```
make test          # the cases in the core, in every division of each body
make fmt           # the format of rustfmt.toml
make bindings BINDING=<binding>/packages/ts/src/cli.ts         # generate the binding sources
make bindings-check BINDING=<binding>/packages/ts/src/cli.ts   # the contract and the binding packages
make clippy        # the lints
make differential HYPER=<hyper>/packages/server/python/src   # random bodies against hyper's Python reader
make check         # clippy and test
```

Requirements: the Rust toolchain of `rust-toolchain.toml`, and the Git dependencies `polyspec-http-media-type` (tag
`v0.0.1`) and `polyspec-binding-core` (tag `v0.0.2`), whose commits `Cargo.lock` pins. Linux and macOS (docs/spec/forms.md,
F-14); Windows is not supported.
