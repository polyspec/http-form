<!-- doc-id: changelog -->

## Unreleased

- `polyspec-http-form` builds for `wasm32-unknown-unknown` (spec F-28). The file I/O of a request, `RequestBody`, the
  stored files and the moves of `UploadedFile` with the dependencies `rustix` and `tempfile`, is in the new Cargo
  feature `fs`, which the default features turn on; `BodyLimits` moved from the module of the body to the root of the
  crate, its path `polyspec_http_form::BodyLimits` unchanged. The contract states `core.features.wasm` as an empty
  list, so the WASM crate depends on the core with `default-features = false` (binding GN-4a), and the check of F-14
  for Linux and macOS is in the module of `fs`. `rust-toolchain.toml` adds the target `wasm32-unknown-unknown`, and
  the new `make wasm-core`, which `make check` runs, runs clippy on the core for that target with and without
  `fault-test`. Before, `cargo build -p polyspec-http-form --target wasm32-unknown-unknown` failed in `errno` 0.3.14,
  which `rustix` takes, with "The target OS is "unknown" or "none", so it's unsupported by the errno crate.", and the
  WASM crate could not build. `make check` passes with the 35 tests of the core.

- `make test-bindings` names the five targets of the contract (BD-27): after `make lint-bindings` and `make
  test-crates` (the clippy and the Rust tests of the binding crates, which it ran before), `make test-python`,
  `make test-node`, `make test-wasm`, `make test-php` and `make test-go` run the tests that binding generates on
  unittest, node --test, PHPUnit and go test. `make node-package` builds the WASM module with wasm-bindgen and the Node
  addon, packs both into the npm package and installs it into `var/node`, where `make test-node` and `make test-wasm`
  run. The variables `PYTHON`, `MATURIN`, `NPM`, `PHP`, `PHP_CONFIG`, `PHPUNIT`, `GO`, `GOFMT` and `WASM_BINDGEN` name
  the tools; `make test-go` fails a Go of another release than the go directive and `make node-package` a wasm-bindgen
  CLI of another release than the crate. `var/`, `packages/ts/wasm/generated` and `packages/go/internal/ffi/generated`
  are ignored. `AGENTS.md` and `AGENTS.ko.md` are new and state these rules. Before, `make test-bindings` ran only
  the Rust tests of the PHP, Python and Node crates, and no target built or tested the Go and WASM bindings.
  With binding at 8faad21 every one of the five targets fails at `make bindings`: `binding generate` refuses the
  contract with the 23 CK-22 findings of the entry below, each a kind that only a receiver made by more than one call
  reaches, or `partial_target` of `move_to`, so no binding is generated and no generated test runs. `make check`
  passes (35 tests of the core). No case runner of a language runs the cases of `contracts/` yet, and the files
  `zz_generated_*.go` are not tracked, because no binding is generated that they could run against or come from.
  `polyspec-http-form` does not compile for `wasm32-unknown-unknown`: its dependency `rustix` takes `errno` 0.3.14,
  which fails with "The target OS is "unknown" or "none", so it's unsupported by the errno crate."

- The contract declares the fourteen fault injection points in `faults` and gives cases to the kinds that one call
  reaches (CK-22): `io` of `request_body.new`, `feed` and `remove` and of `uploaded_file.move_to` by an injection,
  `exists` of `move_to` by the path `.`, a directory that exists, and the one case of X-1, `internal` of
  `uploaded_file.content` by `http_form.internal`. The binding crates of PHP, Python and Node gain the feature
  `fault-test` that binding scaffold writes for a contract with `faults`. The runners of the core arm the faults of a
  case for its call, and `tests/forms.rs` calls `finish` and `parts` after each failure of a chunk. `make bindings-check`
  now gives 23 CK-22 findings where it gave 29: each is a kind that only a receiver made by more than one call reaches
  (a body that ended, failed or was removed, a file that is stored or was moved), which a case of one call cannot make
  (binding G-5, T-5), and `partial_target` of `move_to`, whose target would stay at a path relative to the directory
  of the binding test. A target other than PHP, Python and Node is refused by name for a contract with `faults`, because
  binding generates the fault entry points for those three only.

