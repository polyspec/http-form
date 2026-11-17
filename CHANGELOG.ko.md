<!-- doc-id: changelog -->
<!-- source-sha256: ef840e25735e067686cef40e326a21ad54a873cb30ae968440de3a912331b087 -->

## Unreleased

- binding 0.0.17과 http-media-type 0.0.8(T1.30): 모든 Cargo manifest의 `polyspec-binding-core`가
  `tag = "v0.0.17"`을 적고, 코어의 `polyspec-http-media-type`이 binding 0.0.17 위의 http-media-type release인
  `tag = "v0.0.8"`을 적는다. `package.json`은 `@polyspec/binding` 0.0.17의 release asset을 적고 `package-lock.json`이
  그 integrity를 고정한다. 그래서 binding 0.0.17을 쓰고 이 코어에 의존하는 저장소가 `polyspec-binding-core` 하나를
  고정한다(CK-20). binding check와 generate는 `--cargo`를 받는다(GN-1d). `Cargo.lock`은 아직 binding 0.0.10과
  http-media-type 0.0.6을 고정한다. 그것을 쓰는 recipe가 없고, Cargo가 그것을 다시 쓸 때까지
  `tests/scripts/binding.test.mts`가 거기서 실패한다.

- `assets.yml`의 job은 binding의 검사 앞에서 go를 설치하고, release하지 않을 때 `bindings-check`를 돈다(T1.29).
  그래서 release가 도는 것을 CI가 release의 각 runner에서 돈다. 전에는 release하지 않는 job이 binding의 검사를 돌지
  않았고, 어떤 job도 go를 설치하지 않았다. binding check는 go로 `packages/go`의 go.sum을 읽는다(binding CK-19).

- kit을 release 0.0.120에 고정하고, `make release-candidate-assets`는 tag 없이 커밋된 HEAD에서 release의 archive를
  만든다. CI의 job check의 항목 core가 그것을 돈다(T1.28). kit이 `CHANGELOG.md`의 첫 절 `## X.Y.Z`에서 version을
  읽으므로(kit K1286) recipe는 version을 적지 않는다. 전에는 release가 tag를 만들어 push한 뒤에야 archive를 처음
  만들었다. `tests/scripts/recipes.test.mts`가 그 target, 그 항목, 그 고정을 요구한다.

- `contracts/fields.json`과 `contracts/forms.json`에서 `then`의 모든 call step이 `args`를 준다(T1.27). 매개변수가 없는
  method인 `request_body.remove`와 `request_body.finish`의 step 열여섯 개가 `"args": {}`를 준다. binding은 모든 call
  step에 그것을 요구한다(binding CK-22). 전에는 이 step들이 `args`를 주지 않았다. `make bindings-check`, `make test`와
  `make test-scripts`가 통과한다.

- tag의 GitHub Release는 archive와 `packages.json` 옆에 `contracts/fields.json`과 `contracts/forms.json`을 가지며,
  각 파일의 URL과 sha256을 경로별로 기록하는 `files.json`도 가진다. 그래서 소비자는 복사본 대신 tag의 case를 version과
  무결성으로 고정한다(T1.25). `config/release.json`의 `files`가 두 파일을 적고, kit 0.0.116의 release 도구가 그것을
  게시한다(kit K1267). 이 저장소는 upload나 무결성 코드를 갖지 않는다. 개발 도구는 kit 0.0.116에 의존한다. 전에는
  release가 archive와 `packages.json`만 게시했다. `make test-scripts`는 20개 시험이 모두 통과하고
  `make release-coverage`가 통과한다.

- `contracts/fields.json`은 입력 열네 개의 case를 더 가지며, 각 case는 고정하는 규칙을 밝힌다(T1.26). `multipart`에서
  `Content-Type`이 없는 body는 값이 없다(F-4, F-21.4). 다른 disposition type의 part, `form-data`로 시작하기만 하는 type의
  part, `name`이 없는 part, header line이 없는 part, header line에 CR만 있는 part, parameter 앞에 no-break space가 있는
  part, parameter를 두 번 준 part는 body를 `invalid`로 실패시킨다(F-16, F-23.1, F-23.2). 끝이 없는 quoted value는
  `invalid`로 실패한다(F-6). 따옴표로 감싼 빈 boundary는 `no_boundary`로 실패한다(F-7). 뒤에 line feed가 붙은 urlencoded
  type은 `malformed_content_type`으로 실패한다(F-12, MT-1.3). `streamed`에서 part limit과 정확히 같은 body는 읽히고, part
  limit 2를 넘는 세 번째 part는 그 파일 전에 읽기를 멈추며, 하나의 값으로 보관된 body는 value limit이 제한하고 part limit과
  file limit은 제한하지 않는다(F-25.2, F-25.3, F-9). boundary가 없는 multipart type은 case
  `a_multipart_type_without_a_boundary_fails`(F-7)를 유지한다. 코어는 이미 모든 기대 결과를 냈고 `make test`가 통과한다.

## 0.0.1


- 코어, 바인딩 crate와 도구는 binding 0.0.10과, binding 0.0.10에 의존하는 http-media-type 0.0.6에 의존하므로
  `Cargo.lock`은 `polyspec-binding-core` 하나를 고정한다(T1.8, CK-20). 다섯 target의 생성 시험은 모든 case를 통과한다.
  Python 341개, Node 342개, WASM 227개, PHP 333개 시험과 Go 패키지 세 개가 통과하고, 파일이 instance 값(GN-13f)인
  `file_over`의 case도 모든 target에서 통과한다. `legacy/`의 이전 API용 손으로 쓴 바인딩을 지웠다(T1.9). 그 runner는
  생성된 바인딩에 없는 API를 불렀고, 생성 시험이 `contracts/fields.json`과 `contracts/forms.json`의 모든 case를 각
  target으로 실행한다.

- 코어는 binding 0.0.9에 의존하는 http-media-type 0.0.5에 의존한다(T1.19-5). 그래서 `Cargo.lock`은
  `polyspec-binding-core`를 0.0.9 하나만 고정하고, `make bindings-check`가 통과하며 `make test-scripts`는 19개가 모두
  통과한다.

