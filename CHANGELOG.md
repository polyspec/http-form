<!-- doc-id: changelog -->

## Unreleased

- The recipes read the test features of each binding crate from its manifest (T1.31), and the build tags of `go test`
  come from the same list in the form `polyspec_http_form_<feature>`, under which binding 0.0.17 writes the generated
  Go tests. Before, each recipe named its features: the Go crate was built with `panic-test,codec-test`, without the
  `fault-test` of its manifest, and `go test` took the tags `polyspec_panic_test,polyspec_codec_test`.

- binding 0.0.17 and http-media-type 0.0.8 (T1.30): `polyspec-binding-core` of every Cargo manifest names
  `tag = "v0.0.17"`, `polyspec-http-media-type` of the core names `tag = "v0.0.8"`, the release of http-media-type on
  binding 0.0.17, and `package.json` names the release asset of `@polyspec/binding` 0.0.17, whose integrity
  `package-lock.json` pins, so that a repository on binding 0.0.17 that depends on this core pins one
  `polyspec-binding-core` (CK-20). binding check and generate take `--cargo` (GN-1d). `Cargo.lock` still pins binding
  0.0.10 and http-media-type 0.0.6: no recipe writes it, and `tests/scripts/binding.test.mts` fails on it until Cargo
  writes it again.

- The jobs of `assets.yml` install go before the check of binding and run `bindings-check` when they do not release
  (T1.29), so that CI runs on each runner of the release what the release runs. Before, a job that did not release ran
  no check of binding, and no job installed go, with which binding check reads a go.sum of `packages/go` (binding
  CK-19).

- kit is pinned at its release 0.0.120, and `make release-candidate-assets` builds the archives of the release from the
  committed HEAD with no tag; the entry core of the job check of CI runs it (T1.28). kit reads the version from the
  first section `## X.Y.Z` of `CHANGELOG.md` (kit K1286), so the recipe names none. Before, the archives were first
  built after the release had created and pushed the tag. `tests/scripts/recipes.test.mts` requires the target, the
  entry and the pin.

- Every call step of a `then` in `contracts/fields.json` and `contracts/forms.json` gives its `args` (T1.27): the sixteen
  steps `request_body.remove` and `request_body.finish`, whose methods take no parameters, give `"args": {}`, as binding
  requires of every call step (binding CK-22). Before, these steps gave no `args`. `make bindings-check`, `make test` and
  `make test-scripts` pass.

- The GitHub Release of a tag holds `contracts/fields.json` and `contracts/forms.json` beside the archives and
  `packages.json`, with `files.json`, which records the URL and the sha256 of each by its path, so that a consumer pins
  the cases of a tag by version and integrity in place of a copy (T1.25). `files` of `config/release.json` names the two
  files, and the release tooling of kit 0.0.116 publishes them (kit K1267); this repository holds no upload or integrity
  code. The development tools depend on kit 0.0.116. Before, a release published the archives and `packages.json` alone.
  `make test-scripts` passes 20 of 20 tests and `make release-coverage` passes.

- `contracts/fields.json` holds the cases of fourteen more inputs, each with the rule that it pins (T1.26). In
  `multipart`: a body without `Content-Type` has no values (F-4, F-21.4); a part of another disposition type, of a type
  that only starts with `form-data`, without `name`, without header lines, with a lone CR in a header line, with a
  no-break space before its parameter or with a parameter given twice fails the body with `invalid` (F-16, F-23.1,
  F-23.2); a quoted value without its end fails with `invalid` (F-6); a quoted empty boundary fails with `no_boundary`
  (F-7); an urlencoded type followed by a line feed fails with `malformed_content_type` (F-12, MT-1.3). In `streamed`: a
  body of exactly the part limit is read, a third part past a part limit of two stops the reading before its file, and
  a body held as one value is bounded by the value limit and not by the part or the file limit (F-25.2, F-25.3, F-9). A
  multipart type without a boundary keeps its case `a_multipart_type_without_a_boundary_fails` (F-7). The core already
  gave every expected result, and `make test` passes.

## 0.0.1

