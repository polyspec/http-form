# http-forms

The form values and the `multipart/form-data` bodies of hyper, read by one Rust implementation that PHP, Python and
Node bind. A binding converts types; every rule is in the core, so the three servers run the same code and give the
same results.

```
packages/rust/   the Rust crate polyspec-http-forms: every rule, and interface.json, the interface contract
docs/spec/       forms.md: the rules, the errors and the API
contracts/       fields.json (a copy of hyper's, see sources.json) and forms.json (the cases of this library)
conformance/     the differential test against hyper's Python reader
php/ python/ node/  hand-written bindings of an earlier API; not built, replaced by generated bindings
```

## Use

```rust
// `io()` is the wait token `bindgen_core::Io` of one blocking call, which generated code makes (F-17).
let mut body = RequestBody::new(io(), content_type, content_length, limit, true, value_limit, part_limit, file_limit)?;
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
make clippy        # the lints
make differential HYPER=<hyper>/packages/server/python/src   # random bodies against hyper's Python reader
make check         # clippy and test
```

Requirements: the Rust toolchain of `rust-toolchain.toml`, and the path dependencies `polyspec-http-media-type` and
`polyspec-bindgen-core` beside this repository (`../http-media-type`, `../bindgen`). Linux and macOS (docs/spec/forms.md, F-14); Windows is
not supported.