- `file_over` case의 각 파일은 instance 값 `{"new": "uploaded_file.new", "args": {...}}`, 즉 그 인자로 부르는
  `UploadedFile` 생성자 호출이다(binding GN-13f). 전에는 그 인자의 객체였고, 생성기는 그것을 instance로 읽지 않는다
  (T1.8). Rust 시험도 같은 형식을 읽는다.

- 코어, 바인딩 crate와 도구는 binding 0.0.9와 kit 0.0.114에 의존한다(T1.8, T1.23). binding 0.0.9에서 다섯 target의
  생성 시험이 실행되고 case 세 개를 뺀 모든 case가 통과한다. Python은 341개 중 338개, Node는 342개 중 339개, WASM은
  227개 중 224개, PHP는 333개 중 330개가 통과하고 Go도 같은 세 개만 실패한다. 세 개는 파일이 있는 `file_over`의
  case이며, 그 인자는 `UploadedFile`의 list이다. binding 0.0.9는 이 class의 case 값을 쓰지 않으므로(G-5) 생성 시험이 그
  이유로 실패한다. `file_over`의 case는 member의 매개변수가 아닌 key `limits`와 `over`를 주었다. 이제 각 case는 한
  호출의 매개변수 `files`, `limit`과 `result`를 주고, Rust 시험은 `UploadedFile::from_value`로 파일을 만든다.
  `make node-package`의 WASM crate는 `panic-test` 옆에 feature `fault-test`로 build된다. 생성된 fault 시험이 부르는
  `__armFaults`를 그 feature만 내보내기 때문이다. binding 0.0.7이 `make install-llvm`을 없앴으므로 PHP 확장 job은
  Linux에서 image의 LLVM을, macOS에서 install-llvm-action의 LLVM을 읽고, `make ci-php`는 그 `LLVM_PATH`를
  `LLVM_ROOT`로 넘기며 `DYLD_LIBRARY_PATH`와 `LD_LIBRARY_PATH`를 비운다. `Cargo.lock`은 http-media-type 0.0.4를 통해
  binding 0.0.9 옆에 0.0.5도 고정하므로 `make bindings-check`와 `make release-binding-check`가 CK-20을 보고하고
  `make test-scripts`는 19개 중 18개가 통과한다.

- 코어와 바인딩 crate는 binding 0.0.7과 http-media-type 0.0.4에 의존한다(T1.19-5, T1.23). binding 0.0.7에서 생성된
  바인딩 crate는 컴파일되고 `make lint-bindings`가 통과한다. 전에는 E0308과 E0425로 실패했다. `Error::removal_kind`는
  종류를 `&'static str`로 돌려준다. 생성 코드가 그 text를 복사하는데 Node crate가 소유한 `String`을 다시 변환해 clippy가
  거부했기 때문이다. `Cargo.lock`은 http-media-type 0.0.4를 통해 0.0.7 옆에 binding 0.0.5를 여전히 고정하므로
  `binding check`는 CK-20을 보고하고 `make test-scripts`는 19개 중 18개를 통과한다. 생성된 시험은 binding 0.0.7의
  결함으로 실패한다(T1.8). Python과 PHP는 생성기가 case의 파일로 읽는 key `files`(GN-13i)를 가진 62개 case가 실패하고,
  Node는 시험을 읽지 못하며, Go는 gofmt 형식이 아니다.

- 코어와 바인딩 crate는 http-media-type 0.0.3과, http-media-type 0.0.3이 의존하는 binding 0.0.5에 의존한다
  (T1.19-5). 모든 Cargo manifest가 tag `v0.0.3`과 `v0.0.5`를 적고, `package.json`이 binding 0.0.5와 kit 0.0.113의
  release asset을 적고, `Cargo.lock`은 `polyspec-binding-core` 하나를 고정한다. 전에는 `Cargo.lock`이 binding 0.0.3
  옆에 0.0.2를 고정해 `binding check`가 CK-20을 보고했고 `Cargo.lock`의 test가 실패했다. `make test-scripts`는
  19개 test를 모두 통과하고 `make bindings-check`도 통과한다.

- `release.yml`의 job도 각각 recipe 하나다(T1.24). `make ci-tag`와 `make ci-release`는 token, CI 실행의 id, actor,
  tag를 job의 `env:`에서 읽는다. `ci-tag`는 release-tag에 `GITHUB_ENV`용 자기 파일을 주고 같은 step에서 그 파일의 TAG와
  VERSION을 읽는다. job `release`는 run step 앞에서 각 플랫폼의 asset을 `var/release/platform-assets`로 내려받고,
  `ci-release`가 draft release를 만든 뒤 그것을 `var/release/assets`에 더하므로 publish, asset, undraft의 순서가 유지된다.

- `ci.yml`과 `assets.yml`의 job은 각각 Makefile의 recipe 하나다(T1.24). job의 run step 하나는 `make ci-check
  NAME=<name>`, `make ci-platform`, `make ci-php EXPECTED_PHP_MINOR=<minor>`, `make ci-npm` 중 하나이고, 호출된 workflow의
  input인 `RELEASE`를 받는다. job `ci-passed`는 `make ci-passed RESULTS='<needs의 JSON>'`을 실행한다. 이전에는 job이 설치,
  step 조건, 대상을 YAML의 step과 expression에 가졌으므로 local 실행은 GitHub runner 없이 job을 반복할 수 없었다. 이제
  recipe가 먼저 설치를 실행하고 그것이 실패하면 멈추며, 다음에 검사를 끝까지 실행하고, 마지막에 요약을 항상 쓴다. job
  `check`의 각 항목의 대상은 Makefile의 `CI_CHECK_TARGETS_<name>`이고, `ci-passed`는 `FAIL_SKIPPED`를 스스로 정한다.
  `tests/scripts/workflows.test.mts`는 run step이 둘 이상인 job, run step의 조건이나 환경 변수, matrix 값이나 input이 아닌
  expression, `ci-<job>`이 아닌 대상을 실패시킨다. 이 test는 변경 전의 workflow에서 실패했고 변경 뒤에 통과한다.

