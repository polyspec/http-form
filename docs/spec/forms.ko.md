<!-- doc-id: spec-forms -->
<!-- source-sha256: b9d58d230ec45109bf4fc4cdf01a6e422ab70fe3987544cebaa5afa1e6537d67 -->
# http-form: 규칙과 API

이 라이브러리는 form 값과 `multipart/form-data` 본문을 읽는다: query나 본문의 `application/x-www-form-urlencoded`
텍스트, multipart 본문의 값과 파일, 한도와 함께 chunk로 읽는 본문, 그리고 요청의
파일이다. 이 라이브러리의 규칙은 이 문서의 규칙 F-n이다. hyper처럼 이 라이브러리를 쓰는 서버는 자신에게만
속하는 것을 정하고 이 규칙들을 가리킨다: 한도의 값과 기본값, 애플리케이션의
form type, 요청 검사의 순서, 서버의 API와 host이다("서버가 정하는 것" 절). content type은
media type이며, 그 문법은 의존성 `polyspec-http-media-type`의 규칙 MT-1부터 MT-9이다
(RFC 9110, 8.3.1절과 5.6.6절). 이 문서는 그 규칙들을 그 식별자로 부른다.

`contracts/fields.json`과 `contracts/forms.json`은 이 규칙들의 case를 담는다. `fields.json`은
urlencoded 텍스트, multipart 본문, chunk로 읽는 본문, 바이트로 만든 파일의 case를 담고, `forms.json`은
나머지 규칙의 case를 담는다. case는 binding의 case 규약(G-5)을 따른다: case의 키는 인터페이스 계약에서 그 group이
가리키는 member의 매개변수 이름이고, 바이트는 `{"hex": ...}`이며, content type은 문자가 곧 바이트인 텍스트이고,
한도는 record `limits`이다. `RequestBody`의 method는 receiver를 생성자의 키에서
받으므로, `request_body.feed`는 chunk 하나의 실패를 담고, `request_body.finish`와 `request_body.parts`는
방금 만든 본문의 실패를 담는다. 앞선 호출이 만든 receiver의 case는 그것을 instance 값으로 키 `receiver` 아래에
주며(binding GN-13f), 이름이 `.made`와 `.stored`로 끝나는 group에 있다(결함 주입 지점). 호출이 둘 이상인 scenario(`forms.json`의 `body`,
`fields.json`의 `streamed`)는 코어의 시험만 읽는 case이다.

## 구현 하나

- **F-1** 모든 규칙은 Rust crate `polyspec-http-form`(`packages/rust/`)에 있다: urlencoded reader, content
  type, part의 헤더 줄과 매개변수, 본문을 chunk로 읽는 상태 기계, 값 한도와 part 한도와
  파일 한도, 요청의 디렉터리, 그 파일의 쓰기와 읽기와 이동과 삭제, 그리고 host가 읽는 대로의 요청
  본문(`RequestBody`)이다.
- **F-2** 바인딩은 type을 변환하고 코어의 오류를 그 runtime의 class로 바꾼다. 바인딩은 규칙을 갖지 않는다:
  parsing도, 한도도, 파일 연산도, 자신의 메시지도 없다. 바인딩은 인터페이스 계약에서
  생성된다. `php/`, `python/`, `node/`의 손으로 쓴 바인딩은 빌드하지 않는다.
- **F-3** runtime만 할 수 있는 일은 그 runtime의 서버에 남는다: 요청의 바이트를 host에서
  읽어 `Content-Type`과 `Content-Length`의 값과 함께 `RequestBody`에 넘기는 일이다.

## 바이트와 텍스트

- **F-4** content type, `Content-Length`, boundary, 본문은 바이트이다. `Content-Type`이 없는 요청은
  코어에 content type을 주지 않으며(`None`), 이것은 빈 바이트와 다른 값이다.
- **F-5** 코어가 돌려주는 이름, 값, filename, content type은 코어가 검사한 UTF-8 텍스트이다
  (F-20.3, F-21.5, F-22.4). 파일의 바이트는 바이트이며, 애플리케이션이 만드는 파일의 바이트도 그렇다
  (`UploadedFile::from_value`, F-22.2).

## 오류

모든 실패는 바뀌지 않는 kind를 가진 `Error`이다(plan F-2). 요청 본문의 실패는 서버가 응답하는
상태를 가진다. 범주는 바인딩의 오류 class이다(plan F-3).

| Kind | 상태 | 범주 | 실패 |
|---|---|---|---|
| `invalid` | 400 | value | F-20.3, F-21.5, F-22.4, F-24.5; 끝이 없는 `Content-Disposition`의 따옴표 매개변수 값(F-6); 이름을 가진 `form-data` part가 아닌 part, 또는 매개변수를 두 번 주는 part(F-16) |
| `header_limit` | 400 | value | 8192바이트보다 긴 part의 헤더 줄(F-23.4) |
| `no_boundary` | 400 | value | boundary가 없거나 빈 boundary를 가진 `multipart/form-data` content type(F-7) |
| `malformed_content_type` | 400 | value | media type이 아닌 `Content-Type`(F-12) |
| `invalid_content_length` | 400 | value | 하나 이상의 숫자가 아닌 `Content-Length`(RFC 9110, 8.6절) |
| `body_limit` | 413 | value | 본문 한도(F-25.1) |
| `value_limit` | 413 | value | 값 한도(F-25.2) |
| `part_limit` | 413 | value | part 한도(F-25.3) |
| `file_limit` | 413 | value | 파일 한도(F-25.4) |
| `not_finished` | 없음 | type | `finish()` 전의 `parts()`(F-9) |
| `not_streamed` | 없음 | type | 한 값으로 보관한 본문의 `parts()`(F-9) |
| `finished` | 없음 | type | `finish()` 뒤의 chunk(F-9) |
| `moved` | 없음 | value | 이동한 파일의 이동이나 읽기(F-11), 메시지 `the file was moved (F-11)` |
| `exists` | 없음 | value | 이미 있는 경로로의 이동(F-11), 메시지 `the path exists (F-11)` |
| `removed` | 없음 | value | 디렉터리를 삭제한 뒤에 하는 저장된 파일의 이동이나 읽기, 그 뒤의 chunk(F-10) |
| `not_found`, `permission_denied`, `storage_full`, `io` | 없음 | io | 파일 시스템의 실패. kind는 그 `io::ErrorKind`에 따르고 `errno`를 가지며(plan F-2), 계약의 `Io` 행의 `kinds`이다; `EXDEV`는 `io`이다. rename만 그것을 주고 이동은 그때 복사하기 때문이다(F-11); 크기 검사는 `errno` 없는 `io`이다(F-15); 요청 디렉터리의 새 파일의 경로가 이미 있으면 `errno` `EEXIST`를 가진 `io`이다(F-10) |
| `source_left` | 없음 | io | 복사했으나 원본이 삭제되지 않은 파일(F-11) |
| `partial_target` | 없음 | io | 실패한 복사나 쓰기이며, 완성되지 않은 대상이 삭제되지 않았다(F-11) |
| `internal` | 없음 | internal | 코어의 불변식이 성립하지 않는다(H-8) |

