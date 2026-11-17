<!-- doc-id: agents -->
<!-- source-sha256: e0c07c3d0e40239c93af7af89c60d114dfdec6d1440e20386b2f9cbe16111ef6 -->
# 개발

모든 polyspec 저장소의 작업 원칙은 polyspec의 `PRINCIPLES.md`에 있다. 이 파일은 이 저장소의 규칙만 적는다.

## 바인딩

- 저장소는 Cargo workspace 하나다: 코어 `packages/rust`와, 계약의 다섯 target(binding BD-27)의 바인딩 crate
  `packages/python`, `packages/ts`, `packages/ts/wasm`, `packages/php-ext`, `packages/go/internal/ffi`
  (`packages/go/go.mod`와 함께)다. 이 crate들은 `binding scaffold`가 쓰고 `make bindings-check`가 비교한다. 루트의
  `Cargo.lock`이 `polyspec-binding-core`의 commit을 포함한 모든 의존성을 고정한다. 모든 crate는 workspace의 lint를
  받는다(`[lints] workspace = true`). binding scaffold가 이를 바인딩 manifest마다 쓰므로, 루트 `Cargo.toml`의 lint가
  코어와 바인딩 crate에 똑같이 적용되고, 규칙이 필요로 하는 사용은 `#[allow(..., reason = "<규칙 id>")]`를 가진다.
- 바인딩은 인터페이스 계약 `packages/rust/interface.json`에서 생성되며 손으로 쓰지 않는다.
- `make bindings`가 계약의 target마다 따로 생성한다(`make bindings-<target>`, binding GN-1b). 그래서 생성기가 거부하는
  target은 자기 make target만 실패시킨다. 바인딩을 빌드하거나 시험하는 모든 make target은 자기 target의 생성을 먼저
  실행한다.
  `make test-bindings`는 `make lint-bindings`(바인딩 crate의 clippy), `make test-crates`(바인딩 crate의 Rust 시험),
  그다음 다섯 target마다 하나씩 `make test-python`, `make test-node`, `make test-wasm`, `make test-php`,
  `make test-go`를 실행한다. 각각은 binding이 생성한 시험을 그 언어의 표준 runner로 실행한다: unittest,
  node --test(Node, 그리고 `make node-package`가 묶어 설치하는 같은 package의 하위 경로 `./wasm`으로 WASM),
  PHPUnit, go test. 생성기가 받지 않는 target은 원인과 함께 실패하는 target으로 남는다. 어떤 target도 빼거나
  건너뛰지 않는다(binding BD-27).
- `polyspec-binding-core`는 코어와 바인딩 crate가 binding 저장소의 tag `v0.0.3`, 곧 그 release에 두는 Git 의존성이고,
  `Cargo.lock`이 그 tag의 commit을 고정한다. `polyspec-http-media-type`은 코어가 http-media-type의
  release인 tag `v0.0.1`에 두는 Git 의존성이며, 그 바인딩 crate는 binding의 같은 tag를 적는다(T1.13). binding 자체는 같은
  release의 npm package `@polyspec/binding`이다. `package.json`이 그것을 release asset의 URL로 적고 `package-lock.json`이
  그 integrity로 고정한다. 그래서 생성기와 `binding_core`는 한 version이다(G-6). binding check는 `Cargo.lock`의 tag를
  자기 것으로 받는다(CK-20). recipe는 `node_modules/@polyspec/binding/dist/cli.js`를 실행한다. bootstrap과 `make
  install-packages`가 그것을 설치하고, `make install-binding`이 binding이 `cargo metadata --locked --offline`으로 읽는
  `Cargo.lock`의 crate를 받는다(`cargo fetch --locked`). `tests/scripts/binding.test.mts`가 tag, asset, 그 integrity, 이
  recipe를 요구한다. 변수 `PYTHON`(3.14 이상), `PHP`, `PHP_CONFIG`, `PHPUNIT`, `GO`, `GOFMT`,
  `WASM_BINDGEN`은 `var/tools`와 `vendor/`의 도구를 가리키고(도구 절 참고), `make test-python`은 `MATURIN`의 maturin을
  시험의 가상 환경에 설치한다. `make test-go`는
  `packages/go/go.mod`의 go 지시문과 다른 release의 Go를 실패시키고 `GOTOOLCHAIN=local`로 실행해 toolchain을 받지
  않게 한다. `make node-package`는 `packages/ts/wasm/Cargo.toml`의 `wasm-bindgen` crate와 다른 release의
  wasm-bindgen CLI를 실패시킨다. CLI는 자기 release의 module만 읽기 때문이다(binding GN-18). `CARGOFLAGS`는 바인딩의
  모든 cargo build에 인자를 더한다.