- PHP 확장 asset은 platform/minor별 독립 job을 가지며 setup 하나, expected-minor build 하나, 서로 다른 artifact를
  가진다. `phpExtension.minors`는 8.4와 8.5를 선언하며 그 하한은 PHP 요구와 일치한다. workflow owner는 소유 관계
  mutation을 포함해 test 다섯 개를 통과한다. Red: 이전 설정은 minor를 선언하지 않았고 workflow에는 PHP job이
  없었다. 더 넓은 script 실행은 test 16개를 통과했으며 변경하지 않은 의존성 graph에 binding-core 0.0.2와 0.0.3이
  함께 남아 기존 binding lock assertion 하나가 실패했다(T1.19-1, T1.19-5). native asset과 전체 CI 검증은 binding의
  expected-minor recipe 공개와 의존성 갱신을 기다린다(T1.13, T1.19). 처음 선언한 workflow 명령도 commit된 구현에서
  test 다섯 개를 모두 통과했다. T1.19-6는 이 source 수준의 검증만 기록한다.

- 코어 함수는 생성 코드가 넘기는 대로 인자를 빌려 받는다(binding GN-6, spec F-18.1). `read_held_body`와
  `RequestBody::start`는 `&BodyLimits`를, `UploadedFile::from_value`는 `&str`과 `&[u8]`을 받는다. binding 0.0.2가 생성한
  바인딩 crate는 `BodyLimits`, `String`, `Vec<u8>`를 받는 함수에 `&limits`, `&name`, `&content`를 넘겼으므로, CI의 job
  `bindings`에서 모든 바인딩 crate가 E0308로 컴파일에 실패했다. body의 reader는 자기가 모은 bytes를 복사하지 않고
  가진다(`UploadedFile::from_owned`). 새 시험 `packages/rust/tests/arguments.rs`가 생성 코드의 호출을 한다. Red: 그 시험은
  세 함수에서 E0308로 컴파일되지 않았다. Green: `cargo test --locked --release -p polyspec-http-form`이 시험 44개를
  통과하고, 코어의 `cargo clippy`와 `cargo fmt --check`가 통과한다. Python crate는 binding 0.0.2가 쓰지 않는 변환
  함수도 부른다(E0425). binding 0.0.4가 그것을 쓰므로 바인딩 target은 그 release를 기다린다(T1.23).

- release는 플랫폼별 바인딩 package를 GitHub Release의 asset으로 갖는다. binding 0.0.3이 contract의 release recipe를
  쓴다(binding GN-19h). `make release-recipes`가 `binding release-recipes --root .`을 실행해 무시되는 파일
  `packages/release.mk`를 쓰고, Makefile은 `BINDING_RUN`, `NODE`, `NPM`, `UV`, `BINDING_DEP`를 정의한 뒤 그것을
  include한다. 그래서 이 저장소는 자기 recipe를 두지 않는다. 새 workflow `.github/workflows/assets.yml`은
  `ubuntu-26.04`, `ubuntu-26.04-arm`, `macos-15`에서 각 플랫폼의 Go module 정적 라이브러리, abi3 wheel, PHP 8.4와 8.5용
  PHP 확장, npm 플랫폼 package를 빌드하고, `ubuntu-26.04-arm`에서 WASM module을 가진 npm package를 빌드한다(binding
  GN-19a, GN-19i). `ci.yml`은 모든 실행에서 그것을 호출하고, `release.yml`은 tag에 대해 그것을 호출하며, PHP 확장의
  Composer package로 draft release를 만들고, `make release-go-module`로 Go module을 옮기고, `make
  release-platform-assets`로 asset을 올리고, release를 공개한다. `config/release.json`은 `packages/php-ext`의 Composer
  package를 release하고 `goModules`에 `packages/go`를 더 적지 않는다. `make release-go-module`이 생성 파일과 정적
  라이브러리를 더한 commit에 tag하기 때문이다(binding GN-19f). binding 의존은 release 0.0.3이다(`package.json`,
  `package-lock.json`, 모든 Cargo manifest의 `polyspec-binding-core`의 tag `v0.0.3`, `Cargo.lock`). `@napi-rs/cli`
  3.10.12가 devDependency이고, `config/toolchain.json`이 LLVM 21을 고정하며, `rust-toolchain.toml`이 `llvm-tools`와 Linux
  arch의 musl target을 적고, `mise.toml`과 `mise.lock`이 uv 0.12.24를 적는다. release의 각 job이 tag를 checkout하므로
  target `release-checkout`은 없어졌다. 새 시험 `tests/scripts/workflows.test.mts`가 runner, 고정, run step, release
  step의 순서를 검사한다. 그것이 `tests/scripts/ci.test.mts`의 workflow 시험을 대신하고, `ci.test.mts`는 이제 `ci.yml`의
  event와 마지막 job을 검사하며, 그 release 구성 시험은 빈 `goModules`를 요구한다.
  `Cargo.lock`은 아직 0.0.3 옆에 http-media-type 0.0.1의 binding 0.0.2를 고정한다. 그래서
  http-media-type 0.0.2를 고정하기 전까지(T1.19-5) `binding check --release`는 CK-20을 보고하고
  `tests/scripts/binding.test.mts`의 `Cargo.lock` 시험은 실패한다. Red: include가 없는 Makefile에 `binding check
  --release`가 CK-94를 보고했고, `make test-scripts`는 시험 15개 중 7개에서 실패했다. Green: `binding check --release`는
  CK-20만 보고하고, `make test-scripts`는 시험 15개 중 14개를 통과하며, `make docs-check`와 `make release-coverage`는
  통과한다(T1.19).

- http-media-type은 release된 의존이다. 코어가 http-media-type의 브랜치 `v1/rust-core` 대신 tag `v0.0.1`을 적고,
  `Cargo.lock`이 그 tag의 commit을 고정한다. `binding check --release`는 아무것도 보고하지 않는다(T1.13).
- `make bootstrap`이 다른 모든 install target처럼 그 다운로드를 `$(ONLINE)`으로 실행한다. 다른 release의 npm은 폴더를
  거부하므로(EBADDEVENGINES) 그것은 `package.json`의 `devEngines`가 적는 npm으로 checkout 밖에서 `--prefix`로
  `package-lock.json`을 설치한다. `tests/scripts/ci.test.mts`의 새 시험이 `ONLINE`에 표시를 준 dry run으로 recipe를
  읽는다. 시험은 `$(ONLINE)`이 없는 recipe에서 실패했고 그것이 있으면 통과한다.

