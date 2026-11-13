<!-- doc-id: agents -->
# Development

The working principles of every polyspec repository are in `PRINCIPLES.md` of polyspec; this file states only the
rules of this repository.

## Bindings

- The repository is one Cargo workspace: the core `packages/rust` and the binding crates of the five targets of the
  contract (binding BD-27) `packages/python`, `packages/ts`, `packages/ts/wasm`, `packages/php-ext` and
  `packages/go/internal/ffi` (with `packages/go/go.mod`), which `binding scaffold` writes and `make bindings-check`
  compares. `Cargo.lock` at the root pins every dependency, the commit of `polyspec-binding-core` included. Every
  crate takes the lints of the workspace (`[lints] workspace = true`), which binding scaffold writes into each binding
  manifest, so the lints of the root `Cargo.toml` hold for the core and the binding crates alike, and a use that a
  rule needs carries `#[allow(..., reason = "<rule id>")]`.
- The bindings are generated from the interface contract `packages/rust/interface.json` and are not written by hand.
  The hand-written bindings of an earlier API are in `legacy/`, which no target builds; they stay until the generated
  bindings pass their tests.
- `make bindings` generates, each target of the contract apart (`make bindings-<target>`, binding GN-1b), so that a
  target that the generator refuses fails its own make targets alone; every make target that builds or tests a binding
  runs the generation of its targets first. `make test-bindings` runs
  `make lint-bindings` (clippy on the binding crates), `make test-crates` (the Rust tests of the binding crates), then
  `make test-python`, `make test-node`, `make test-wasm`, `make test-php` and `make test-go`, one for each of the five
  targets, each of which runs the tests that binding generates on the standard runner of its language: unittest,
  node --test (Node, and WASM through the subpath `./wasm` of the same installed package, which `make node-package`
  packs and installs), PHPUnit and go test. A target that the generator does not take stays a failing target with its
  cause; no target is left out or skipped (binding BD-27).
- The variables `BINDING` (the cli of binding), `PYTHON` (3.14 or later), `MATURIN`, `PHP`, `PHP_CONFIG`, `PHPUNIT`,
  `NODE`, `NPM`, `GO`, `GOFMT` and `WASM_BINDGEN` name the tools; a recipe installs none. `make test-go` fails a Go of
  another release than the go directive of `packages/go/go.mod` and runs it with `GOTOOLCHAIN=local`, so that it
  fetches no toolchain, and `make node-package` fails a wasm-bindgen CLI of another release than the `wasm-bindgen`
  crate of `packages/ts/wasm/Cargo.toml`, because the CLI reads only a module of its own release (binding GN-18).
  `CARGOFLAGS` adds arguments to every cargo build of a binding.
- The generated folders `generated/` of the binding packages, `packages/ts/wasm/generated` and
  `packages/go/internal/ffi/generated` are not tracked, nor is `var/`, which holds the virtual environment, the wheel
  and the installed npm packages of the tests. In the Go module binding owns the files `zz_generated_*.go` (binding
  GN-3); they are tracked once `make bindings` writes them, because a Go module is fetched as source.
- On macOS a PHP extension and a Node addon take the symbols of their runtime when it loads them, so
  `.cargo/config.toml` tells the linker to leave them undefined, and the recipes name the library that Cargo builds by
  the extension of the platform (`dylib` on macOS, `so` on Linux).
- The core builds for `wasm32-unknown-unknown` without its feature `fs`, which holds the files of a request and
  their dependencies `rustix` and `tempfile` (spec F-28). The contract states the features of the WASM build in
  `core.features.wasm`, an empty list, and the WASM crate depends on the core with `default-features = false` (binding
  GN-4a); no manifest splits a dependency by `[target.'cfg(...)'.dependencies]`, because `core.features` is the one
  place that states what a target builds. An item that needs `fs` carries `#[cfg(feature = "fs")]` and stays out of
  the WASM binding through the `only` of its unit or member; `UploadedFile` is in WASM in its form in memory. `rust-toolchain.toml` installs the target, and `make wasm-core`,
  which `make check` runs, builds the core for it under clippy.
- The contract cases of `contracts/` do not run through a binding yet: ten cases take two forms of a call of `then`
  (binding GN-13f), its own `faults` and `"on": "receiver"`, which the binding that `Cargo.lock` pins does not read, so
  its check refuses them and `make bindings` writes no binding; no case runner of a language exists before a
  generated binding that it can run against. A case gives a receiver that earlier calls make as an instance value
  (binding GN-13f), which `tests/common` makes for the tests of the core, the faults and the receiver of a call of
  `then` included.

## Checks

- `make check` runs rustfmt, clippy, the tests, `make wasm-core` and `make docs-check`.
- `make docs-check` reads each document that `config/documents.json` selects, with its links, anchors and fences and
  the `doc-id` and `source-sha256` of each Korean twin, and the checklist `docs/plans/rust-core/checklist.md`, whose
  rows `config/checklist.json` declares; `make bootstrap` installs the @polyspec/kit that it runs. The plan of the
  repository is `docs/plans/rust-core/plan.md`; a task is a row of that checklist, marked `[o]` only after its
  verification command passed on the committed tree. Run it before a commit. After a Korean twin is translated again,
  `make documents-stamp PATHS=<twin>` writes its `source-sha256`; no marker is written by hand.