- The core, the binding crates and the tool depend on binding 0.0.10 and http-media-type 0.0.6, which depends on
  binding 0.0.10, so `Cargo.lock` pins one `polyspec-binding-core` (T1.8, CK-20). The generated tests of the five
  targets pass every case: Python 341, Node 342, WASM 227 and PHP 333 tests, and the three Go packages; the cases of
  `file_over` with files, whose files are instance values (GN-13f), pass in every target. The hand-written bindings of
  an earlier API in `legacy/` are removed (T1.9): their runners called an API that the generated bindings do not have,
  and the generated tests run every case of `contracts/fields.json` and `contracts/forms.json` through each target.

- The core depends on http-media-type 0.0.5, which depends on binding 0.0.9 (T1.19-5), so `Cargo.lock` pins one
  `polyspec-binding-core`, 0.0.9: `make bindings-check` passes, and `make test-scripts` passes 19 of 19 tests.

- Each file of a case of `file_over` is an instance value, `{"new": "uploaded_file.new", "args": {...}}`, the call of
  the constructor of `UploadedFile` with its arguments (binding GN-13f), in place of the object of those arguments,
  which no generator reads as an instance (T1.8). The Rust test reads the same form.

- The core, the binding crates and the tool depend on binding 0.0.9 and kit 0.0.114 (T1.8, T1.23). With binding 0.0.9
  the generated tests of the five targets run and pass every case but three: Python 338 of 341, Node 339 of 342, WASM
  224 of 227, PHP 330 of 333 and Go fails only the same three. The three are the cases of `file_over` with files,
  whose argument is a list of `UploadedFile`, a class of which binding 0.0.9 writes no case value (G-5), so the
  generated test fails with that reason. The cases of `file_over` gave the keys `limits` and `over`, which are no
  parameters of the member; each case now gives the parameters `files` and `limit` and the `result` of one call, and
  the Rust test builds the files with `UploadedFile::from_value`. The WASM crate of `make node-package` is built with
  the feature `fault-test` beside `panic-test`, because the generated fault tests call `__armFaults`, which only that
  feature exports. The job of the PHP extension reads LLVM from the image on Linux and from install-llvm-action on
  macOS, whose `LLVM_PATH` `make ci-php` passes as `LLVM_ROOT`, with `DYLD_LIBRARY_PATH` and `LD_LIBRARY_PATH` empty,
  since binding 0.0.7 removed `make install-llvm`. `Cargo.lock` still pins binding 0.0.5 beside 0.0.9 through
  http-media-type 0.0.4, so `make bindings-check` and `make release-binding-check` report CK-20 and `make test-scripts`
  passes 18 of 19 tests.

- The core and the binding crates depend on binding 0.0.7 and http-media-type 0.0.4 (T1.19-5, T1.23). With binding
  0.0.7 the generated binding crates compile and `make lint-bindings` passes; before, they failed with E0308 and
  E0425. `Error::removal_kind` returns the kind as `&'static str`, the text that the generated code copies, because
  the Node crate converted the owned `String` again and clippy refused the conversion. `Cargo.lock` still pins binding
  0.0.5 beside 0.0.7 through http-media-type 0.0.4, so `binding check` reports CK-20 and `make test-scripts` passes 18
  of 19 tests. The generated tests fail by defects of binding 0.0.7 (T1.8): Python and PHP fail 62 cases whose key
  `files` the generator reads as the files of a case (GN-13i), Node fails to load its test, and Go is not
  gofmt-formatted.

- The core and the binding crates depend on http-media-type 0.0.3 and on binding 0.0.5, the release that
  http-media-type 0.0.3 depends on (T1.19-5): every Cargo manifest names the tags `v0.0.3` and `v0.0.5`, `package.json`
  names the release asset of binding 0.0.5 and of kit 0.0.113, and `Cargo.lock` pins one `polyspec-binding-core`.
  Before, `Cargo.lock` pinned binding 0.0.2 beside 0.0.3, `binding check` reported CK-20 and the test of `Cargo.lock`
  failed. `make test-scripts` passes 19 of 19 tests and `make bindings-check` passes.

- The jobs of `release.yml` are one recipe each too (T1.24): `make ci-tag` and `make ci-release`, which read the
  token, the id of the CI run, the actor and the tag from the `env:` of the job. `ci-tag` gives release-tag a file of its
  own for `GITHUB_ENV` and reads TAG and VERSION from it in the same step. The job `release` downloads the assets of
  each platform into `var/release/platform-assets` before its run step, and `ci-release` adds them to
  `var/release/assets` after it created the draft release, so the order of publish, assets and undraft stays.

