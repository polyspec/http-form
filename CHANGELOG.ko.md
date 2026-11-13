<!-- doc-id: changelog -->
<!-- source-sha256: 6b2ab546a42b469e0c067ac44878aac480fec987ab772fe4f0c0e93c3fbafa0f -->

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

- `Cargo.lock`은 같은 라이브러리 묶음의 모든 레포가 고정하는 binding 9de9d03을 고정하고(CK-20), 그 위의
  `polyspec-http-media-type` 커밋을 고정한다. binding의 scaffold는 같은 바인딩 package를 쓴다.