- 바인딩 package의 생성 폴더 `generated/`, `packages/ts/wasm/generated`, `packages/go/internal/ffi/generated`는
  추적하지 않는다. 시험의 가상 환경, wheel, 설치한 npm package를 담는 `var/`도 추적하지 않는다. Go module에서
  binding은 `zz_generated_*.go` 파일을 소유하고(binding GN-3), `make bindings`가 그 파일을 쓴다. 이 파일들과
  `packages/go/internal/ffi/lib/<platform>`의 정적 라이브러리는 추적하지 않는다. Go는 module을 tag의 소스로 받으므로
  release가 둘을 Go tag의 commit에 더하고(binding GN-15g, CK-19), 추적한 사본은 pin이 바뀔 때마다 고정된 binding이 쓰는
  것과 달라지기 때문이다.
- macOS에서 PHP 확장과 Node addon은 runtime이 불러올 때 그 runtime의 기호를 받는다. 그래서 `.cargo/config.toml`이
  linker에게 그 기호를 정의하지 않은 채 두게 하고, recipe는 Cargo가 빌드하는 library를 platform의 확장자(macOS는
  `dylib`, Linux는 `so`)로 부른다.
- 코어는 feature `fs` 없이 `wasm32-unknown-unknown`으로 빌드된다. `fs`는 요청의 파일과 그 의존성 `rustix`,
  `tempfile`을 담는다(spec F-28). 계약은 WASM 빌드의 feature를 `core.features.wasm`에 빈 목록으로 적고, WASM crate는
  코어에 `default-features = false`로 의존한다(binding GN-4a). 어떤 manifest도 `[target.'cfg(...)'.dependencies]`로
  의존성을 가르지 않는다. target이 무엇을 빌드하는지 적는 자리는 `core.features` 하나이기 때문이다. `fs`가 필요한
  항목은 `#[cfg(feature = "fs")]`를 달고, 그 unit이나 member의 `only`로 WASM 바인딩에서 빠진다. `UploadedFile`은
  메모리의 형태로 WASM에 있다. `rust-toolchain.toml`이 그
  target을 설치하고, `make check`가 실행하는 `make wasm-core`가 그 target으로 코어를 clippy와 함께 빌드한다.
- `contracts/`의 계약 case는 아직 바인딩으로 실행되지 않는다: case 열 개가 `then` 호출의 두 꼴(binding GN-13f), 곧 자기
  `faults`와 `"on": "receiver"`를 쓰는데, `Cargo.lock`이 고정하는 binding은 그것을 읽지 않는다. 그래서 그 검사가 그것을
  거부하고 `make bindings`가 바인딩을 쓰지 않는다. 언어의 case runner는 그것이 실행될 생성 바인딩이 생기기 전에는 두지
  않는다. case는 앞선 호출이 만드는 receiver를 instance 값으로 주며(binding GN-13f), 코어의 시험을 위해 `tests/common`이
  `then` 호출의 fault와 receiver까지 그것을 만든다.

## 검사

- `make check`는 rustfmt, clippy, 시험, `make wasm-core`, `make docs-check`를 실행한다.
- `make docs-check`는 `config/documents.json`이 고르는 각 문서를 그 링크, anchor, fence, 각 한국어 쌍의 `doc-id`와
  `source-sha256`과 함께 읽고, `config/checklist.json`이 행을 선언하는 checklist `docs/plans/rust-core/checklist.md`를
  읽는다. 그것이 실행하는 @polyspec/kit은 `make bootstrap`이 설치한다. 저장소의 계획은 `docs/plans/rust-core/plan.md`다.
  작업은 그 checklist의 행이며, 확인 명령이 commit된 tree에서 통과한 뒤에만 `[o]`로 표시한다. commit 전에 그것을
  실행한다. 한국어 쌍을 다시 번역한 뒤에는 `make documents-stamp PATHS=<쌍>`이 그 `source-sha256`을 쓴다. marker는 손으로
  쓰지 않는다.