- The core calls the fault injection points of its spec (X-1, binding G-5): `packages/rust/src/fault.rs` names the
  fourteen points `http_form.<id>`, and the directory, the file of a part, its write and its size, the removal of the
  directory, the read of a stored file and each step of a move call `binding_core::fault::point`, which does nothing
  without the new feature `fault-test` of the crate; the point `http_form.internal`, armed as `broken`, makes the lock
  of a file a poisoned one (H-8). The tests of the crate turn the feature on through the dev-dependency, which adds
  `libc` and `serde_json` to the dependencies of `polyspec-binding-core` in `Cargo.lock`, and `tests/faults.rs` arms the
  points of a stored file (read, rename, open, create, copy, sync, remove) and requires each armed fault to fire. Before,
  the spec named the points and the core called none.

- The contract names the five targets of binding (BD-27): `targets` of `packages/rust/interface.json` is php, python,
  node, go and wasm, `packages.go` names the Go module `github.com/polyspec/http-form/packages/go`, the entry `.` of
  the npm package takes the conditions `{node: napi, browser: wasm}`, each error class names its WASM class, the Node
  one, and `errors.fixed` names `TypeError` for WASM as for Node. Before, the contract named php, python and node, so
  no Go or WASM binding was generated. A body read in chunks and its files have no WASM binding (F-28, binding Q-4):
  the units `request_body` and `uploaded_file` are `only` php, python, node and go, which the contract check requires
  of a member that waits on blocking I/O or takes a path (CK-15). The tracked binding packages gain what binding
  scaffold writes for the two targets (`packages/go/go.mod`, `packages/go/internal/ffi`, `packages/ts/wasm`, the browser
  exports of `packages/ts/package.json`), and both crates are members of the workspace. `make bindings-check` keeps its
  29 CK-22 findings from before this change. No make target builds or tests the Go and WASM bindings yet.

- `Cargo.lock` pins the binding whose generator holds the value aliases, the fault injection and the error units in one
  line of development (CK-20), and the binding packages are those that its scaffold writes: each binding crate takes
  `polyspec-binding-core` with the features `construct` and `lock`, and the lock gains `tokio` as a dependency of
  `polyspec-binding-core`. `make bindings-check` still fails, now with 29 findings of CK-22 where it had 40, and
  `make bindings` writes nothing because of them: 28 name a member and a kind that no case raises, 9 of them a kind
  of the file system that only a fault injection point reaches, and one requires a case that arms the point
  `http_form.internal` as `broken`. The pinned `polyspec-binding-core` has fault injection and the contract schema has
  `faults`, so the spec no longer says that the points wait for them: the core calls no point and the contract
  declares none.

- Every tracked document has its Korean twin: `README.ko.md`, `contracts/README.ko.md`, `docs/spec/forms.ko.md` and
  `legacy/README.ko.md` are new. The two files of a pair hold the same `doc-id` marker, and the Korean file holds
  `source-sha256`, the sha256 of its English file. No target checks the pairs, because this repository has no document
  check.

- The spec states which `errno` a failed move has (F-11): `partial_target` has that of the failed copy, size check or
  write to the disk, and `removalErrno` that of the failed removal; `source_left` has that of the removal of the source.
  It also lists the fault injection points of the file system calls of the core, which are not implemented: they wait
  for the fault injection of `polyspec-binding-core` and for `faults` of the contract schema.

- A new file of the directory of a request whose path exists fails the chunk with the failure of the file system, `io`
  with the `errno` `EEXIST` (F-10). Before, it failed with `internal`. The core never makes one file name twice, but
  another process of the same user can create a file in the directory, so the condition is one of the environment.

- `make check` passes on macOS: the helper `last` of `tests/behavior.rs` is used only by the test of a move to another
  file system, which runs on Linux, and it is now inside that test. Before, clippy failed the helper as dead code on
  macOS.

- `Cargo.lock` pins binding at 9de9d03, the commit that every repository of the library family pins (CK-20), and the
  commit of `polyspec-http-media-type` on it; its scaffold writes the same binding packages.
