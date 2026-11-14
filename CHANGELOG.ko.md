<!-- doc-id: changelog -->
<!-- source-sha256: 92ca5167ca6915389331f223d5d7bfcbc6492b5e367c68855624098742e7d749 -->

## Unreleased

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