unsigned integer가 아닌 한도의 kind `invalid_number`(범주 value)와 경로가 아닌 값의 kind `invalid_path`는
바인딩의 것이다(plan B-2, B-3).

## Form 값과 파일

- **F-20** `parse_urlencoded(text)`는 query나 본문의 `application/x-www-form-urlencoded` 텍스트를 nesting 없이
  읽는다.
  1. F-20.1 텍스트는 모든 `&`에서 나뉘고, 빈 조각은 건너뛴다(I-6). 조각의 이름은 첫 `=` 앞의
     텍스트이고 값은 그 나머지이며, 조각에 `=`가 없으면 값은 빈 텍스트이다. 이름과 값 모두에서 `+`는 공백이고,
     16진 숫자 두 개가 붙은 `%XX`는 그 바이트이며, 그 밖의 `%`는 그대로 남는다.
  2. F-20.2 `roles[]`나 `a[b]`처럼 대괄호가 있는 이름은 적힌 그대로의 이름이고, 빈 이름도 이름이다.
  3. F-20.3 결과는 각 이름을 첫 등장 순서로 나열하고 그 값을 요청 순서로 담은 list이다
     (`Fields`). list는 10진수 이름을 모든 언어에서 텍스트로 유지한다. UTF-8이 아닌 이름이나 값은
     `invalid`로 실패한다.
- **F-21** 본문의 form 값.
  1. F-21.1 form 값은 raw 본문에서 nesting 없이 읽으며, F-20.3의 list와 같은 list이다.
  2. F-21.2 `application/x-www-form-urlencoded` 본문(F-12.2)은 F-20으로 읽는다.
  3. F-21.3 `multipart/form-data` 본문은 `Content-Disposition`에 `name` 매개변수가 있고
     `filename` 매개변수가 없는 part마다 값 하나를 준다(F-16): 값의 이름은 그 매개변수이고 값은 part의 content
     그대로이다. `filename`이 있는 part는 파일이며 값이 아니다.
  4. F-21.4 다른 type의 본문과 `Content-Type`이 없는 요청의 본문에는 form 값이 없다.
  5. F-21.5 UTF-8이 아닌 이름이나 값은 본문을 `invalid`로 실패시킨다.
- **F-22** `multipart/form-data` 본문의 파일.
  1. F-22.1 파일 part는 `Content-Disposition`에 `name` 매개변수와 `filename` 매개변수가 모두 있는 part이다(F-16).
  2. F-22.2 파일(`UploadedFile`)은 `name` 매개변수인 이름, 보낸 그대로이며 decoding하지 않은 `filename`
     매개변수인 filename, 보낸 그대로의 `Content-Type` 줄의 값이거나 그 줄이 없으면
     빈 텍스트인 content type, 그리고 part의 content 그대로인 바이트를 가진다. 애플리케이션은 바이트로
     파일을 만들 수 있고(`UploadedFile::from_value`), 그 파일의 크기는 그 바이트의 수이다.
  3. F-22.3 filename이 빈 파일도 파일이다.
  4. F-22.4 이름, filename, content type은 UTF-8이며, 이들이 UTF-8이 아닌 part는 본문을
     `invalid`로 실패시킨다.
  5. F-22.5 `filename*`(RFC 5987)은 읽지 않는다(I-4).
  6. F-22.6 파일들은 각 이름을 그 첫 파일의 순서로 나열하고 그 파일을 요청 순서로 담은 list이다
     (`Files`).
- **F-23** part의 헤더 줄.
  1. F-23.1 part의 헤더 줄은 CRLF로 구분된다. 어떤 이름의 줄은 첫 `:` 앞의 텍스트가 대소문자 구분 없이
     그 이름과 같은 줄이고, 그 줄의 값은 줄의 나머지에서 앞뒤의 공백과 tab을 뺀
     것이다. part가 어떤 줄을 가지는지는 F-16이 정한다.
  2. F-23.2 `Content-Disposition`은 첫 `;` 앞의 텍스트가 앞뒤의 공백과 tab을 빼고
     대소문자 구분 없이 `form-data`일 때 `form-data`이다. 각 매개변수는 `;`과 공백이나 tab 뒤에 오며, `=`까지가
     대소문자 구분 없이 비교하는 이름이고, 값은 따옴표를 뺀 quoted string(F-6)이거나 다음 `;`, 공백, tab까지의
     텍스트이다. quoted string 안의 텍스트는 매개변수가 아니고, no-break space 같은 다른 공백 문자는
     아무것도 구분하지 않는다.
  3. F-23.3 실패하는 part는 본문 전체를 실패시킨다: 그 본문의 form 값과 파일이 모두 실패한다.
  4. F-23.4 part의 헤더 줄은 delimiter 줄의 끝부터 빈 줄까지 최대 8192바이트이다
     (`HEADER_LIMIT`). 헤더 줄이 더 긴 본문은 `header_limit`(400)으로 실패한다.
- **F-27** `file_over(files, limit)`는 `limit`보다 바이트가 많은 파일이 있으면 true이고, 모든 파일이 `limit`바이트
  이하이거나 파일이 없으면 false이다. `limit`는 `uint`이므로 바인딩은 음수, 소수, boolean
  값을 파일을 비교하기 전에 `invalid_number`로 거부한다(plan B-2).

## Content type과 매개변수

- **F-6** `Content-Disposition`의 따옴표 매개변수 값은 그 `"`와 다음 `"` 사이의 텍스트이며 모든
  backslash를 유지한다. 브라우저가 filename의 backslash는 그대로 보내고 `"`는 `%22`로 보내기 때문이다. 따옴표가
  없는 값은 텍스트 그대로이며 `"`를 담고 있어도 그렇다. 다음 `"`가 없는 따옴표 값은 `invalid`로 실패한다.
  브라우저는 filename의 `"`를 `%22`로 보내므로 form의 어떤 client도 그런 값을 보내지 않는다. 그런 값을 텍스트의
  끝까지 읽으면 client가 보내지 않은 값이 나왔다(plan P-3). content type의 따옴표 값은 MT-3.3과 MT-3.4를
  따른다(F-12).