- binding이 Go module에서 소유하는 Go 파일 `zz_generated_*.go`는 추적하지 않는다(`.gitignore`). release가 그것을 Go
  tag의 commit에 더하고(binding GN-15g, CK-19), 추적한 사본은 pin이 바뀔 때마다 고정된 binding이 쓰는 것과 달라진다.
  전에는 `AGENTS.md`가 `make bindings`가 그것을 쓰면 추적한다고 했고, binding 0.0.2로 처음 생성한 Go에서 그 파일 24개가
  commit되었다.

- binding은 release된 의존이다. 코어와 바인딩 crate의 `polyspec-binding-core`가 브랜치 `v1/rust-core` 대신 binding
  저장소의 tag `v0.0.2`를 적고, 0.0.2의 binding scaffold가 바인딩 package를 다시 썼으며(Python crate의 license와 build
  script, Node package의 napi target, PHP 확장의 download method), `Cargo.lock`이 그 tag의 커밋과, 바인딩 crate가 같은
  tag를 적는 브랜치 head의 http-media-type을 고정한다. http-media-type 의존은 http-media-type이 tag를 낼 때까지 그 브랜치에
  남는다. 도구는 같은 release의 npm package `@polyspec/binding`이고, `package.json`이 그것을 release asset의 URL로 적고
  `package-lock.json`이 그 integrity로 고정한다. recipe는 그 `dist/cli.js`를 실행하고, binding이 crate를 `cargo metadata
  --locked --offline`으로 읽으므로 `make install`과 CI의 모든 job이 실행하는 새 target `make install-binding`이 `cargo
  fetch --locked`를 실행한다. binding을 `var/binding`에 clone하던 `scripts/binding.mts`는 지웠다. 새 시험
  `tests/scripts/binding.test.mts`는 브랜치 의존에서 네 시험 모두 실패했고 release로 통과한다. binding 0.0.2는 이 계약의
  PHP, Python, Node, WASM, Go를 생성하고, `binding check`와 `binding check --release`는 finding을 내지 않는다.

- record `form`은 계약의 module `body`에, 그것을 주는 `parts`를 가진 `request_body` 옆에 있다. module `forms`에 있으면
  `body`와 `forms`의 Go package가 서로 import하고 Go는 그것을 거부하므로, 더 뒤의 binding의 `binding generate --target go`가
  계약을 거부했다(GN-15a). record가 `body`에 있으면 `forms`는 `body`만 import하고, 더 뒤의 binding이 PHP, Python, Node,
  WASM, Go를 생성한다. record는 Go에서 `body.Form`이고, 다른 target은 그것을 전과 같이 부른다.

- `contracts/fields.json`이 `request_body.remove`의 case `a_second_removal_does_nothing`를 가진다. 그 receiver가 `then`에서
  body를 지우므로 case는 제거를 두 번째로 부르고 그것이 아무것도 하지 않기를 요구한다(F-10). 제거는 receiver가
  `&mut self`인 `request_body`의 cleanup이므로 그 뒤에도 객체가 남고 호출자가 그것을 다시 부를 수 있다. 이후 binding의
  `binding check`는 그런 case가 없는 계약에 CK-76을 보고하고, 그 case가 있으면 finding을 내지 않는다. 코어의 시험은
  `request_body.remove`의 case에 receiver가 있으면 그 receiver에서 body를 가져온다(`packages/rust/tests/fields.rs`).

- `[profile.release]`가 `lto = false`를 정한다. 전에는 `lto = true`를 정했고, 그러면 Go 정적 라이브러리를
  http-media-type의 정적 라이브러리와 함께 링크하는 Go program이 `rust_eh_personality` 같은 중복 symbol로 실패하며(binding
  GN-15i, CK-91), 이후 binding의 `binding check`가 CK-91을 보고한다. Cargo는 `lto`를 profile 전체에 정하므로 release의 PHP
  확장, Node addon, wheel도 LTO 없이 빌드된다. 새 시험 `tests/scripts/profile.test.mts`가 `Cargo.toml`과
  `.cargo/config.toml`의 profile을 읽는다. 변경 전에는 `make test-scripts`가 시험 하나에 실패했고, 변경 후에는 모든 시험이
  통과했다.

- 계약은 `request_body`에 생성자가 없다고 적지 않았다. binding은 `constructible: false`가 없는 class를 생성자가 있는
  것으로 읽으므로, `binding generate`는 static member `start`만 있는 `request_body`를 모든 target에서
  거부했다(binding U-1). unit `request_body`가 `constructible: false`를 적고, F-9가 그 말을 적으며, `tests/start.rs`가
  그것을 요구한다. 더 뒤의 binding의 `binding check`는 계약의 발견을 보고하지 않고, 그 `binding generate`는 그 거부를
  더는 내지 않는다. 그 대신 모든 target에서 `request_body`를 거부한다. cleanup `request_body.remove`의 코어 receiver가
  `&mut self`인데 binding은 receiver가 `self`인 cleanup만 받기 때문이다(binding GN-6p). Go에서는
  `uploaded_file.content`와 `uploaded_file.move_to`도 거부한다(wait `blocking_io`). `make fmt`와 `make docs-check`는
  통과한다. 실행하지 않음: `make test`, `make clippy`. 그래서 바뀐 시험은 아직 compile되지 않았다.

- 추적 파일은 Git commit을 적지 않는다. changelog와 checklist가 이 저장소와 binding의 commit을 적고 있었는데, commit은
  저장소 없이 읽는 사람에게 아무것도 알려 주지 않고 history가 합쳐지거나 다시 쓰이면 아무것도 가리키지 않는다. 각
  기록은 사실을 그대로 두고 binding을 고정된 것 또는 더 뒤의 것으로 적는다. `AGENTS.md`에 규칙이 있고, `make
  test-scripts`가 `tests/scripts/commits.test.mts`를 실행한다. 이 시험은 commit된 tree에서 실패했고 지금 통과한다.

- release는 release commit에 Go module `packages/go`를 `packages/go/vX.Y.Z`로 tag한다. `config/release.json`의
  `goModules`가 그 module을 가리킨다. 전에는 `goModules`가 비어 있었고 규칙은 tag가 Go module을 tag하지 않는다고 했다.
  Go는 저장소 루트 아래의 module을 그 형태의 tag로만 가져오므로, 다른 저장소가 그 module을 version으로 요구할 수
  없었다(binding GN-15g). `make release-coverage`는 모든 package 파일이 분류되었다고 보고한다. `AGENTS.md`는 첫 release
  전에 열려 있는 일을 적는다. `origin`의 `main`, 그리고 kit 0.0.111과 binding의 release를 기다리는 바인딩의 package다.