- Every job of `ci.yml` and `assets.yml` is one recipe of the Makefile (T1.24): its one run step is `make ci-check
  NAME=<name>`, `make ci-platform`, `make ci-php EXPECTED_PHP_MINOR=<minor>` or `make ci-npm`, with `RELEASE`, the input
  of the called workflow, and the job `ci-passed` runs `make ci-passed RESULTS='<json of needs>'`. Before, a job held
  its installs, its step conditions and its targets in YAML steps and expressions, so a local run could not repeat it
  without the runner of GitHub. The recipe now runs the installs first and stops at their failure, then the checks to
  their end, then the summary always; the targets of each entry of the job `check` are `CI_CHECK_TARGETS_<name>` of
  the Makefile, and `ci-passed` sets `FAIL_SKIPPED` itself. `tests/scripts/workflows.test.mts` fails a job with more
  than one run step, a condition or an environment on a run step, an expression other than a matrix value or an input,
  and a target other than `ci-<job>`; it failed on the workflows before the change and passes after it.

- PHP extension assets have independent platform/minor jobs with one setup, one expected-minor build and distinct
  artifacts. `phpExtension.minors` declares 8.4 and 8.5, with its lower bound matching the PHP requirement. The
  workflow owner passes five tests including ownership mutations. Red: the earlier configuration named no minors
  and the workflow had no PHP job. The broader script run passed 16 tests and failed the existing binding lock
  assertion because the unchanged dependency graph retains binding-core 0.0.2 beside 0.0.3 (T1.19-1, T1.19-5).
  Native asset and complete CI proof remain pending the published binding expected-minor recipe and dependency
  updates (T1.13, T1.19). The original declared workflow command also passed all five tests on the committed
  implementation; T1.19-6 records only this source-level verification.

- The core functions take their arguments borrowed, as the generated code passes them (binding GN-6, spec F-18.1):
  `read_held_body` and `RequestBody::start` take `&BodyLimits`, and `UploadedFile::from_value` takes `&str` and
  `&[u8]`. The binding crates that binding 0.0.2 generated passed `&limits`, `&name` and `&content` to functions that
  took `BodyLimits`, `String` and `Vec<u8>`, so every binding crate failed to compile with E0308 in the job `bindings`
  of CI. The reader of a body keeps the bytes that it collected without a copy (`UploadedFile::from_owned`). The new test
  `packages/rust/tests/arguments.rs` makes the calls of the generated code. Red: it did not compile, with E0308 for the
  three functions. Green: `cargo test --locked --release -p polyspec-http-form` passes 44 tests, and `cargo clippy` and
  `cargo fmt --check` of the core pass. The Python crate also calls a converter that binding 0.0.2 does not write
  (E0425), which binding 0.0.4 writes, so the binding targets wait for that release (T1.23).

- The release holds the packages of the bindings per platform as assets of its GitHub Release. binding 0.0.3 writes
  the release recipes of the contract (binding GN-19h): `make release-recipes` runs `binding release-recipes --root .`,
  which writes the ignored file `packages/release.mk`, and the Makefile includes it after it defines `BINDING_RUN`,
  `NODE`, `NPM`, `UV` and `BINDING_DEP`, so the repository keeps no recipe of its own. The new workflow
  `.github/workflows/assets.yml` builds on `ubuntu-26.04`, `ubuntu-26.04-arm` and `macos-15` the static library of
  the Go module, the abi3 wheel, the PHP extension for PHP 8.4 and 8.5 and the npm platform package of each platform,
  and on `ubuntu-26.04-arm` the npm package with its WASM module (binding GN-19a, GN-19i). `ci.yml` calls it for every
  run, and `release.yml` calls it for the tag, creates the draft release with the Composer package of the PHP extension,
  carries the Go module with `make release-go-module`, uploads the assets with `make release-platform-assets` and makes
  the release public. `config/release.json` releases the Composer package of `packages/php-ext` and no longer names
  `packages/go` in `goModules`, because `make release-go-module` tags the commit that adds the generated files and the
  static libraries (binding GN-19f). The dependency on binding is the release 0.0.3 (`package.json`,
  `package-lock.json`, the tag `v0.0.3` of `polyspec-binding-core` of every Cargo manifest, and `Cargo.lock`);
  `@napi-rs/cli` 3.10.12 is a devDependency, `config/toolchain.json` pins LLVM 21, `rust-toolchain.toml` names
  `llvm-tools` and the musl targets of the Linux arches, and `mise.toml` with `mise.lock` names uv 0.12.24. The target
  `release-checkout` is removed, since each job of the release checks out the tag. The new test
  `tests/scripts/workflows.test.mts` checks the runners, the pins, the run steps and the order of the release steps, in
  place of the test of the workflows of `tests/scripts/ci.test.mts`, which now checks the events and the last job of
  `ci.yml`, and whose test of the release configuration requires an empty `goModules`.
  `Cargo.lock` still pins the binding 0.0.2 of http-media-type 0.0.1 beside 0.0.3, so `binding check --release` reports
  CK-20 and the test of `Cargo.lock` in `tests/scripts/binding.test.mts` fails until http-media-type 0.0.2 is pinned
  (T1.19-5). Red: `binding check --release` reported CK-94 for the Makefile without the include, and `make
  test-scripts` failed 7 of 15 tests: three of `workflows.test.mts`, three of `binding.test.mts` and the test of the
  release configuration of `ci.test.mts`. Green: `binding check --release` reports CK-20
  alone, `make test-scripts` passes 14 of 15 tests, and `make docs-check` and `make release-coverage` pass (T1.19).