- **F-7** `boundary` 매개변수가 없거나 그 값이 빈 `multipart/form-data` content type은
  `no_boundary`(400)로 실패한다. 그런 본문은 읽을 수 없으며, 다른 type의 본문으로 읽으면 본문을 버렸다(plan P-4).
- **F-12** `Content-Type`의 값은 `polyspec_http_media_type::parse`(MT-4)가 읽으며, 이 라이브러리의 다른 reader는
  읽지 않는다. media type이 아닌 값은, 빈 값을 포함하여, `malformed_content_type`(400)으로 실패한다.
  그 오류는 `parse`의 실패를 source로 가지며, 그 실패의 kind `malformed_media_type`에는 상태가 없다. 상태는 호출자가
  정하기 때문이다(MT-8.1). `Content-Type`이 없는 요청에는 media type이 없다(F-4).
  1. F-12.1 `media_type(content type)`은 소문자로 된 type, `/`, subtype이며(MT-1.2) 매개변수는 없고,
     `Content-Type`이 없는 요청에서는 빈 텍스트이다. 서버는 이것을 자신의 form type과 비교한다.
  2. F-12.2 `is_urlencoded(content type)`은 type과 subtype이 `application/x-www-form-urlencoded`이면 true이고,
     다른 type이거나 type이 없으면 false이다.
  3. F-12.3 `multipart_boundary(content type)`은 type과 subtype이 `multipart/form-data`이면 MT-3.4를 적용한
     `boundary` 매개변수의 값이거나 `no_boundary`(F-7)이고, 다른 type이거나 type이 없으면 `None`이다. 대소문자 구분
     없이 비교하여 두 번 준 매개변수는 media type을 실패시키며(MT-3.5), `boundary`도 그렇다.
  4. F-12.4 그 type이 정의하지 않는 매개변수는 아무것도 바꾸지 않는다(I-7). 본문을 읽는 방법을 바꾸지 않기
     때문이다: `application/x-www-form-urlencoded`는 매개변수를 정의하지 않고 그 parser는 UTF-8을 decode하며, "only UTF-8 is
     conforming"(WHATWG URL, 5절, 5.1)이므로 `charset`도 아무것도 바꾸지 않고 UTF-8이 아닌 본문은
     `invalid`로 실패한다(F-21.5). `multipart/form-data`는 `boundary`만 정의하며, 그 optional parameter는 "none"이다(RFC
     7578, 8절).

## 본문 읽기

- **F-24** chunk로 읽는 `multipart/form-data` 본문(`stream`을 준 `RequestBody`, `MultipartReader`).
  1. F-24.1 reader는 본문도 파일 part의 content도 한 값으로 보관하지 않는다: chunk 하나 외에 reader가 보관하는 것은
     최대 delimiter, part 하나의 헤더 줄(F-23.4), 값 한도 안의 텍스트 값, 그리고 읽은 part의 이름,
     filename, content type이다.
  2. F-24.2 delimiter는 `--`와 boundary이다(F-12.3).
  3. F-24.3 첫 delimiter는 본문을 시작하거나 CRLF 뒤에 오며, 그 앞의 바이트는 무시한다(I-1). 그 뒤의
     모든 delimiter는 part의 content를 끝내는 CRLF 뒤에 오며, 그 CRLF는 content가 아니다.
  4. F-24.4 delimiter 뒤에는 part의 헤더 줄을 시작하는 CRLF가 오거나 본문을 끝내는 `--`가 온다.
     그 `--` 뒤의 바이트는 무시한다(I-2).
  5. F-24.5 delimiter 뒤에 다른 바이트가 있는 본문, 본문을 끝내는 `--` 전에 끝나는 본문, 그리고
     헤더 줄에서 빈 줄 전에 delimiter로 시작하는 줄이 있는 part는 `invalid`로 실패한다.
  6. F-24.6 결과는 chunk에 의존하지 않는다: 어떤 chunk 분할로 읽은 본문이든 chunk 하나로 읽은 본문의
     값, 파일, 실패를 준다(F-8).
- **F-25** 본문의 한도(`BodyLimits`). 한도를 넘으면 본문은 그 kind와 상태 413으로 실패한다.
  1. F-25.1 본문 한도는 본문의 바이트를 제한한다(`body_limit`). 한도보다 큰 `Content-Length`는 바이트를
     읽기 전에 실패한다(F-9).
  2. F-25.2 값 한도는 reader가 한 값으로 보관하는 바이트를 제한한다: chunk로 읽지 않는 본문, 그리고
     chunk로 읽는 본문의 텍스트 값의 이름과 content를 합친 것이다(`value_limit`).
  3. F-25.3 part 한도는 chunk로 읽는 본문의 part 수를 제한하며 모든 part를 센다(`part_limit`).
  4. F-25.4 파일 한도는 chunk로 읽는 본문의 각 파일의 바이트를 제한한다(`file_limit`). 이 한도는
     파일 하나의 임시 저장소(F-10)를 제한하며 메모리를 제한하지 않는다. 파일은 잘리지 않고 일부만 넘겨지지도 않는다:
     한도 안의 파일은 통째로 넘겨진다.
  5. F-25.5 reader는 본문의 첫 실패에서 멈춘다(F-8, F-9).
  6. F-25.6 애플리케이션이나 시험이 바이트로 만든 요청의 본문처럼 서버가 한 값으로 보관하는 본문은
     `read_held_body(content type, body, limits)`가 F-25의 한도로 읽는다: 본문 한도와
     값 한도는 `RequestBody`가 보관하는 본문을 제한하듯이 그 바이트를 제한하고(바이트 하나에서는 본문 한도가
     먼저이다), part 한도, 파일 한도, 값 한도는 chunk로 읽는 본문을 제한하듯이 `multipart/form-data` 본문의
     part, 파일, 텍스트 값을 제한한다. 그 파일은 값이다. 서버는
     `multipart/form-data`가 자신의 form type 가운데 하나일 때 보관한 본문을 이렇게 읽는다(F-25는 본문의 규칙이며
     서버가 본문을 받은 방법의 규칙이 아니다).