- `Cargo.toml`의 `[profile.release]`는 `lto = false`를 정한다(binding GN-15i, CK-91). Go 정적 라이브러리는 release
  profile로 빌드되고, LTO는 표준 라이브러리와 `binding_core`를 그 하나의 object에 넣으므로, Go program이 그것을
  http-media-type의 정적 라이브러리와 함께 링크하면 `rust_eh_personality` 같은 중복 symbol로 실패한다. Cargo는 `lto`를
  package 하나가 아니라 profile 전체에 정하므로 profile 자체가 LTO를 끄고, release의 PHP 확장, Node addon, wheel도 LTO
  없이 빌드된다. `tests/scripts/profile.test.mts`가 이 profile을 요구한다.

## 도구

- recipe의 모든 도구는 추적 파일에 고정되고 checkout의 `var/tools`에 설치되며, 기계의 도구를 쓰지 않는다. 그래야 결과가
  기계에 있는 것에 따라 달라지지 않는다. `mise.toml`은 Node.js, npm, Go, Python, Composer, wasm-bindgen CLI, uv를 binding의
  release로 정하고 `mise.lock`은 그 digest를 정한다. `config/toolchain.json`은 mise의 release와 digest와 LLVM의 major
  release를, `rust-toolchain.toml`은 Rust toolchain과 component `llvm-tools`와 target `wasm32-unknown-unknown`,
  `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`을, `.php-version`은 `var/tools/bin`의 wrapper가
  실행하는 PHP를, `composer.json`과 `composer.lock`은 `make install-php-tools`가 `vendor/`에 설치하는 PHPUnit을 정한다.
  새 checkout은 `make bootstrap`을 실행하고(`package.json`의 `devEngines`의 npm으로 `package-lock.json`을 설치한다. 다른
  release의 npm은 폴더를 거부하므로(EBADDEVENGINES) checkout 밖에서 `--prefix`로 설치하고, 모든 install target처럼
  `$(ONLINE)`으로 내려받는다), 이어서
  `make install`을 실행한다. 모든 recipe는 `var/tools/bin`과 mise의 shim을 `PATH` 맨 앞에 두고 돈다.
- `make test-scripts`는 `tests/scripts/ci.test.mts`와 `tests/scripts/workflows.test.mts`를 실행한다. 앞의 시험은 추적
  파일이 정확한 release로 고정하지 않은 도구와 기계의 도구를 가리키는 Makefile의 도구 변수를 실패로 본다. 뒤의 시험은
  Makefile이나 release recipe의 make target 하나가 아닌 workflow step, commit으로 고정되지 않은 action, `assets.yml`의
  플랫폼 runner가 아니면서 `ubuntu-26.04-arm`이 아닌 runner의 job, 시간 한도를 실패로 본다.

## CI와 release

- CI가 실행하는 workflow(`ci.yml`, `assets.yml`, 그리고 그것들이 호출하는 모든 workflow)의 job은 각각 Makefile의
  recipe `ci-<job>` 하나다. job의 run step은 정확히 하나이고 make 변수만 인자로 받는 `make ci-<job>`이다. matrix 값이나
  호출된 workflow의 input은 `NAME=${{ matrix.key }}` 또는 `NAME=${{ inputs.key }}`로 넘기고, run 줄에는 다른 expression이
  없다. 나머지 step은 모두 runner를 준비하거나 결과를 옮기는 `uses:` step이다. checkout, `setup-php`, bootstrap action,
  보고서나 asset의 upload가 그것이다. 조건 `if:`는 `uses:` step에만 둘 수 있고, run step, job, workflow는 환경 변수를
  정하지 않는다. recipe가 job의 순서와 조건을 가진다. 먼저 설치를 실행하고 그것이 실패하면 멈추며, 다음에 검사를 하나씩
  끝까지 실행하고(`make ci-targets`, 또는 `make -k` 하나의 goal들), 마지막에 보고서를 항상 쓴다. recipe의 status는 검사의
  status다. YAML이 가졌던 값, 예를 들어 matrix 항목의 대상은 Makefile의 변수다. 이유는 다음과 같다. 내용이 YAML의 step,
  step 조건, expression에 있는 job은 GitHub runner에서만 돌므로 local 실행이 CI의 실행과 같을 수 없었다. 내용이 recipe에
  있으면 `make ci-<job> NAME=value`가 local과 CI에서 같은 호출이다. 판정 job `ci-passed`는 다른 job의 결과를 읽으므로 CI에만
  있다. 그 run step 하나는 `make ci-passed RESULTS='${{ toJSON(needs) }}'`이고, 이것이 허용되는 유일한 다른 expression이다.
  `release.yml`의 job `tag`와 `release`도 이 규칙을 따르며 `make ci-tag`와
  `make ci-release`다. token, CI 실행의 id, actor, `needs`의 output 같은 실행의 값은 job의 `env:`로 받아 run 줄이
  `make ci-<job>`으로 남고, job `release`는 run step 앞에서 각 플랫폼의 asset을 내려받는다. `make test-scripts`가 이 규칙을 검사한다(`tests/scripts/workflows.test.mts`).