- recipe의 모든 도구가 추적 파일에 고정되고 `var/tools`에 설치되며, 저장소에 GitHub의 검사와 release가 생겼다. 전에는
  Makefile이 php, php-config, phpunit, maturin, go, gofmt, wasm-bindgen, node, npm을 기계에서 가져오고 binding의 cli를 변수
  `BINDING`에서 받았으므로 결과가 기계에 따라 달랐고, 어떤 workflow도 검사를 실행하거나 commit을 tag하지 않았다.
  `mise.toml`과 `mise.lock`, `config/toolchain.json`, `.php-version`, `composer.json`과 `composer.lock`이 도구를 고정하고,
  `make install`이 그것을 설치하며, `scripts/binding.mts`가 `Cargo.lock`이 고정한 binding을 실행한다.
  `.github/workflows/ci.yml`은 `ubuntu-26.04-arm` 위에서 `make check`, `make bindings-check`, `make test-bindings`, `make
  release-coverage`를 job `core`와 `bindings`로 실행하고, `.github/workflows/release.yml`은 성공한 실행의 commit을
  tag하고 그 GitHub Release를 만든다. `config/release.json`은 코어 crate를 Git tag로 release한다. 코어와 binding
  manifest의 version은 0.1.0이었고 이제 0.0.1이다. release된 version이 없기 때문이다. `make test-scripts`는
  `tests/scripts/ci.test.mts`를 실행하며, 그 시험 4개는 전에 실패했고 지금 통과한다. `make release-coverage`는 모든
  package 파일이 분류되었다고 보고한다. `make bindings-check`는 고정된 binding이 읽지 못하는 case에서 여전히
  실패한다(T1.5).

- 본문은 코어의 static 함수 `RequestBody::start`와 인터페이스 계약의 static member `request_body.start`로 시작하며, 이
  member는 본문을 돌려준다. `RequestBody`에는 생성자가 없다(F-9). 전에는 계약이 생성자 `request_body.new`를
  `wait: blocking_io`로 두었다. 시작이 본문의 디렉터리를 만들기 때문이다(F-10, F-17). 생성자를 await할 수 없는 언어에는
  기다리는 생성자의 형태가 없고 binding은 그런 생성자를 거부하므로(binding GN-6m), 계약의 어떤 바인딩도 생성할 수
  없었다. 호출자는 `RequestBody::new(...)`를 쓰던 자리에 `RequestBody::start(...)`를 쓰고, `contracts/forms.json`의 group
  `request_body.new`와 그것으로 만드는 case의 모든 receiver는 이름이 `request_body.start`이다. 시험 `tests/start.rs`는
  기다리는 member가 생성자나 property가 아니고 `request_body.start`가 본문을 돌려주는 static member임을 요구한다.
  `make test`는 41개 통과, 0개 실패다. binding이 계약에서 생성하는 시험은 실행하지 않았다. binding이 아직 이 계약을
  생성하지 않기 때문이다.

- 더 뒤의 binding의 end-to-end 검사가 `contracts/forms.json`의 group `content_type`의 키 `mediaType`을
  거부했다. 인터페이스 계약이 가리키는 group의 case 키는 snake_case이기 때문이다(G-5). 그 키는 이제
  `media_type`이고, group `body`의 `after_removal`도 같은 규약을 따른다. test `tests/case_keys.rs`는 두 계약 파일의
  모든 case의 모든 키를 검사하며, `make check`는 51 passed, 0 failed를 준다.

- 계약 검사가 지적한 kind 열 개(CK-22)가 더 뒤의 binding의 `then` 호출의 두 꼴(GN-13f)로 case를 가진다.
  `contracts/forms.json`의 `request_body.finish.made`와 `request_body.parts.made`에서 `then`의 호출 `feed`는 자기만을 위해
  `http_form.create_file`을 `ENOENT`, `EACCES`, `ENOSPC`, `EIO`로 장전하고, `finish`와 `parts`는 `not_found`,
  `permission_denied`, `storage_full`, `io`를 errno와 함께 다시 준다(F-9, F-10). `contracts/fields.json`의
  `uploaded_file.content.stored`와 `uploaded_file.move_to.stored`에서 `"on": "receiver"`를 가진 호출
  `request_body.remove`가 꺼낸 파일의 본문을 지우고, `content`와 `move_to`는 `removed`로 실패한다. `tests/common`은
  `then` 호출의 fault를 그 member에 장전하고 `"on": "receiver"`에서 본문을 부르며, `tests/forms.rs`는 case가 적는
  실패의 errno를 비교한다. 더 뒤의 binding의 검사는 CK-22 지적을 내지 않는다. `Cargo.lock`의 고정은 그 commit이 나온 뒤
  따르며, `make check`는 통과 50, 실패 0을 낸다.

- `Cargo.lock`은 binding의 더 뒤 commit을 고정하고, `binding scaffold`가 바인딩 패키지를 다시 쓴다. 바인딩 crate의 allow는
  자기 규칙 id를 이유로 주고, Go crate의 manifest에는 새 줄이 있다. 그 binding의 검사는 같은 CK-22 지적 열 건만 내고,
  `make check`는 통과 50, 실패 0을 낸다.

- 저장소에는 계획도 checklist도 문서 검사도 없었다. `docs/plans/rust-core`가 계획과 checklist를 한국어 쌍과 함께
  담는다. 끝난 단계는 자기 commit을 적고, 계약과 그 workspace는 binding의 두 case 꼴을 기다리며, 바인딩, `legacy/`의
  runner, end-to-end 검사, Go 바인딩은 binding check가 막는 생성을 기다린다. `make docs-check`는 @polyspec/kit의
  check-documents를 `config/documents.json`의 문서와 `config/checklist.json`의 checklist에 실행하고, `make check`가
  그것을 실행한다. @polyspec/kit은 `package.json`이 release의 URL로, `package-lock.json`이 integrity로 고정하며
  `make bootstrap`이 설치한다. `make documents-stamp`는 한국어 쌍의 `source-sha256`을 쓴다.