- **F-8** 본문의 첫 실패는 그 바이트가 가장 먼저 오는 실패이다(F-24.6). 실패의 바이트는 그때까지 읽은
  바이트가 그 실패를 결정하게 되는 첫 바이트이다:

  | 실패 | 그 바이트 |
  |---|---|
  | `body_limit` | `limit`바이트 다음의 바이트 |
  | 한 값으로 보관한 본문의 `value_limit` | `valueLimit`바이트 다음의 바이트 |
  | `part_limit` | 한도를 넘은 part의 delimiter 뒤에 오는 CRLF의 LF |
  | 이름의 `value_limit` | part의 헤더 줄을 끝내는 빈 줄의 마지막 바이트 |
  | content의 `value_limit`, `file_limit` | 한도를 넘은 content 바이트 |
  | UTF-8이 아닌 값의 `invalid` | 그 바이트 뒤로 content가 UTF-8 텍스트의 시작이 아니게 되는 바이트, 또는 끝이 자른 sequence에서는 content의 끝 |
  | delimiter 뒤의 `invalid` | `--`와 CRLF의 `-`도 CR도 아닌 첫 바이트, 또는 이들을 완성하지 않는 둘째 바이트 |
  | 헤더 줄의 `invalid`(F-6, F-16, F-22.4, F-24.5) | 헤더 줄을 끝내는 빈 줄의 마지막 바이트 |
  | `header_limit` | 그 바이트 뒤로는 헤더 줄의 8192바이트 안에서 빈 줄이 시작할 수 없게 되는 바이트 |
  | 끝이 없는 본문의 `invalid` | 본문의 끝(`finish()`) |

  delimiter(`CRLF--`와 boundary)를 시작할 수 있는 content 바이트는 뒤의 바이트가 그렇지 않음을 보일 때까지
  보류된다. 그 바이트는 content이며, 그 뒤의 바이트부터 값의 한도와 UTF-8에 계산된다. 두 실패의
  바이트가 같으면 한도(413)가 바이트의 실패(400)보다 먼저이고, `body_limit`이 다른 한도보다 먼저이다. 본문 한도를
  넘는 chunk는 한도까지 읽으며, 그 바이트가 실패하지 않을 때에만 `body_limit`이 그 chunk의 실패이다.
  따라서 본문이 실패할 때에도 결과는 chunk에 의존하지 않는다.
- **F-9** `RequestBody::new(content type, Content-Length, stream, limits)`는 본문을 시작한다. `limits`는 본문 한도,
  값 한도, part 한도, 파일 한도의 record `BodyLimits`이다. 수가 아닌 `Content-Length`는
  `invalid_content_length`로 실패하고, 한도보다 큰 `Content-Length`는 바이트를 읽기 전에 `body_limit`으로 실패한다
  (F-25.1). 64비트가 담는 것보다 자릿수가 많은 수는 모든 한도보다 크다. chunk로 읽는 본문(`stream`과 type
  `multipart/form-data`)은 boundary가 필요하고(F-6, F-7) 디렉터리를 받는다. `feed(chunk)`는 다음 chunk를 읽는다. 그
  첫 실패(F-8)는 그 호출과 그 뒤 모든 호출의 오류이며, 본문은 그 뒤로 바이트도 열린
  파일도 갖지 않는다(F-25.5). host는 마지막 chunk 뒤에 `finish()`를 호출한다: chunk로 읽는 본문이 끝나지 않았으면
  `invalid`로 실패한다(F-24.5). 두 번째 `finish()`는 첫 번째의 결과를 주고, 그 뒤의 chunk는 `finished`로 실패한다.
  `parts()`는 chunk로 읽는 본문의 field와 파일을 준다. `parts()`는 한 값으로 보관한 본문에서는 `not_streamed`로,
  실패한 본문에서는 그 첫 실패로, `finish()` 전에는 `not_finished`로 실패한다. 한 값으로 보관한 본문은
  그 바이트를 `held()`로 주며, host는 그것을 `read_body`, `fields_from_body`, `files_from_body`로 읽는다.
- **F-16** `multipart/form-data` 본문의 모든 part는 type이 `form-data`이고 `name` 매개변수를 가진
  `Content-Disposition` 줄 하나를 가지고(RFC 7578, 4.2절), `Content-Type` 줄은 최대 하나이며, `:`가 없는 헤더 줄이 없고(RFC
  5322, 2.2절: header field는 이름, colon, body이다), `Content-Disposition`의 어떤 매개변수도
  두 번 주지 않는다. 매개변수 이름은 대소문자 구분 없이 비교한다(RFC 6266, 4.1절: 그런 값은 invalid이다).
  `Content-Disposition`이 없는 part, `Content-Disposition` 줄이나 `Content-Type` 줄이 둘인 part(어느 줄을 쓸지 정하는 규칙이 없다),
  `:`가 없는 줄을 가진 part, 다른 disposition type의 part, `name`이 없는 part, 매개변수를 두 번 준 part는 본문을
  헤더 줄의 바이트(F-8)에서 `invalid`로 실패시킨다. 이것은 무시하는 입력이 아니다: 무시하면
  client가 보낸 part를 버렸다(plan P-4). 이렇게 실패하는 part에는 이름이 없으므로 값 한도에 계산되지 않는다.
- **F-13** 파일 쓰기처럼 `MultipartReader`의 sink가 내는 실패는 그 호출의 실패이다.
  reader는 그 뒤로 아무것도 읽지 않고 그 뒤의 모든 호출은 `SinkFailed`를 준다. 이것은 본문의 실패가 아니다(H-6).

## 파일