- http-media-type is a released dependency. The core names the tag `v0.0.1` of http-media-type in place of its branch
  `v1/rust-core`, and `Cargo.lock` pins the commit of that tag; `binding check --release` reports nothing (T1.13).
- `make bootstrap` runs its download through `$(ONLINE)`, as every other install target does; it installs
  `package-lock.json` with the npm that `devEngines` of `package.json` names, outside the checkout with `--prefix`,
  because an npm of another release refuses the folder (EBADDEVENGINES). A new test of `tests/scripts/ci.test.mts`
  reads the recipe by a dry run with a mark for `ONLINE`: it failed on the recipe without `$(ONLINE)` and passes with
  it.

- The Go files `zz_generated_*.go` that binding owns in the Go module are not tracked (`.gitignore`): a release adds
  them to the commit of its Go tag (binding GN-15g, CK-19), and a tracked copy differs from what the pinned binding
  writes after every change of the pin. Before, `AGENTS.md` tracked them once `make bindings` wrote them, and the first
  generation of Go with binding 0.0.2 committed 24 of them.

- binding is a released dependency. `polyspec-binding-core` of the core and of the binding crates names the tag
  `v0.0.2` of the binding repository in place of the branch `v1/rust-core`, binding scaffold of 0.0.2 wrote the binding
  packages again (the license and the build script of the Python crate, the napi targets of the Node package, the
  download method of the PHP extension), and `Cargo.lock` pins the commit of the tag and http-media-type at the head of
  its branch, whose binding crates name the same tag; the dependency on http-media-type stays on its branch until
  http-media-type releases a tag. The tool is the npm package `@polyspec/binding` of the same release, which
  `package.json` names by the URL of its release asset and `package-lock.json` pins by its integrity; the recipes run
  its `dist/cli.js`, and the new target `make install-binding`, which `make install` and every job of CI run, runs
  `cargo fetch --locked`, because binding reads the crates with `cargo metadata --locked --offline`.
  `scripts/binding.mts`, which cloned binding into `var/binding`, is removed. The new test
  `tests/scripts/binding.test.mts` failed its four tests on the branch dependency and passes with the release. binding
  0.0.2 generates PHP, Python, Node, WASM and Go for the contract, and `binding check` and `binding check --release`
  give no finding.

- The record `form` is in the module `body` of the contract, beside `request_body`, whose `parts` gives it. In the
  module `forms` it made the Go packages of `body` and `forms` import each other, which Go refuses, so `binding
  generate --target go` of a later binding refused the contract (GN-15a); with the record in `body`, `forms` imports
  `body` alone, and a later binding generates PHP, Python, Node, WASM and Go. The record is
  `body.Form` in Go; the other targets name it as before.

- `contracts/fields.json` holds the case `a_second_removal_does_nothing` of `request_body.remove`, whose receiver removes
  the body in its `then`, so the case calls the removal a second time and requires that it does nothing (F-10). The
  removal is the cleanup of `request_body` with the receiver `&mut self`, so the object stays after it and a caller can
  call it again; `binding check` of a later binding reports CK-76 for a contract without such a case and gives no
  finding with it. The tests of the core take the body of a case of `request_body.remove` from its receiver when it has
  one (`packages/rust/tests/fields.rs`).

