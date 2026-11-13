<!-- doc-id: agents -->
# Development

The working principles of every polyspec repository are in `PRINCIPLES.md` of polyspec; this file states only the
rules of this repository.

## Bindings

- The repository is one Cargo workspace: the core `packages/rust` and the binding crates of the five targets of the
  contract (binding BD-27) `packages/python`, `packages/ts`, `packages/ts/wasm`, `packages/php-ext` and
  `packages/go/internal/ffi` (with `packages/go/go.mod`), which `binding scaffold` writes and `make bindings-check`
  compares. `Cargo.lock` at the root pins every dependency, the commit of `polyspec-binding-core` included.
- The bindings are generated from the interface contract `packages/rust/interface.json` and are not written by hand.
  The hand-written bindings of an earlier API are in `legacy/`, which no target builds; they stay until the generated
  bindings pass their tests.
- `make bindings` generates; every make target that builds or tests a binding runs it first. `make test-bindings` runs
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
- The contract cases of `contracts/` do not run through a binding yet: `binding check` refuses the contract with the
  CK-22 findings that `CHANGELOG.md` lists, so `make bindings` writes no binding, and no case runner of a language
  exists before a generated binding that it can run against.