- **F-10** 요청의 디렉터리는 `tempfile`이 시스템의 임시 디렉터리 아래에 이름
  `http-form-`과 무작위 문자로 만들고, 이름이 이미 있으면 `tempfile`이 문자를 다시 뽑으며, mode는 `0700`이다.
  파일 part의 content는 도착하는 대로 그 디렉터리의 파일에 쓴다. 그 파일의 mode는 `0600`이고
  `exec`에서 닫힌다. 각 파일은 본문 안에서 그 파일 part의 번호를 이름으로 하는 새 파일로 만든다. 코어 자신은
  한 이름을 두 번 만들지 않지만, 그 디렉터리는 같은 사용자의 다른 프로세스가 쓸 수 있는 파일 시스템에 있으므로
  이미 있는 경로는 환경의 조건이며 코어의 깨진 불변식이 아니다: 그 chunk는
  파일 시스템의 실패인 `errno` `EEXIST`를 가진 `io`로 실패하고, 실패한 호출은 이미 있는 그 파일을 열지도 쓰지도
  삭제하지도 않는다. `RequestBody::remove()`는 디렉터리를 이동하지 않은 파일과 함께 삭제한다. 디렉터리는
  그 요청에만 속하므로 사라진 디렉터리는 `not_found`로 실패하고, 삭제한 뒤의 두 번째 호출은
  아무것도 하지 않는다. 삭제한 뒤에는 그 본문의 저장된 모든 파일이, `parts()`가 준 모든 `UploadedFile`을 통해,
  `removed`로 실패하고 chunk도 그렇게 실패한다. `remove()` 없이 drop된 본문은 자신의 디렉터리를 삭제한다. 그
  삭제의 실패는 어떤 호출자에게도 닿지 않으며, 바인딩은 `remove()`가 호출되지 않았음을 경고한다(plan B-10).
- **F-11** `UploadedFile::move_to(path)`는 `path`의 파일을 대체하지 않고 파일을 이동하며, rename으로 하거나 다른
  파일 시스템으로는 복사와 삭제로 한다. 이동한 뒤에 `move_to`는 `moved`로 실패하고 `content`도 그렇게 실패하며,
  `size`는 그 파일의 바이트 수이다. 저장된 파일은 Linux에서는 `RENAME_NOREPLACE`를 준 `renameat2`로,
  macOS에서는 `RENAME_EXCL`을 준 `renameatx_np`로 rename한다(`rustix::fs::renameat_with`를 통해). `EEXIST`는 `exists`로 실패한다.
  다른 파일 시스템의 경로인 `EXDEV`만 다른 길을 간다. rename이 그 파일을 이동할 수 없음을 알리는 errno는
  그것뿐이기 때문이다(plan P-3): 복사는 `path`에 새 파일을 열고(`EEXIST`는 `exists`로 실패한다), 복사하고, 그 파일을
  디스크에 쓰고(`sync_all`), 원본을 삭제한다. 복사, 그 크기, 디스크 쓰기가 실패하면 복사는 자신이 만든
  파일을 삭제하고 그 실패를 준다. 그 삭제도 실패하면 오류는 두 실패를 모두 가진
  `partial_target`이다. `partial_target`의 `errno`는 실패한 복사, 크기 검사, 디스크 쓰기의 것으로, 그 오류의
  원인이며, 크기 검사가 실패했으면 없다(F-15). 실패한 삭제의 kind와 `errno`는 그 오류 class의
  property `removalKind`와 `removalErrno`이다. `source_left`의 `errno`는 실패한 원본
  삭제의 것이다. 원본의 삭제만 실패하면 그 파일은 이동한 것으로 치고 오류는
  `source_left`이다. 그 밖의 모든 errno는, flag가 없는 파일 시스템의 `EINVAL`과 `ENOSYS`를 포함하여,
  이동의 오류이다. 바이트가 값인 파일은 새 파일에 쓰고 디스크에 쓰며, 실패한 쓰기는 그 파일을
  마찬가지로 삭제한다. `UploadedFile`의 모든 clone과 `parts()`가 한 part에 대해 주는 모든 파일은 같은 파일이다:
  하나를 통한 이동은 모두의 이동이다(H-16).
- **F-15** `std`는 `close(2)`의 실패를 알리지 않으므로 코어는 대신 크기를 검사한다(H-22): 쓴
  파일을 닫기 전의 크기와 `content()`가 읽는 바이트 수는 쓴 바이트 수이다.
  다른 수는 `errno` 없는 `io`로 실패한다. 모든 바이트 수는 범위 검사와 함께 변환한다.
- **F-17** blocking I/O를 하는 public 함수는 의존성
  `polyspec-binding-core`의 wait token `binding_core::Io`를 첫 인자로 받는다(binding design I-2 (4), plan B-5 (a)): `RequestBody::new`(디렉터리를
  만든다), `feed`(파일을 쓴다), `finish`(실패한 본문의 파일을 닫는다), `remove`,
  `UploadedFile::content`, `UploadedFile::move_to`이다. token은 생성된 코드만 feature
  `construct`를 통해 만들며, 이 crate의 시험은 그 feature를 dev-dependency로 켠다. 다른 public 함수는 I/O를 하지 않는다. `RequestBody`의
  `Drop`은 host가 `remove`를 호출하지 않은 본문의 디렉터리를(F-10) token 없이 삭제한다. `Drop`은
  인자를 받지 않기 때문이다. 이것은 빠진 cleanup 호출의 안전망이며, 바인딩이 그 누락을 보고한다(plan B-10).
- **F-28** 조각으로 읽는 본문에는 WASM 바인딩이 없다: 그것은 시스템 임시 디렉터리 아래의 디렉터리에 파일을
  두고(F-10) 파일 I/O를 기다리는데(F-17), 브라우저의 WebAssembly는 그것을 할 수 없다(binding Q-4). `RequestBody`는
  PHP, Python, Node, Go의 클래스다. `UploadedFile`은 다섯 target의 클래스이고, WASM에서는 메모리의 형태만 가진다:
  WASM에는 파일 시스템이 없으므로 WASM의 파일은 바이트를 값으로 가지며, WASM의 함수인 `files_from_body`와
  `file_over`가 그런 파일을 주고 받는다. WASM에서 그 클래스는 `new`, property `name`, `filename`, `content_type`,
  `size`, 그리고 `UploadedFile::held_content`를 부르고 아무것도 기다리지 않는 `content`를 가진다. 경로를 받는
  `move_to`와, 저장된 파일을 읽는 다른 target의 `content`는 PHP, Python, Node, Go에만 있는 member다. 코어는 파일
  I/O를 Cargo feature `fs`에 두고, 기본 feature가 그것을 켠다: `RequestBody`, 요청의 디렉터리와 파일,
  `UploadedFile`의 저장됨·이동됨·삭제됨 상태와 `UploadedFile::move_to`, 그리고 `wasm32-unknown-unknown`으로 빌드되지
  않는 의존성 `rustix`와 `tempfile`이다. `fs` 없이 코어는 한 값으로 든 본문의 함수와, 바이트를 값으로 가진
  `UploadedFile`을 두고, `UploadedFile::held_content`를 가진다. 경로와 blocking I/O를 feature 하나가 담는다. 코어의
  blocking I/O는 모두 파일 I/O이기 때문이다. 계약은 WASM 바인딩을
  `core.features.wasm`의 feature로 빌드하고 그 목록은 비어 있으므로, 생성 WASM crate는 코어에
  `default-features = false`로 의존한다(binding GN-4a). 다른 모든 target은 기본 feature로 빌드한다.
