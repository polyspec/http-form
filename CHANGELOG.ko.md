
- 요청 디렉터리의 새 파일 경로가 이미 있으면 그 chunk는 파일 시스템 실패 `io`와 `errno` `EEXIST`로 실패한다(F-10).
  전에는 `internal`로 실패했다. 코어는 한 파일 이름을 두 번 만들지 않지만, 같은 사용자의 다른 프로세스가 그 디렉터리에
  파일을 만들 수 있으므로 이것은 환경의 조건이다.

- `make check`가 macOS에서 통과한다. `tests/behavior.rs`의 도우미 `last`는 Linux에서만 도는 다른 파일 시스템 이동
  시험만 쓰므로 그 시험 안으로 옮겼다. 전에는 macOS에서 clippy가 이 도우미를 dead code로 실패시켰다.

- `Cargo.lock`은 같은 라이브러리 묶음의 모든 레포가 고정하는 binding 9de9d03과 그 위의 `polyspec-http-media-type` 커밋을 고정한다(CK-20). 그 scaffold는 같은 바인딩 package를 쓴다.
