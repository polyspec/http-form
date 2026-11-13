
- `make check`가 macOS에서 통과한다. `tests/behavior.rs`의 도우미 `last`는 Linux에서만 도는 다른 파일 시스템 이동
  시험만 쓰므로 그 시험 안으로 옮겼다. 전에는 macOS에서 clippy가 이 도우미를 dead code로 실패시켰다.

- `Cargo.lock`은 같은 라이브러리 묶음의 모든 레포가 고정하는 binding 9de9d03과 그 위의 `polyspec-http-media-type` 커밋을 고정한다(CK-20). 그 scaffold는 같은 바인딩 package를 쓴다.