- `[profile.release]` sets `lto = false`; before, it set `lto = true`, with which a Go program that links the Go static
  library beside the static library of http-media-type fails with duplicate symbols such as `rust_eh_personality`
  (binding GN-15i, CK-91), and `binding check` of a later binding reports CK-91. Cargo sets `lto` for a whole profile,
  so the PHP extension, the Node addon and the wheel of a release are built without LTO too. The new test
  `tests/scripts/profile.test.mts` reads the profile of `Cargo.toml` and `.cargo/config.toml`: before the change `make
  test-scripts` failed one test, with it every test passed.

- The contract did not state that `request_body` has no constructor. binding reads a class without
  `constructible: false` as one with a constructor, so `binding generate` refused `request_body`, which has the static
  member `start` alone, for every target (binding U-1). The unit `request_body` names `constructible: false`, F-9 names
  that statement, and `tests/start.rs` requires it. `binding check` of a later binding reports no finding of the
  contract, and its `binding generate` no longer gives that refusal; it refuses `request_body` for every target
  because the cleanup `request_body.remove` has the core receiver `&mut self` and binding takes a cleanup with the
  receiver `self` alone (binding GN-6p), and `uploaded_file.content` and `uploaded_file.move_to` for Go (the wait
  `blocking_io`). `make fmt` and `make docs-check` pass. Not run: `make test` and `make clippy`, so the changed test is
  not compiled yet.

- No tracked file names a Git commit. The changelog and the checklist named commits of this repository and of binding,
  which say nothing to a reader without the repository and name nothing after a history is joined or written again.
  Each record keeps its fact and states the binding as the pinned one or a later one. `AGENTS.md` holds the rule, and
  `make test-scripts` runs `tests/scripts/commits.test.mts`, which failed on the committed tree and passes now.

- The release tags the Go module `packages/go` with `packages/go/vX.Y.Z` at the release commit: `goModules` of
  `config/release.json` names the module. Before, `goModules` was empty and the rules said that a tag tags no Go
  module; Go takes a module below the root of a repository only by a tag of that form, so no other repository could
  require the module at a version (binding GN-15g). `make release-coverage` reports every package file classified.
  `AGENTS.md` states what is open before the first release: `main` on `origin`, and the packages of the bindings, which
  wait for kit 0.0.111 and a release of binding.

- Every tool of a recipe is pinned in a tracked file and installed into `var/tools`, and the repository has its checks
  on GitHub and its release. Before, the Makefile took php, php-config, phpunit, maturin, go, gofmt, wasm-bindgen, node
  and npm from the machine and the cli of binding from the variable `BINDING`, so a result depended on the machine,
  and no workflow ran a check or tagged a commit. `mise.toml` with `mise.lock`, `config/toolchain.json`,
  `.php-version` and `composer.json` with `composer.lock` pin the tools, `make install` installs them, and
  `scripts/binding.mts` runs the binding that `Cargo.lock` pins. `.github/workflows/ci.yml` runs `make check`, `make
  bindings-check`, `make test-bindings` and `make release-coverage` in the jobs `core` and `bindings` on
  `ubuntu-26.04-arm`, and `.github/workflows/release.yml` tags the commit of a successful run and creates its GitHub
  Release; `config/release.json` releases the core crate as a Git tag. The version of the core and of the binding
  manifests is 0.0.1, where it was 0.1.0, because no version was released. `make test-scripts` runs
  `tests/scripts/ci.test.mts`, whose 4 tests failed before and pass now; `make release-coverage` reports every package
  file classified. `make bindings-check` still fails on the cases that the pinned binding does not read (T1.5).

- A body is started by the static function `RequestBody::start` of the core and the static member
  `request_body.start` of the interface contract, which returns the body; `RequestBody` has no constructor (F-9).
  Before, the contract named the constructor `request_body.new` with `wait: blocking_io`, because the start creates
  the directory of the body (F-10, F-17). A language whose constructor cannot be awaited has no form of a constructor
  that waits, and binding refuses one (binding GN-6m), so no binding of the contract could be generated. A caller
  writes `RequestBody::start(...)` where it wrote `RequestBody::new(...)`, and the group `request_body.new` of
  `contracts/forms.json` and every receiver of a case made by it are named `request_body.start`. The test
  `tests/start.rs` requires that no member that waits is a constructor or a property and that `request_body.start` is
  a static member that returns the body; `make test` gives 41 passed and 0 failed. The tests that binding generates
  from the contract did not run, because binding does not generate the contract yet.