- `assets.yml`은 PHP 이외의 build를 platform마다 한 번씩 `platform`에 두고 `php`에 platform과
  `phpExtension.minors`의 모든 조합을 둔다. 각 PHP 항목은 `matrix.php`의 setup 하나,
  `make ci-php EXPECTED_PHP_MINOR=${{ matrix.php }}` 하나, 서로 다른 artifact
  `assets-php-${{ matrix.platform }}-${{ matrix.php }}`를 가진다. `ci-php`의 recipe는 `make release-php-ext
  PHP=php EXPECTED_PHP_MINOR=$(EXPECTED_PHP_MINOR)`로 extension을 한 번 빌드한다. 두 job은 각각 자기 platform의 runner를 쓴다.
  release는 `assets-*` artifact를 병합한다. 이 소유 관계는 한 minor가 다른 minor의 결과를 대체하지 못하게 하고
  local run이 실제 runtime 하나를 선택하게 한다. workflow test는 선언된 구조를 증명한다. T1.13과 T1.19의 native 검증은 expected-minor
  검사(B2.275)를 가진 binding recipe의 공개, 재생성, 실제 실행을 기다린다. 건너뛴 setup action은 runtime 선택을
  증명하지 않는다.
- `.github/workflows/ci.yml`은 `main`의 push와 수동 실행에서 돈다. matrix 항목마다 job 하나다. `core`는 `fmt`, `clippy`,
  `test`, `wasm-core`, `bindings-check`, `docs-check`, `test-scripts`, `release-coverage`, 그리고 tag 없이 커밋된
  HEAD에서 release의 archive를 만드는 `release-candidate-assets`를 실행하고, `bindings`는 `make
  test-bindings`의 대상을 실행한다. 각 job은 `make ci-check NAME=<name>`을 실행한다. 그 recipe는 항목의 도구를 설치하고(`CI_CHECK_INSTALLS_<name>`), 항목의
  대상(`CI_CHECK_TARGETS_<name>`)을 `make ci-targets`로 실행하며, `make ci-summary`로 job의 요약을 쓴다. `make ci-targets`는
  모든 대상을 끝까지 실행하고 보고서 `var/ci/<name>/`을 쓴다. job `assets`는 `.github/workflows/assets.yml`을 호출하고, 그것은
  release와 같이 각 플랫폼의 asset을 빌드한다. 그래서 빌드되지 않는 asset이 tag가 생기기 전에 release를 멈춘다. 마지막
  job `ci-passed`는 다른 모든 job이 통과했을 때만 통과한다. job은 `ubuntu-26.04-arm`에서 돈다. 다만 `assets.yml`의 job
  `platform`과 `php`는 `ubuntu-26.04`, `ubuntu-26.04-arm`, `macos-15`에서 돈다. 플랫폼의 asset은 그 플랫폼에서만 빌드되고
  로드되며, release의 어떤 도구도 다른 플랫폼용으로 컴파일하지 않기 때문이다(binding GN-19a).
  `.github/actions/bootstrap`은 모든 job의 bootstrap이다.
- binding이 contract의 release recipe를 쓴다(binding GN-19h). `make release-recipes`가 `binding release-recipes --root .`을
  실행해 `packages/release.mk`를 쓰고, Makefile은 `BINDING_RUN`, `NODE`, `NPM`, `UV`, `BINDING_DEP`를 정의한 뒤 그 파일을
  include한다. 그 파일은 생성되고 추적되지 않으므로 고정된 binding과 어긋날 수 없고, 이 저장소는 자기 release recipe를
  쓰지 않는다. `binding check`는 include가 없거나 그 파일의 target에 자기 recipe를 가진 Makefile을 보고한다(CK-94).
  recipe에는 `package.json`의 napi CLI `@napi-rs/cli`, `config/toolchain.json`의 LLVM major release(job이 준비한다. ubuntu-26.04
  image는 `/usr/lib/llvm-<major>`에 두고, macOS에서는 install-llvm-action이 설치하며 `make ci-php`가 그 `LLVM_PATH`를
  `LLVM_ROOT`로 넘긴다), `rust-toolchain.toml`의 component `llvm-tools`와 musl target, `mise.toml`의 uv가 필요하다.