- **F-14** 대체하지 않는 이동에는 `renameat2`나 `renameatx_np`가 필요하므로 feature `fs`를 켠 이 crate는 Linux와
  macOS에서만 빌드된다(plan T-2). 그 검사는 그 feature의 module에 있으므로, 그 feature가 없는 코어는
  `wasm32-unknown-unknown` 같은 다른 target으로 빌드된다(F-28).

## 실패 주입점

코어는 아래 표의 주입점마다 의존성 `polyspec-binding-core`의 macro `binding_core::fault::point!`를
부르고(`packages/rust/src/fault.rs`, 그 상수가 주입점의 이름이다), 계약은 같은 주입점을 `faults`에 선언한다(binding
G-5). macro는 주입점을 쓰는 이 crate의 feature `fault-test`로 주입점을 판정하며(binding GN-5), 그 feature는
`polyspec-binding-core`의 같은 feature를 켠다. 그 feature 없이는 주입점이 아무것도 하지 않는다. 바인딩 crate는 자기
feature `fault-test`로, 이 crate의 시험은 그 feature를 켠 이 crate에 대한 dev-dependency로 그것을 켜고, release
빌드에는 없다. 주입점을 arming하는 case는 호출 하나의 case이다(G-5): arming은 그 member의 호출에만 유효하다.

파일 시스템의 실패(variant `Io`, `SourceLeft`, `PartialTarget`)는 환경에서 오며, member의 어떤 인자도 그 실패에 닿지
않는다. 주입점은 코어 안의 시스템 호출 하나이다. kind `io`의 주입은 그 호출을 주어진 `errno`로 실패시킨다. 그 실패는
시스템의 실패처럼 오류 표에 따라 그 `errno`에서 kind를 얻으므로, `io` 주입은 그 행이 다른 결과를 적지 않는 한 variant
`Io`의 네 kind(`not_found`, `permission_denied`, `storage_full`, `io`)를 채운다. `errors.kinds`의 variant `Io` 행이
`kinds`에 그 넷을 적고, `io` 주입을 받는 주입점의 `fills`도 그 넷을 적는다. 주입점은 그 표의 kind만 채우기
때문이다(binding CK-3). kind `short`의 주입은 읽기, 쓰기, 복사가 주어진 수의 바이트만 처리하고 돌아오게 하며, 크기
검사가 그것을 `errno` 없는 `io`로 찾는다(F-15). `http_form.internal`에서 kind `broken`의 주입은 파일의 lock을 poison된
것으로 다룬다(H-8). arming한 주입점은 case가 시험하는 member의 호출 동안, 그 호출을 실행하는 thread에서 유효하다. 코어는
thread를 시작하지 않으므로 모든 주입점은 그 호출의 arming을 받는다.

| Id | 자리(함수) | 주입 | 채우는 kind | 조건의 규칙 | arming을 받는가 |
|---|---|---|---|---|---|
| `http_form.create_directory` | `RequestBody::new`의 `create_directory` | `io` | `Io`의 kind | F-10 | 예 |
| `http_form.create_file` | 파일 part의 새 파일, `feed`의 `Parts::take` | `io` | `Io`의 kind | F-10 | 예 |
| `http_form.write_file` | 파일 part의 content 쓰기, `feed`의 `Parts::take` | `io`, `short` | `Io`의 kind | F-10, F-15 | 예 |
| `http_form.file_size` | 쓴 파일을 닫기 전의 크기, `feed`의 `Parts::take` | `io` | `Io`의 kind | F-15 | 예 |
| `http_form.remove_directory` | `RequestBody::remove`의 `remove_dir_all` | `io` | `Io`의 kind | F-10 | 예 |
| `http_form.read` | 저장된 파일의 열기와 읽기, `UploadedFile::content`의 `read_stored` | `io`, `short` | `Io`의 kind | F-11, F-15 | 예 |
| `http_form.rename` | `move_to`의 `Ops::rename` | `io` | `Io`의 kind. 다만 `EXDEV`는 복사로 가며 그 자체로는 아무것도 채우지 않는다; `EEXIST`는 `exists`를 준다 | F-11 | 예 |
| `http_form.open` | 복사의 원본의 `Ops::open` | `io` | `Io`의 kind | F-11 | 예 |
| `http_form.create` | 복사의 대상이나 값의 대상의 `Ops::create` | `io` | `Io`의 kind; `EEXIST`는 `exists`를 준다 | F-11 | 예 |
| `http_form.copy` | `Ops::copy` | `io`, `short` | `Io`의 kind; `http_form.remove`와 함께 `partial_target` | F-11, F-15 | 예 |
| `http_form.write` | 값의 `Ops::write` | `io` | `Io`의 kind; `http_form.remove`와 함께 `partial_target` | F-11 | 예 |
| `http_form.sync` | 대상의 `Ops::sync` | `io` | `Io`의 kind; `http_form.remove`와 함께 `partial_target` | F-11 | 예 |
| `http_form.remove` | `Ops::remove`: 실패한 복사, 쓰기, sync 뒤에는 대상의 삭제이고, 그 밖에는 복사의 원본의 삭제 | `io` | 대상에는 `partial_target`, 원본에는 `source_left` | F-11 | 예 |
| `http_form.internal` | `content`와 `move_to`의 `UploadedFile` lock: `broken`으로 arming하면 lock이 poison된다 | `broken` | `internal` | X-1, H-8 | 예 |

`partial_target`은 한 case에 주입 셋이 필요하다. 복사에는 rename의 `EXDEV` 뒤에만 닿고, 그 kind에는 대상의 삭제가 실패한
뒤에만 닿기 때문이다: `EXDEV`를 준 `http_form.rename`, 복사의 실패를 준 `http_form.copy`(또는 `http_form.sync`), 삭제의
실패를 준 `http_form.remove`이다. 그 `errno`는 복사의 것이고 `removalErrno`는 삭제의 것이다(F-11).
`http_form.create_file`의 `EEXIST`는 `io`를 주고(F-10), `http_form.rename`과 `http_form.create`의 `EEXIST`는 `exists`를
주며, `exists`에는 입력도 닿는다: 이미 있는 대상 경로이다.

