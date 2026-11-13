<!-- doc-id: agents -->
<!-- source-sha256: ed9b7ddde15c76aabee2269fba1b23aeee1a5ef2fd283b1bc540c4c2bd03bb50 -->
# 개발

모든 polyspec 저장소의 작업 원칙은 polyspec의 `PRINCIPLES.md`에 있다. 이 파일은 이 저장소의 규칙만 적는다.

## 바인딩

- 저장소는 Cargo workspace 하나다: 코어 `packages/rust`와, 계약의 다섯 target(binding BD-27)의 바인딩 crate
  `packages/python`, `packages/ts`, `packages/ts/wasm`, `packages/php-ext`, `packages/go/internal/ffi`
  (`packages/go/go.mod`와 함께)다. 이 crate들은 `binding scaffold`가 쓰고 `make bindings-check`가 비교한다. 루트의
  `Cargo.lock`이 `polyspec-binding-core`의 commit을 포함한 모든 의존성을 고정한다.
- 바인딩은 인터페이스 계약 `packages/rust/interface.json`에서 생성되며 손으로 쓰지 않는다. 이전 API의 손으로 쓴
  바인딩은 `legacy/`에 있고 어떤 target도 그것을 빌드하지 않는다. 생성된 바인딩이 시험을 통과할 때까지 남는다.
- `make bindings`가 생성한다. 바인딩을 빌드하거나 시험하는 모든 make target은 그것을 먼저 실행한다.
  `make test-bindings`는 `make lint-bindings`(바인딩 crate의 clippy), `make test-crates`(바인딩 crate의 Rust 시험),
  그다음 다섯 target마다 하나씩 `make test-python`, `make test-node`, `make test-wasm`, `make test-php`,
  `make test-go`를 실행한다. 각각은 binding이 생성한 시험을 그 언어의 표준 runner로 실행한다: unittest,
  node --test(Node, 그리고 `make node-package`가 묶어 설치하는 같은 package의 하위 경로 `./wasm`으로 WASM),
  PHPUnit, go test. 생성기가 받지 않는 target은 원인과 함께 실패하는 target으로 남는다. 어떤 target도 빼거나
  건너뛰지 않는다(binding BD-27).
- 변수 `BINDING`(binding의 cli), `PYTHON`(3.14 이상), `MATURIN`, `PHP`, `PHP_CONFIG`, `PHPUNIT`, `NODE`, `NPM`, `GO`,
  `GOFMT`, `WASM_BINDGEN`이 도구를 가리킨다. recipe는 도구를 설치하지 않는다. `make test-go`는
  `packages/go/go.mod`의 go 지시문과 다른 release의 Go를 실패시키고 `GOTOOLCHAIN=local`로 실행해 toolchain을 받지
  않게 한다. `make node-package`는 `packages/ts/wasm/Cargo.toml`의 `wasm-bindgen` crate와 다른 release의
  wasm-bindgen CLI를 실패시킨다. CLI는 자기 release의 module만 읽기 때문이다(binding GN-18). `CARGOFLAGS`는 바인딩의
  모든 cargo build에 인자를 더한다.
- 바인딩 package의 생성 폴더 `generated/`, `packages/ts/wasm/generated`, `packages/go/internal/ffi/generated`는
  추적하지 않는다. 시험의 가상 환경, wheel, 설치한 npm package를 담는 `var/`도 추적하지 않는다. Go module에서
  binding은 `zz_generated_*.go` 파일을 소유한다(binding GN-3). Go module은 소스로 받아지므로, `make bindings`가 그
  파일을 쓰면 추적한다.
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
- `contracts/`의 계약 case는 아직 바인딩으로 실행되지 않는다: `binding check`가 `CHANGELOG.md`에 적은 CK-22
  finding으로 계약을 거부하므로 `make bindings`가 바인딩을 쓰지 않는다. 언어의 case runner는 그것이 실행될 생성
  바인딩이 생기기 전에는 두지 않는다.
