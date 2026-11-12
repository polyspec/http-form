# 변경 기록

## 미출시

- 코어 crate는 `packages/rust/`의 `polyspec-http-forms`이고 규칙은 `docs/spec/forms.md`에 있다.
- 모든 실패는 kind, status, 파일 시스템 실패에는 `errno`를 가진 `Error`다. `None`으로 실패하는 함수는 없다.
- `RequestBody::new`가 `Content-Length`를 받는다. `feed`는 바이트 위치로 정한 본문의 첫 실패를 내고 이후 모든 호출이 같은 실패를 낸다. host가 `finish`를 부른다. `parts`는 `not_finished`, `not_streamed`, 또는 본문의 실패를 낸다.
- boundary가 없는 `multipart/form-data`는 `no_boundary`로 실패한다. 닫히지 않은 따옴표 parameter 값은 실패한다. 긴 header 줄은 `header_limit`로 실패한다.
- `largest_file` 대신 `Form::file_over(limit)`를 둔다.
- `UploadedFile`의 복제본은 상태를 공유한다. 옮기기는 파일을 덮어쓰지 않고, 다른 파일 시스템으로만 복사하며, `source_left`와 `partial_target`을 낸다. `remove` 뒤에 저장된 파일은 `removed`로 실패한다.
- 요청 디렉터리는 `tempfile`로 만들고 이름은 `http-forms-…`이다. `remove` 없이 버려진 본문은 디렉터리를 지운다.
- 손으로 쓴 PHP, Python, Node 바인딩은 더 이상 빌드하지 않는다.
- `RequestBody::new`, `feed`, `finish`, `remove`, `UploadedFile::content`, `move_to`는 첫 인자로 기다림 토큰 `bindgen_core::Io`를 받는다(F-17).
- `Content-Type`은 `polyspec-http-media-type`이 media type으로 읽는다(F-12). `media_type()`이 `mediaType()`을 준다. 앞뒤에 줄바꿈이나 A0 바이트가 있는 값, 닫히지 않은 따옴표, 두 번 준 parameter, 빈 값처럼 media type이 아닌 값은 `malformed_content_type`(400)로 실패하며 media type의 실패를 담는다. urlencoded type 앞의 공백은 무시한다. 따옴표 boundary의 quoted pair는 그 바이트가 된다. content type 인자는 `Option<&[u8]>`이고 `None`은 `Content-Type`이 없는 요청이다.
- `Content-Disposition`이 없거나, disposition type이 다르거나, `name`이 없는 part와 `Content-Disposition`에 두 번 준 parameter는 무시하지 않고 본문을 `invalid`로 실패시킨다(F-16).
- `HEADER_LIMIT`은 `u64`다. `Error`는 `bindgen_core::Kinds`를, `RequestBody`는 `bindgen_core::Cleanup`을 구현한다.
- 인터페이스 계약은 `packages/rust/interface.json`이다(F-18).
- `Content-Disposition`이나 `Content-Type` 줄이 둘이거나 `:` 없는 header 줄이 있는 part는 본문을 `invalid`로 실패시킨다(F-16). form type이 정의하지 않은 parameter는 무시한다(I-7).
- 코어는 인터페이스 계약의 타입을 받고 돌려준다. `RequestBody::new`는 한도를 record `BodyLimits`로 받는다. `Fields`와 `Files`는 이름과 그 값의 목록이다. `Form`은 record이고 `file_over(files, limit)`는 함수다. `Collector`가 reader의 조각을 모은다. `held`, `directory`, `UploadedFile`의 이름들은 소유 값이다. `Error::removal_kind`와 `removal_errno`가 `partial_target`의 삭제 실패를 준다.
- crate는 고정 toolchain인 Rust 1.99.0을 요구한다. `rustfmt.toml`이 형식을 정하고 `make fmt`가 검사한다.
- form 값, 파일, 조각으로 읽는 본문의 규칙은 `docs/spec/forms.md`의 F-20~F-25와 F-27이고, 그 case는 사본이 아닌 `contracts/fields.json`과 `contracts/forms.json`이다. `sources.json`과 그 검사는 없어졌다. 8192바이트보다 긴 part header는 그 case에서도 `header_limit`로 실패하고, `moved`, `exists`, `removed`의 메시지는 F-11과 F-10을 가리킨다.
- `polyspec-http-media-type`과 `polyspec-bindgen-core`는 다른 checkout의 경로가 아니라 branch `v1/rust-core`의 Git 의존이며 `Cargo.lock`이 커밋을 고정한다.
