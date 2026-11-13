<!-- doc-id: changelog -->

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