`finish`와 `parts`는 실패할 수 있는 시스템 호출을 하지 않는다: 본문의 첫 실패를 다시 주므로(F-9),
그 둘의 파일 시스템 실패는 앞선 `feed`가 만난 실패이다.

주입점을 arming하는 계약의 case는 호출 하나의 group에 있다: `request_body.new`(`create_directory`),
`request_body.feed`(`create_file`, `io`와 `short`의 `write_file`, `file_size`), `request_body.remove`
(`remove_directory`), `uploaded_file.move_to`(값인 파일의 `create`, `write`, `sync`), `uploaded_file.content`
(`internal`, X-1의 case 하나), `uploaded_file.content.stored`(`read`), `uploaded_file.move_to.stored`(`EXDEV`의
`rename`, `copy`, `remove`). 호출 하나의 case는 그 unit의 생성자로 case의 키에서 receiver를 만들거나(binding G-5,
T-5), 앞선 호출이 만드는 instance 값으로 키 `receiver` 아래에 준다(binding GN-13f). 그런 값은 `request_body.new`로
만들고 `then`의 호출을 준 본문이거나(그 호출 하나는 자기가 이름하는 kind로 실패할 수 있다), 그런 본문의 `parts`에서
`pick`의 단계가 닿는 저장된 파일이다. `contracts/forms.json`의 group `request_body.feed.made`,
`request_body.finish.made`, `request_body.parts.made`는 `feed`의 `finished`와 `removed`, `finish`의 `removed`, 그리고
`finish`와 `parts`가 다시 주는 chunk의 실패(F-9)를 준다. `contracts/fields.json`의 `uploaded_file.content.stored`와
`uploaded_file.move_to.stored`는 `content`의 `moved`와 `Io`의 kind 넷, `move_to`의 `moved`, `source_left`,
`partial_target`을 준다. case의 경로는 상대 경로이고, 코어의 시험은 그것을 case의 디렉터리 아래에서 받는다. 꼴 둘이
아직 없으므로 계약 검사는 다음 kind를 지적한다(CK-22). `finish`와 `parts`가 다시 주는 파일 시스템의 실패에는 주입점을
arming하는 `then`의 호출이 필요하다. case의 arming은 그 case 자신의 호출에만 유효하기 때문이다. `content`와
`move_to`의 `removed`에는 파일을 고른 뒤 본문을 지우는 호출, 곧 `then`이 부르는 instance가 아닌 다른 instance의
호출이 필요하다. 코어의 시험은 이 kind 하나하나에 닿는다: `tests/forms.rs`의 `bodies`는 chunk의 실패마다 그 뒤에
`finish`와 `parts`를 부르고, `tests/fields.rs`의 `streamed`와 `tests/forms.rs`의 `bodies`는 파일의 `removed`를 주며,
`tests/faults.rs`는 저장된 파일의 파일 시스템 실패를 준다.

## 무시하는 입력 (plan P-4)

| Id | 규칙 | 바이트 | 근거 | Case |
|---|---|---|---|---|
| I-1 | F-24.3 | 첫 delimiter 앞의 바이트 | RFC 2046, 5.1.1절: preamble은 무시해야 한다 | `fields.json` multipart `the_bytes_before_the_first_delimiter_are_ignored` |
| I-2 | F-24.4 | 본문을 끝내는 `--` 뒤의 바이트 | RFC 2046, 5.1.1절: epilogue는 무시해야 한다 | `fields.json` multipart `the_bytes_after_the_closing_delimiter_are_ignored` |
| I-4 | F-22.5 | `Content-Disposition`의 매개변수 `filename*` | RFC 7578, 4.2절: `filename*`은 쓰면 안 되며, `filename` 매개변수만 파일의 이름을 정한다 | `forms.json` multipart `a_filename_star_is_ignored` |
| I-5 | F-23.1 | 이름이 `Content-Disposition`도 `Content-Type`도 아닌 헤더 줄 | RFC 7578, 4.8절: 다른 header field는 무시해야 한다 | `fields.json` multipart `a_part_header_of_8192_bytes_is_read`, `forms.json` multipart `other_header_lines_are_ignored` |
| I-6 | F-20.1 | 두 `&` 사이의 빈 조각 | WHATWG URL, application/x-www-form-urlencoded parsing: 빈 sequence는 건너뛴다 | `fields.json` urlencoded `empty_parts_are_skipped` |
| I-7 | F-12 | form type이 정의하지 않는 매개변수: `application/x-www-form-urlencoded`의 모든 매개변수, 그리고 `multipart/form-data`의 `boundary`가 아닌 매개변수 | WHATWG URL, 5절: 그 type에는 매개변수가 없고 그 parser는 UTF-8을 decode한다; RFC 7578, 8절: optional parameter는 "none" | `forms.json` multipart `the_charset_of_an_urlencoded_type_is_ignored`, `another_parameter_of_an_urlencoded_type_is_ignored`, `fields.json` multipart `a_boundary_inside_a_quoted_string_is_no_boundary` |

media type의 reader는 http-media-type의 표에 있는 입력을 무시한다(MT-1.3, MT-3.2, MT-3.4, MT-2.2):
값의 앞뒤와 `;` 앞뒤의 whitespace, 빈 매개변수, 따옴표 값의 따옴표, 이름의 대소문자이다.
이름을 가진 `form-data` part가 아닌 part인 I-3은 행이 아니다: 그런 part는 실패한다(F-16).

코드는 각 자리에 그 id를 표시한다.

## 서버가 정하는 것

다음은 이 라이브러리를 쓰는 서버가 정하는 것이며, 이 문서의 어떤 규칙도 이것을 정하지 않는다:

- F-25의 한도의 값과 그 기본값, 그리고 애플리케이션이 유효한 한도를 선언하는지의 검사;
- 애플리케이션의 form type, `multipart/form-data` 본문을 chunk로 읽는지(F-9의 `stream`), form type이 아닌 type의
  본문의 상태, 그리고 `malformed_content_type` 뒤에 오는 것(F-12);
- 본문 읽기를 둘러싼 요청 검사의 순서, 그리고 상태의 응답 텍스트;
- 요청의 API: request target의 raw query, F-20부터 F-22의 값을 돌려주는 method와 한 이름의
  마지막 값이나 파일을 돌려주는 method, 그리고 10진수 텍스트가 integer가 되는 방법;
- 요청의 디렉터리를 삭제하는 때(F-10), 그리고 그 host들이 본문의 chunk를 읽는 방법(F-3).