- 계약이 앞선 호출이 만든 receiver만 닿는 kind에 case를 준다(binding GN-13f). `Cargo.lock`은 그 case 꼴을 받고
  검사하는 binding을 고정한다. case는 그런 receiver를 키 `receiver` 아래에 instance 값으로 준다. 그것은
  `request_body.new`가 만들고 `then`의 호출이 feed, finish, remove하는 본문이며(그 호출 하나는 자기가 이름하는 kind로
  실패할 수 있다), 또는 그런 본문의 `parts`에서 `pick`이 닿는 저장된 파일이다. `contracts/forms.json`의 새 group
  `request_body.feed.made`, `request_body.finish.made`, `request_body.parts.made`는 case 14개를 담는다: `feed`의
  `finished`와 `removed`, `finish`의 `removed`, 그리고 `finish`와 `parts`가 다시 주는 chunk의 `invalid`,
  `header_limit`, `body_limit`, `value_limit`, `part_limit`, `file_limit`(F-9)이다. `contracts/fields.json`의 새 group
  `uploaded_file.content.stored`와 `uploaded_file.move_to.stored`는 case 8개를 담는다: `content`의 `moved`, 지점
  `http_form.read`에 의한 `content`의 `Io` kind 넷, `move_to`의 `moved`, `source_left`, `partial_target`이며, 마지막
  둘은 `EXDEV`의 `http_form.rename`, `http_form.copy`, `http_form.remove` 지점에 의한다. `tests/common`이 코어의 시험을
  위해 instance 값을 만든다. `make bindings-check`는 32건이던 CK-22 finding을 10건으로 낸다: `finish`와 `parts`의
  `Io` kind 넷에는 주입점을 arming하는 `then`의 호출이 필요하고, `content`와 `move_to`의 `removed`에는 파일을 고른 뒤
  본문을 지우는 호출이 필요하다. `make check`는 시험 50개 통과, 0개 실패를 낸다.

- 코어가 결함 주입 지점이 macro `binding_core::fault::point!`인 binding을 받는다. 이 macro는 지점을 쓰는 crate의 feature
  `fault-test`로 지점을 판정한다(binding GN-5): `fault.rs`가 `point!`를 부르고, 코어의 시험은 crate 자신에 대한
  dev-dependency로 `polyspec-http-form`의 `fault-test`를 켜서 지점이 발동하게 한다. 전에는 dev-dependency가 binding의
  것만 켰으므로, macro로 바뀐 뒤 `tests/faults.rs`의 시험 여섯이 장전한 결함이 발동하지 않아 실패했다. `make
  bindings`는 target마다 따로 생성하므로(`make bindings-<target>`, binding GN-1b), 생성기가 거부하는 target은 자기 make
  target만 실패시키고, 바인딩의 make target은 자기 target의 생성만 필요로 한다. `make check`는 시험 50개 통과, 0개
  실패를 낸다.

- 다른 파일 시스템으로의 이동 시험이 macOS에서도 실행된다(H-9). 전에는 `cfg(target_os = "linux")`를 가져서
  라이브러리의 플랫폼인 macOS에서 한 번도 실행되지 않았다. 이제 모든 플랫폼에서 실행된다: 다른 파일 시스템은 주어지면
  `HTTP_FORM_OTHER_FS`, 아니면 Linux에서는 `/dev/shm`, macOS에서는 시험이 자기 디렉터리에 hdiutil로 만들어 붙이고
  끝나거나 실패할 때 떼고 지우는 APFS 디스크 이미지이므로, 그냥 `cargo test`도 그것을 실행한다. `make test`는 시험
  38개 통과, 0개 실패를 낸다.

- 바인딩 패키지와 `Cargo.lock`이 지금 `v1/rust-core`의 binding을 받는다(binding L-2, CK-19, CK-20): `binding
  scaffold`가 추적되는 바인딩 패키지를 다시 쓰고, `Cargo.lock`이 `polyspec-binding-core`의 그 commit을 고정하며,
  `packages/php-ext/build.rs`가 새로 생겼다: macOS에서 확장 하나에만 PHP의 기호를 정의하지 않은 채 두라고 linker에
  알린다. 이제 바인딩 manifest마다 자기 목록 대신 workspace의 lint를 받으므로(`[lints] workspace = true`),
  workspace가 바인딩 crate들이 가졌던 lint를 모두 가진다: rustc의 `warnings`, `unsafe_code`, `unused_must_use`와
  clippy의 `clippy::all`, `allow_attributes_without_reason`, `map_err_ignore`이며, 이제 코어도 이를 지킨다. 코어의
  다섯 자리는 kind가 호출자에게 주는 것을 모두 말하는 오류를 버리고, 자리마다 그 규칙과 함께
  `#[allow(clippy::map_err_ignore, reason = ...)]`를 가진다: `multipart.rs`의 `count`와 `store/files.rs`의
  `read_stored`(H-22, 맞지 않는 수), `part_ended`가 앞서 검사한 값의 UTF-8(H-8), `UploadedFile`의 poison된
  잠금(H-8), `urlencoded.rs`의 `utf8`(F-5). 전에는 지금 binding의 `make bindings-check`가 바인딩 패키지마다 binding
  scaffold가 쓰는 것과 다르다고, `Cargo.lock`의 고정을 다른 commit이라고 보고했다. 이제는 둘 이상의 호출이 만드는
  receiver의 CK-22 32건만 보고한다. `make check`는 시험 40개 통과, 0개 실패를 낸다.

- `UploadedFile`이 메모리의 형태로 WASM의 클래스다(F-28): `new`, `name`, `filename`, `content_type`, `size`, 그리고
  `fs` 없는 코어의 새 `UploadedFile::held_content`를 부르고 아무것도 기다리지 않는 `content`이다. `move_to`와,
  저장된 파일을 읽는 다른 target의 `content`는 WASM이 없는 `only`를 가진다. 전에는 클래스 전체가 WASM이 없는
  `only`를 가졌는데, WASM의 함수인 `files_from_body`와 `file_over`가 그 파일을 주고 받으므로 계약이 WASM에서 맞지
  않았다. `make wasm-core`는 `fs` 없는 코어의 unit 시험도 실행한다. 파일 I/O는 feature `fs` 하나로 두고 따로
  `blocking-io`를 두지 않는다: 코어의 blocking I/O는 모두 파일 I/O이므로, 둘째 feature는 같은 항목을 두 이름으로
  부르게 된다.