- The end-to-end check of a later binding refused the key `mediaType` of the group `content_type` of
  `contracts/forms.json`, because a case key of a group that the interface contract names is in snake_case (G-5). The
  key is `media_type` now, and `after_removal` of the group `body` follows the same convention. The test
  `tests/case_keys.rs` checks every key of every case of both contract files, and `make check` gives 51 passed and 0
  failed.

- The ten kinds that the contract check reported (CK-22) have their cases, in the two forms of a call of `then` of
  a later binding (GN-13f). In `request_body.finish.made` and `request_body.parts.made` of `contracts/forms.json`, the
  call `feed` of `then` arms `http_form.create_file` for itself alone with `ENOENT`, `EACCES`, `ENOSPC` and `EIO`, and
  `finish` and `parts` give `not_found`, `permission_denied`, `storage_full` and `io` again with the errno (F-9,
  F-10). In `uploaded_file.content.stored` and `uploaded_file.move_to.stored` of `contracts/fields.json`, the call
  `request_body.remove` with `"on": "receiver"` removes the body of the picked file, and `content` and `move_to` fail
  with `removed`. `tests/common` arms the faults of a call of `then` for its member and calls the body for
  `"on": "receiver"`, and `tests/forms.rs` compares the errno of a failure that a case names. The check of a later
  binding gives no CK-22 finding; the pin of `Cargo.lock` follows once that commit is released, and `make check` gives
  50 passed and 0 failed.

- `Cargo.lock` pins a later commit of binding, and `binding scaffold` writes the binding packages again: the allow of a binding
  crate gives its rule id as its reason, and the Go crate holds a new line of its manifest. The check of that binding
  gives the same ten CK-22 findings and no other, and `make check` gives 50 passed and 0 failed.

- The repository had no plan, no checklist and no check of its documents. `docs/plans/rust-core` holds the plan and
  the checklist with their Korean twins: the done steps name their commits, the contract and its workspace wait for
  two case forms of binding, and the bindings, the runners of `legacy/`, the end-to-end check and the Go binding wait
  for the generation that the binding check holds back. `make docs-check` runs check-documents of @polyspec/kit, which
  `package.json` pins by the URL of its release and `package-lock.json` by its integrity and `make bootstrap`
  installs, on the documents of `config/documents.json` and the checklist of `config/checklist.json`, and `make check`
  runs it. `make documents-stamp` writes the `source-sha256` of a Korean twin.

- The contract gives the kinds that only a receiver made by earlier calls reaches their cases (binding GN-13f), and
  `Cargo.lock` pins the binding whose case forms and check take them. A case gives such a receiver under the key
  `receiver` as an instance value: a body that `request_body.new` makes and the calls of `then` feed, finish or
  remove, one of which may fail with the kind that it names, or a stored file that `pick` reaches in the `parts` of
  such a body. The new groups `request_body.feed.made`, `request_body.finish.made` and `request_body.parts.made` of
  `contracts/forms.json` hold 14 cases: `finished` and `removed` of `feed`, `removed` of `finish`, and `invalid`,
  `header_limit`, `body_limit`, `value_limit`, `part_limit` and `file_limit` of a chunk that `finish` and `parts` give
  again (F-9). The new groups `uploaded_file.content.stored` and `uploaded_file.move_to.stored` of
  `contracts/fields.json` hold 8 cases: `moved` of `content`, the four kinds of `Io` of `content` by the point
  `http_form.read`, and `moved`, `source_left` and `partial_target` of `move_to`, the last two by the points
  `http_form.rename` with `EXDEV`, `http_form.copy` and `http_form.remove`. `tests/common` makes the instance values
  for the tests of the core. `make bindings-check` gives 10 CK-22 findings where it gave 32: the four kinds of `Io` of
  `finish` and of `parts` need a call of `then` that arms points, and `removed` of `content` and `move_to` needs a
  removal of the body after the file is picked. `make check` gives 50 tests passed and 0 failed.