## API

| 코어 | 목적 |
|---|---|
| `parse_urlencoded(text)` | F-20 |
| `media_type(content type)` | F-12.1 |
| `is_urlencoded(content type)` | F-12.2 |
| `multipart_boundary(content type)` | F-7, F-12.3 |
| `read_body(content type, body)`, `fields_from_body`, `files_from_body` | F-21, F-22: 한 값으로 보관한 본문 |
| `read_held_body(content type, body, limits)` | F-25.6 |
| `parse_multipart(body, boundary)`, `parse_multipart_chunks(chunks, boundary)`, `Collector`: `take`, `collect`, `into_form` | F-24 |
| `MultipartReader::new(boundary, limits)`: `feed`, `feed_with`, `finish`, `closed`, `buffered`, `parts` | F-24, F-25, F-8, F-13 |
| `RequestBody`: `new`, `feed`, `finish`, `parts`, `held`, `size`, `streamed`, `directory`, `remove`; `BodyLimits` | F-9, F-10, F-17 |
| `Form`: `fields`, `files`; `Fields`, `Files`: 이름과 그 값의 list | F-20.3, F-22.6 |
| `file_over(files, limit)` | F-27 |
| `UploadedFile`: `from_value`, `name`, `filename`, `content_type`, `size`, `content`, `move_to` | F-22.2, F-11, F-17 |
| `Error`: `kind`, `status`, `errno`, `removal`, `removal_kind`, `removal_errno` | 오류 표, F-11 |
| `fault`: 실패 주입점의 상수 | 실패 주입점 |

## 인터페이스 계약

- **F-18** 인터페이스 계약은 `packages/rust/interface.json`이며(binding design I-1),
  `@polyspec/binding`의 schema를 따른다. 계약은 위 표의 member 가운데 host가 쓰는 것(함수, `Form`,
  `RequestBody`, `UploadedFile`), 오류 표의 범주의 오류 class `HttpFormValueError`, `HttpFormTypeError`, `HttpFormIoError`,
  `HttpFormStoreError`, `HttpFormBusyError`, `HttpFormInternalError`
  (design E-2), `partial_target`의 오류 class `PartialTargetError`, 그리고 이전 바인딩과의 차이를
  `changes`에, "실패 주입점" 절의 실패 주입점을 `faults`에 선언한다.
  `MultipartReader`, `parse_multipart`, `parse_multipart_chunks`는 Rust API로 남는다(F-3). `malformed_content_type`은 이
  라이브러리의 kind이며, 그런 `Content-Type`을 가진 요청이 받는 상태 400을 가진다. 그 오류는 http-media-type의
  실패를 가지므로(F-12) 계약은 그 라이브러리의 kind를 부르지 않고 그 property를 전달하지 않는다. `RequestBody`는
  `binding_core::Cleanup`을 구현하고(그 cleanup 호출은 `remove`이다) `Error`는 `binding_core::Kinds`를 구현한다.
  1. 코어 함수는 계약 type의 Rust type을 받고 돌려준다(binding design I-2 (1), T-1, T-2):
     결과는 owned 값(`String`, `Vec<u8>`, `PathBuf`, `Vec`)이고, `Fields`와 `Files`는 list
     `list<tuple<text, list<…>>>`이며, `Form`과 `BodyLimits`는 record이므로(design T-4) `file_over`는
     파일들의 함수이고 `Form`의 member가 아니다. 계약의 모든 member는 코어의 함수이며, `PartialTargetError`의 property
     `removalKind`와 `removalErrno`도 그렇다. 이 둘은 다른 kind에서 `internal`로 실패한다.

## 검증

- `make test`: 코어에서 도는 `contracts/fields.json`과 `contracts/forms.json`의 case. 모든 multipart case와
  모든 body case는 바이트 하나씩으로도, 모든 위치에서 나눈 두 chunk로도 읽으며, reader는 두
  chunk 사이에 최대 그 상한만큼 보관한다(F-24.1, F-24.6). 실패 주입점을 arming하는 case는 그것과 함께 돌며,
  arming한 실패는 모두 일어나야 한다. `tests/faults.rs`는 저장된 파일의 주입점을 arming한다. 무작위 한도를 준 무작위 본문은 모든 분할에서 바이트
  하나씩 읽은 본문의 결과를 준다(`HTTP_FORM_ROUNDS`, 기본값 3000). 다른 파일 시스템으로의 이동에는
  `HTTP_FORM_OTHER_FS`에 다른 파일 시스템의 디렉터리가 필요하며, 주어지지 않으면 시험이 실패한다.
  `scripts/other-fs.sh`가 `make test`에 그것을 준다: Linux에서는 `/dev/shm`, macOS에서는 실행 동안 붙이고 실행이
  끝나거나 실패하거나 멈출 때 떼는 APFS 디스크 이미지다(H-9). 이동의 각 단계의 실패는 일부러 실패하는
  연산으로 시험한다.
- `make clippy`: plan R-1의 lint와 `as_conversions`.
- `make wasm-core`: `fs` 없는 코어의 unit 시험을 이 기계에서 실행하고, `fs` 없이 `wasm32-unknown-unknown`으로
  빌드한 코어를 `fault-test`가 있을 때와 없을 때 clippy로 검사한다(F-28). `make check`는 `make fmt`, `make clippy`, `make test` 뒤에 그것을 실행한다.
- `make bindings-check BINDING=<cli>`: 인터페이스 계약은 `polyspec-http-media-type`의 계약을 의존성으로 하여
  `@polyspec/binding`의 계약 검사를 통과하고, 바인딩 package `packages/ts`,
  `packages/ts/wasm`, `packages/python`, `packages/php-ext`, `packages/go`는 `binding scaffold`가 쓰는 것과 같다(binding design L-2). `make clippy`,
  `make fmt`, `make test`는 코어 crate를 가리키고, `make lint-bindings`와 `make test-bindings`는 `make bindings` 뒤에
  PHP, Python, Node의 바인딩 crate를 검사한다. Go와 WASM 바인딩을 빌드하거나 시험하는 make target은 아직 없다.
- `make differential HYPER=<dir> PYTHONPATH=...`: 무작위 본문을 코어와 hyper의 Python reader로 읽는다. 이 시험은
  hyper의 public이 아닌 module을 읽으므로 hyper가 reader를 export할 때까지 그 결과는 참고일 뿐이다(plan
  H-25). F-6이나 F-7이 설명하는 차이는 따로 센다.