- kind `cross_device`와 `IoKind::CrossDevice`를 없앤다: `EXDEV`는 이제 `errno`를 가진 `io`이다(F-2). `EXDEV`는
  rename만 주고 이동은 그때 복사하므로(F-11) 그 kind는 어떤 호출자에게도 닿지 않았다. 계약의 `Error::Io` 행이 kind
  `not_found`, `permission_denied`, `storage_full`, `io`를 적고, `io` 주입을 받는 모든 주입점의 `fills`가 그 넷을
  적으며, case 열둘이 `ENOENT`, `EACCES`, `ENOSPC`로 `request_body.new`, `request_body.feed`,
  `request_body.remove`, `uploaded_file.move_to`에 `not_found`, `permission_denied`, `storage_full`을 준다. 코어의
  시험 하나가 그 행을 코어가 주는 kind와 견준다. `make bindings-check`는 23개였던 CK-22 finding을 32개 준다. 새
  아홉은 `request_body.finish`, `request_body.parts`, `uploaded_file.content`의 그 세 kind이며, 전에 같은 member의
  `io`처럼 호출 둘 이상으로 만든 receiver만 닿는다. 전에는 그 행이 kind를 적지 않아, 계약은 코어가 주는 네 kind
  가운데 `io`만 말했다.

- `polyspec-http-form`이 `wasm32-unknown-unknown`으로 빌드된다(spec F-28). 요청의 파일 I/O, 곧 `RequestBody`, 저장된
  파일과 `UploadedFile`의 이동이 의존성 `rustix`, `tempfile`과 함께 새 Cargo feature `fs`에 있고, 기본 feature가
  그것을 켠다. `BodyLimits`는 본문 module에서 crate 루트로 옮겼고 경로 `polyspec_http_form::BodyLimits`는 그대로다.
  계약이 `core.features.wasm`을 빈 목록으로 적으므로 WASM crate는 코어에 `default-features = false`로 의존하고
  (binding GN-4a), Linux와 macOS에 대한 F-14의 검사는 `fs`의 module에 있다. `rust-toolchain.toml`이 target
  `wasm32-unknown-unknown`을 더하고, `make check`가 실행하는 새 `make wasm-core`가 그 target의 코어를 `fault-test`가
  있을 때와 없을 때 clippy로 검사한다. 전에는 `cargo build -p polyspec-http-form --target wasm32-unknown-unknown`이
  `rustix`가 받는 `errno` 0.3.14에서 "The target OS is "unknown" or "none", so it's unsupported by the errno crate."로
  실패했고 WASM crate는 빌드될 수 없었다. `make check`는 코어 시험 35개로 통과한다.

- `make test-bindings`가 계약의 다섯 target을 이름한다(BD-27): `make lint-bindings`와 `make test-crates`(전에도
  실행하던 바인딩 crate의 clippy와 Rust 시험) 뒤에 `make test-python`, `make test-node`, `make test-wasm`,
  `make test-php`, `make test-go`가 binding이 생성한 시험을 unittest, node --test, PHPUnit, go test로 실행한다.
  `make node-package`는 wasm-bindgen으로 WASM module을, 그리고 Node addon을 빌드해 둘을 npm package로 묶고
  `var/node`에 설치하며, `make test-node`와 `make test-wasm`이 거기서 실행된다. 변수 `PYTHON`, `MATURIN`, `NPM`,
  `PHP`, `PHP_CONFIG`, `PHPUNIT`, `GO`, `GOFMT`, `WASM_BINDGEN`이 도구를 가리킨다. `make test-go`는 go 지시문과 다른
  release의 Go를, `make node-package`는 crate와 다른 release의 wasm-bindgen CLI를 실패시킨다. `var/`,
  `packages/ts/wasm/generated`, `packages/go/internal/ffi/generated`는 무시된다. `AGENTS.md`와 `AGENTS.ko.md`를
  새로 두어 이 규칙을 적는다. 전에는 `make test-bindings`가 PHP, Python, Node crate의 Rust 시험만 실행했고, 어떤
  target도 Go와 WASM 바인딩을 빌드하거나 시험하지 않았다. 그 변경의 binding에서 다섯 target 모두가 `make bindings`에서
  실패한다: `binding generate`가 아래 항목의 CK-22 finding 23개로 계약을 거부한다. 각각은 호출 둘 이상으로 만든
  receiver만 닿는 kind이거나 `move_to`의 `partial_target`이다. 그래서 바인딩이 생성되지 않고 생성 시험도 실행되지
  않는다. `make check`는 통과한다(코어 시험 35개). 언어의 case runner는 아직 `contracts/`의 case를 실행하지 않고,
  `zz_generated_*.go` 파일도 추적하지 않는다. 그것이 실행될, 또는 그것이 나올 생성 바인딩이 없기 때문이다.
  `polyspec-http-form`은 `wasm32-unknown-unknown`으로 컴파일되지 않는다: 의존성 `rustix`가 `errno` 0.3.14를 받고,
  그것이 "The target OS is "unknown" or "none", so it's unsupported by the errno crate."로 실패한다.

- 계약이 실패 주입점 열넷을 `faults`에 선언하고, 호출 하나가 닿는 kind에 case를 준다(CK-22): 주입으로
  `request_body.new`, `feed`, `remove`와 `uploaded_file.move_to`의 `io`, 이미 있는 디렉터리인 경로 `.`로 `move_to`의
  `exists`, 그리고 X-1의 case 하나인 `http_form.internal`로 `uploaded_file.content`의 `internal`이다. PHP, Python,
  Node의 바인딩 crate는 binding scaffold가 `faults`가 있는 계약에 쓰는 feature `fault-test`를 얻는다. 코어의 runner는
  case의 실패를 그 호출에 arming하고, `tests/forms.rs`는 chunk의 실패마다 그 뒤에 `finish`와 `parts`를 부른다.
  `make bindings-check`는 이제 29개 대신 CK-22 finding 23개를 낸다: 하나하나가 호출 둘 이상으로 만든 receiver(끝났거나
  실패했거나 지워진 본문, 저장되었거나 옮겨진 파일)만 닿는 kind로서 호출 하나의 case가 만들 수 없는 것이고(binding G-5,
  T-5), 나머지 하나는 대상이 바인딩 시험의 디렉터리에 대한 상대 경로에 남을 `move_to`의 `partial_target`이다.
  `faults`가 있는 계약에서 PHP, Python, Node가 아닌 target은 이름으로 거부된다. binding이 그 셋에만 실패 진입점을
  생성하기 때문이다.