- The core takes binding whose fault injection point is the macro `binding_core::fault::point!`, which judges a point
  by the feature `fault-test` of the crate that writes it (binding GN-5): `fault.rs` calls `point!`, and the tests of
  the core turn on `fault-test` of `polyspec-http-form` through a dev-dependency on the crate itself, so that its
  points fire; before, the dev-dependency turned on only that of binding, and with the macro the six tests of
  `tests/faults.rs` failed because no armed fault fired. `make bindings` generates each target apart
  (`make bindings-<target>`, binding GN-1b), so that a target that the generator refuses fails its own make targets
  alone, and each make target of a binding needs only the generation of its targets. `make check` gives 50 tests
  passed and 0 failed.

- The test of a move to another file system runs on macOS too (H-9). Before, it carried `cfg(target_os = "linux")`,
  so it never ran on macOS, a platform of the library. Now it runs on every platform: the other file system is
  `HTTP_FORM_OTHER_FS` when it is given, else `/dev/shm` on Linux and, on macOS, a disk image of APFS that the test
  makes and attaches with hdiutil in a directory of its own and detaches and removes when it ends or fails, so a
  plain `cargo test` runs it. `make test` gives 38 tests passed and 0 failed.

- The binding packages and `Cargo.lock` take the binding of the current `v1/rust-core` (binding L-2, CK-19, CK-20):
  `binding scaffold` writes the tracked binding packages again, `Cargo.lock` pins that commit of
  `polyspec-binding-core`, and `packages/php-ext/build.rs` is new: on macOS it tells the linker to leave the symbols of
  PHP undefined for the extension alone. Each binding manifest now takes the lints of the workspace
  (`[lints] workspace = true`) instead of a list of its own, so the workspace holds every lint that the binding crates
  held: `warnings`, `unsafe_code` and `unused_must_use` of rustc, and `clippy::all`, `allow_attributes_without_reason`
  and `map_err_ignore` of clippy, which the core now obeys too. Five places of the core drop an error whose kind says
  all that the caller is given, and each carries `#[allow(clippy::map_err_ignore, reason = ...)]` with its rule:
  `count` of `multipart.rs` and `read_stored` of `store/files.rs` (H-22, a number that does not fit), the UTF-8 of a
  value that `part_ended` checked before (H-8), the poisoned lock of `UploadedFile` (H-8) and `utf8` of
  `urlencoded.rs` (F-5). Before, `make bindings-check` of the current binding reported each binding package as
  different from what binding scaffold writes and the pin of `Cargo.lock` as another commit; now it reports only the
  32 CK-22 findings of the receivers that more than one call makes. `make check` gives 40 tests passed and 0 failed.

- `UploadedFile` is a class of WASM in its form in memory (F-28): `new`, `name`, `filename`, `content_type`, `size`
  and `content`, which calls the new `UploadedFile::held_content` of the core without `fs` and waits on nothing.
  `move_to` and the `content` of the other targets, which reads a stored file, carry `only` without WASM. Before, the
  whole class carried `only` without WASM, while `files_from_body` and `file_over`, functions of WASM, give and take
  its files, so the contract did not hold for WASM. `make wasm-core` also runs the unit tests of the core without
  `fs`. The file I/O stays one feature `fs` and has no separate `blocking-io`: every blocking I/O of the core is file
  I/O, so a second feature would name the same items twice.

- The kind `cross_device` is removed, and `IoKind::CrossDevice` with it: `EXDEV` is now `io` with its `errno` (F-2).
  Only a rename gives `EXDEV`, and the move then copies (F-11), so the kind reached no caller. The contract row of
  `Error::Io` names its kinds `not_found`, `permission_denied`, `storage_full` and `io`, the `fills` of every point
  with an `io` injection name the four, and twelve cases give `not_found`, `permission_denied` and `storage_full` to
  `request_body.new`, `request_body.feed`, `request_body.remove` and `uploaded_file.move_to` by `ENOENT`, `EACCES` and
  `ENOSPC`. A test of the core compares the row with the kinds that the core gives. `make bindings-check` gives 32
  CK-22 findings where it gave 23: the nine new ones are those three kinds of `request_body.finish`,
  `request_body.parts` and `uploaded_file.content`, which only a receiver made by more than one call reaches, as
  `io` of the same members before. Before, the row named no kinds, so the contract stated only `io` of the four
  kinds that the core gives.

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
  With the binding of that change every one of the five targets fails at `make bindings`: `binding generate` refuses the
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

- `Cargo.lock` pins binding at the commit that every repository of the library family pins (CK-20), and the
  commit of `polyspec-http-media-type` on it; its scaffold writes the same binding packages.
