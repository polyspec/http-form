<!-- doc-id: readme -->
<!-- source-sha256: 06166809fcd33135d0da200e1f24272f16f757e53c93220ef2949775f8bc6b51 -->
# http-form

form 값과 `multipart/form-data` 본문을 읽는 라이브러리이다. Rust 구현 하나가 읽고, 모든 계약이 적는 다섯
target인 PHP, Python, Node, 브라우저(WASM), Go가 그 구현을 바인딩한다. 조각으로 읽는 본문과 그 파일은 WASM 바인딩에
없다(F-28). 바인딩은 type을 변환한다. 모든 규칙은 코어에 있으므로 세 서버는 같은 코드를 실행하고
같은 결과를 낸다.

```
packages/rust/   Rust crate polyspec-http-form: 모든 규칙, 그리고 인터페이스 계약 interface.json
packages/ts/ packages/ts/wasm/ packages/python/ packages/php-ext/ packages/go/   binding scaffold가 쓰는 얇은
                 바인딩 package
docs/spec/       forms.md: 규칙, 오류, API
contracts/       fields.json과 forms.json: 규칙의 case
conformance/     hyper의 Python reader와 비교하는 differential 시험
legacy/          이전 API의 손으로 쓴 바인딩과 그 runner. 빌드하지 않는다
```

## 사용

```rust
// `io()`는 blocking 호출 하나의 wait token `binding_core::Io`이며, 생성된 코드가 만든다(F-17).
let limits = BodyLimits { limit, value_limit, part_limit, file_limit };
let mut body = RequestBody::start(io(), content_type, content_length, true, limits)?;
for chunk in chunks {
    body.feed(io(), chunk)?; // 본문의 첫 실패. kind와 상태를 가진다
}
body.finish(io())?;
let form = body.parts()?;
// ...
body.remove(io())?;
```

## 검사

```
make test          # 코어에서 도는 case. 각 본문을 모든 분할로 읽는다
make fmt           # rustfmt.toml의 형식
make bindings BINDING=<binding>/packages/ts/src/cli.ts         # 바인딩 소스를 생성한다
make bindings-check BINDING=<binding>/packages/ts/src/cli.ts   # 계약과 바인딩 package
make clippy        # lint
make differential HYPER=<hyper>/packages/server/python/src   # 무작위 본문을 hyper의 Python reader와 비교한다
make check         # clippy와 test
```

요구 사항: `rust-toolchain.toml`의 Rust toolchain, 그리고 Git 의존성 `polyspec-http-media-type`과
`polyspec-binding-core`(브랜치 `v1/rust-core`. 그 commit은 `Cargo.lock`이 고정한다). Linux와 macOS(docs/spec/forms.md,
F-14). Windows는 지원하지 않는다.