- 코어가 spec의 실패 주입점을 부른다(X-1, binding G-5): `packages/rust/src/fault.rs`가 주입점 열넷
  `http_form.<id>`를 적고, 디렉터리, part의 파일과 그 쓰기와 크기, 디렉터리의 삭제, 저장된 파일의 읽기, 이동의 각
  단계가 `binding_core::fault::point`를 부른다. 그 함수는 crate의 새 feature `fault-test` 없이는 아무것도 하지 않는다.
  `broken`으로 arming한 주입점 `http_form.internal`은 파일의 lock을 poison된 것으로 만든다(H-8). crate의 시험은
  dev-dependency로 그 feature를 켜며, 그래서 `Cargo.lock`에서 `polyspec-binding-core`의 의존성에 `libc`와
  `serde_json`이 더해진다. `tests/faults.rs`는 저장된 파일의 주입점(read, rename, open, create, copy, sync,
  remove)을 arming하고, arming한 실패가 모두 일어나기를 요구한다. 전에는 spec이 주입점을 적었고 코어는 어떤 주입점도
  부르지 않았다.

- 계약이 binding의 다섯 target을 적는다(BD-27): `packages/rust/interface.json`의 `targets`는 php, python, node,
  go, wasm이고, `packages.go`는 Go 모듈 `github.com/polyspec/http-form/packages/go`를 적으며, npm 패키지의 entry
  `.`는 조건 `{node: napi, browser: wasm}`을 받고, 각 오류 클래스는 WASM 클래스로 Node의 것을 적으며,
  `errors.fixed`는 Node처럼 WASM에도 `TypeError`를 적는다. 전에는 계약이 php, python, node만 적어 Go와 WASM 바인딩이
  생성되지 않았다. 조각으로 읽는 본문과 그 파일에는 WASM 바인딩이 없다(F-28, binding Q-4): unit `request_body`와
  `uploaded_file`은 `only` php, python, node, go이며, 계약 검사는 blocking I/O를 기다리거나 경로를 받는 member에 이것을
  요구한다(CK-15). 추적되는 바인딩 패키지에 binding scaffold가 두 target을 위해 쓰는 것(`packages/go/go.mod`,
  `packages/go/internal/ffi`, `packages/ts/wasm`, `packages/ts/package.json`의 브라우저 exports)이 더해지고, 두
  crate가 workspace의 member가 된다. `make bindings-check`는 이 변경 전부터 있던 CK-22 finding 29개를 그대로 낸다.
  Go와 WASM 바인딩을 빌드하거나 시험하는 make target은 아직 없다.

- `Cargo.lock`은 값 alias, 실패 주입, 오류 unit의 생성기를 한 개발 줄기에 가진 binding을 고정하고(CK-20), 바인딩
  package는 그 scaffold가 쓰는 것이다. 각 바인딩 crate는 `polyspec-binding-core`를 feature `construct`와 `lock`과 함께
  받고, lock에는 `polyspec-binding-core`의 의존으로 `tokio`가 더해진다. `make bindings-check`는 여전히 실패하며,
  40건이던 CK-22 지적이 29건이 되었고, 그 때문에 `make bindings`는 아무것도 쓰지 않는다. 28건은 어떤 case도 내지
  않는 member와 kind를 가리키고, 그 가운데 9건은 실패 주입점만 닿는 파일 시스템의 kind이며, 1건은 주입점
  `http_form.internal`을 `broken`으로 장전하는 case를 요구한다. 고정한 `polyspec-binding-core`에는 실패 주입이 있고
  계약 schema에는 `faults`가 있으므로, spec은 주입점이 그것들을 기다린다고 더는 말하지 않는다. 코어는 어떤 주입점도
  부르지 않고 계약은 어떤 주입점도 선언하지 않는다.

- 추적되는 모든 문서가 한국어 쌍을 가진다. `README.ko.md`, `contracts/README.ko.md`, `docs/spec/forms.ko.md`,
  `legacy/README.ko.md`를 새로 두었다. 한 쌍의 두 파일은 같은 `doc-id` marker를 가지고, 한국어 파일은 영어 파일의
  sha256인 `source-sha256`을 가진다. 이 저장소에는 문서 검사가 없으므로 이 쌍을 검사하는 target은
  없다.

- spec이 실패한 이동의 `errno`가 어느 쪽인지 적는다(F-11). `partial_target`의 `errno`는 실패한 복사·크기 검사·디스크
  쓰기의 것이고 `removalErrno`는 실패한 삭제의 것이다. `source_left`의 `errno`는 원본 삭제의 것이다. 코어의 파일 시스템
  호출에 둘 실패 주입점 표도 적었다. 주입점은 구현되지 않았다. `polyspec-binding-core`의 실패 주입과 계약 schema의
  `faults`를 기다린다.

- 요청 디렉터리의 새 파일 경로가 이미 있으면 그 chunk는 파일 시스템 실패 `io`와 `errno` `EEXIST`로 실패한다(F-10).
  전에는 `internal`로 실패했다. 코어는 한 파일 이름을 두 번 만들지 않지만, 같은 사용자의 다른 프로세스가 그 디렉터리에
  파일을 만들 수 있으므로 이것은 환경의 조건이다.

- `make check`가 macOS에서 통과한다. `tests/behavior.rs`의 도우미 `last`는 Linux에서만 도는 다른 파일 시스템 이동
  시험만 쓰므로 그 시험 안으로 옮겼다. 전에는 macOS에서 clippy가 이 도우미를 dead code로
  실패시켰다.

- `Cargo.lock`은 같은 라이브러리 묶음의 모든 레포가 고정하는 binding commit을 고정하고(CK-20), 그 위의
  `polyspec-http-media-type` 커밋을 고정한다. binding의 scaffold는 같은 바인딩 package를 쓴다.