- `config/release.json`은 모든 package 파일을 분류하고, `make release-coverage`가 그것을 검사한다. tag `vX.Y.Z`는 코어
  crate `packages/rust`를 Git tag로 release한다. Cargo 의존은 그것을 `tag = "vX.Y.Z"`로 가리킨다. 그 tag는 binding
  manifest의 version이기도 하며, binding scaffold가 코어의 version으로 그 manifest를 쓴다. kit은 PHP 확장
  `packages/php-ext`의 Composer package를 묶는다. 그 `composer.json`은 PIE에게 pre-packaged binary를 내려받게 한다.
  workflow `assets.yml`은 binding의 recipe로 각 플랫폼의 asset을 빌드한다. Go module의 정적 라이브러리, abi3 wheel,
  `phpExtension.minors`가 선언한 PHP minor 8.4와 8.5용 PHP 확장, npm 플랫폼 package, 그리고 WASM module을 가진 npm package 자체다. Go module `packages/go`는
  `goModules`에 없다. `make release-go-module`이 tag의 tree에 생성 파일과 정적 라이브러리를 더한 commit을 만들고, 그것을
  consumer로 증명하고, 그 commit에 `packages/go/vX.Y.Z`를 tag한다(binding GN-19f). 둘 다 없는 tag의 Go module은
  consumer가 빌드할 수 없기 때문이다.
- release는 `main`에서 성공한 `ci.yml`의 push 실행이나 수동 실행의 commit이다. version-bump commit `chore(release):
  Release X.Y.Z`는 코어의 version(루트 `Cargo.toml`의 `[workspace.package]`의 `version`)을 정하고, binding
  scaffold(`node node_modules/@polyspec/binding/dist/cli.js scaffold --contract packages/rust/interface.json`)로 binding manifest를 다시 쓰며,
  `Cargo.lock`의 로컬 package version을 맞추고, 두 changelog의 `## Unreleased`를 `## X.Y.Z`로 바꾸고 그 위에 빈
  `## Unreleased`를 둔다. 그 push의 CI 실행이 성공하면 `.github/workflows/release.yml`이 스스로 시작하고, `gh workflow run
  release.yml -f run=<id>`가 그것을 다시 시작한다. `make release-tag`가 그 commit에 annotated tag `vX.Y.Z`를 만들어
  push하고, `assets.yml`이 `make release-binding-check` 뒤에 tag에서 각 플랫폼의 asset을 빌드하며, `make release-publish`가
  kit의 archive로 draft GitHub Release를 만들고, `make release-go-module`이 Go module을 옮기고, `make
  release-platform-assets`가 각 플랫폼의 asset을 label `polyspec-http-form-<runtime>-<version>-<platform>`으로
  올리며(binding GN-19g), `make release-undraft`가 release를 공개한다. tag를 손으로 만들지 않는다.
- 첫 release 0.0.1은 코어의 tag와 함께 플랫폼별 바인딩 package를 갖는다(T1.13, T1.19). release의 브랜치는 `main`이다.
  GitHub이 `v1/rust-core`를 `main`으로 이름을 바꿨다(T1.22).
  코어와 바인딩 crate는 http-media-type 0.0.3과, http-media-type 0.0.3이 의존하는 binding 0.0.5에 의존한다. 그래서
  `Cargo.lock`은 `polyspec-binding-core` 하나를 고정한다(T1.19-5). 첫 release는 바인딩도 기다린다(T1.8~T1.11, T1.23).

## Commit과 변경

- 추적 파일은 이 저장소든 다른 저장소든 Git commit을 적지 않는다. 기록은 무엇이 바뀌었고 검사가 무엇을 냈는지를 적고,
  의존은 `Cargo.lock`이 고정한 것("고정된 binding")이나 그 release로 가리킨다. commit id는 저장소 없이 읽는 사람에게
  아무것도 알려 주지 않고, history가 다른 history와 합쳐지거나 다시 쓰이면 아무것도 가리키지 않는다. `v1/rust-core`를
  `main`으로 옮길 때가 그렇다. commit으로 고정하는 파일은 예외다. lock 파일과 workflow의 action이다. `make
  test-scripts`는 `tests/scripts/commits.test.mts`를 실행하며, 이 시험은 commit id를 가진 추적 파일을 실패로 본다.
