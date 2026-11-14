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
- binding is the generator of the commit of `polyspec-binding-core` that `Cargo.lock` pins: `scripts/binding.mts`
  clones the checkout that Cargo made of that commit into `var/binding/<commit>` and installs its dependencies there by
  its `package-lock.json`, so that the generator and `binding_core` are one version. The variables `PYTHON` (3.14 or
  later), `PHP`, `PHP_CONFIG`, `PHPUNIT`, `GO`, `GOFMT` and `WASM_BINDGEN` name the tools of `var/tools` and `vendor/`
  (see Tools), and `make test-python` installs the maturin of `MATURIN` into the virtual environment of the test. `make test-go` fails a Go of
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
- `[profile.release]` of `Cargo.toml` sets `lto = false` (binding GN-15i, CK-91). The Go static library is built with
  the release profile, and LTO would put the standard library and `binding_core` into its one object, so a Go program
  that links it beside the static library of http-media-type would fail with duplicate symbols such as
  `rust_eh_personality`. Cargo sets `lto` for a whole profile and not for one package, so the profile itself keeps LTO
  off, and the PHP extension, the Node addon and the wheel of a release are built without it too.
  `tests/scripts/profile.test.mts` requires the profile.

## Tools

- Every tool of a recipe is pinned in a tracked file and installed into `var/tools` of the checkout, never taken from
  the machine, so that a result does not depend on what a machine has: `mise.toml` names Node.js, npm, Go, Python,
  Composer and the wasm-bindgen CLI at the releases of binding and `mise.lock` their digests, `config/toolchain.json`
  the release of mise and its digests, `rust-toolchain.toml` the Rust toolchain with the target
  `wasm32-unknown-unknown`, `.php-version` the PHP, which the wrappers of `var/tools/bin` run, and `composer.json` with
  `composer.lock` PHPUnit, which `make install-php-tools` installs into `vendor/`. A fresh checkout runs `make
  bootstrap`, which installs `package-lock.json` with the npm of `devEngines` of `package.json`, then `make install`.
  Every recipe runs with `var/tools/bin` and the shims of mise first on `PATH`.
- `make test-scripts` runs `tests/scripts/ci.test.mts`, which fails a tool that no tracked file pins by its exact
  release, a tool variable of the Makefile that names a tool of the machine, and a workflow step that is not one make
  target of the Makefile, an action that no commit pins, a job on another runner than `ubuntu-26.04-arm` and a time
  limit.

## CI and release

- `.github/workflows/ci.yml` runs on the push of `main` and on a manual run, one job per matrix entry: `core` runs
  `fmt`, `clippy`, `test`, `wasm-core`, `bindings-check`, `docs-check`, `test-scripts` and `release-coverage`, and
  `bindings` runs the targets of `make test-bindings`. Each job runs them with `make ci-targets`, which runs every
  target to its end and writes the report `var/ci/<name>/`, and the last job `ci-passed` passes only when every other
  job passed. `.github/actions/bootstrap` is the bootstrap of every job.
- `config/release.json` classifies every package file, which `make release-coverage` checks. A tag `vX.Y.Z` releases the
  core crate `packages/rust` as a Git tag, which a Cargo dependency names with `tag = "vX.Y.Z"`, and it is the version
  of the binding manifests, which binding scaffold writes from the version of the core.
- A release is the commit of a successful push or manual run of `ci.yml` on `main`. The version-bump commit
  `chore(release): Release X.Y.Z` sets the version of the core (`version` of `[workspace.package]` of the root
  `Cargo.toml`), writes the binding manifests again with binding scaffold (`node scripts/binding.mts scaffold --contract
  packages/rust/interface.json`), updates the versions of the local packages in `Cargo.lock` and renames `## Unreleased`
  of both changelogs to `## X.Y.Z`, with a new empty `## Unreleased` above it. When the CI run of that push succeeds,
  `.github/workflows/release.yml` starts by itself, and `gh workflow run release.yml -f run=<id>` starts it again: `make
  release-tag` creates and pushes the annotated tag `vX.Y.Z` at that commit, and `make release-publish` and `make
  release-undraft` create the GitHub Release. No tag is created by hand.
- Open before the first release, which is not complete without them: `origin` has no `main`, so the release pushes
  as `main` the merge of `v1/rust-core` with the history of the earlier API and makes `main` the default branch; and the packages of the bindings (the abi3 wheel, the
  PHP extension, the npm package with its platform packages) are rows of `config/release.json` only when kit 0.0.111
  and a release of binding exist, because kit 0.0.109 refuses their kinds; binding generates the release recipes of
  these packages (binding B2.262), so this repository writes no recipe of its own (T1.19). The release also tags the Go module `packages/go` with `packages/go/vX.Y.Z` at the release commit (`goModules` of
  `config/release.json`), because Go takes a module below the root of a repository by the tag of that form (binding
  GN-15g). The first release also waits for the bindings (T1.13).

## Commits and changes

- No tracked file names a Git commit, of this repository or of another: a record states what changed and what a
  check gave, and names a dependency by what `Cargo.lock` pins ("the pinned binding") or by its release. A commit id
  says nothing to a reader without the repository, and it names nothing after a history is joined with another or
  written again, as the move of `v1/rust-core` onto `main` does. The files that pin by a commit are the exceptions:
  the lock files and the actions of the workflows. `make test-scripts` runs `tests/scripts/commits.test.mts`, which
  fails a tracked file that holds a commit id.
