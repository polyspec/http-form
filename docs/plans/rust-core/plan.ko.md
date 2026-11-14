<!-- doc-id: plan-rust-core -->
<!-- source-sha256: 414e3d664970289e30acee5f8374d90670acb38cca9209e41cc6b035c21da3ad -->
# Rust 코어

[English](plan.md)

http-form은 [명세](../../spec/forms.ko.md)의 규칙으로 HTTP 요청의 form을 읽는다. `application/x-www-form-urlencoded`
본문의 field와 `multipart/form-data` 본문의 part와 저장된 파일을 각각 그 한도 안에서 읽는다. 모든 규칙은
`packages/rust`의 Rust crate `polyspec-http-form`에 한 번 쓰이고, `contracts/`의 case로 한 번 시험된다. PHP, Python,
Node, 브라우저(WASM), Go 패키지는 자기 코드를 갖지 않는다. binding이 인터페이스 계약 `packages/rust/interface.json`에서
각 패키지를 생성하고, binding scaffold가 그 바인딩 crate를 쓴다. 그래서 규칙에는 처음 것과 달라질 수 있는 두 번째
사본이 없다. 예전 API의 손으로 쓴 바인딩은 생성 바인딩이 시험을 통과할 때까지 `legacy/`에 남는다.

checklist는 각 단계가 앞 단계를 필요로 하는 순서를 따른다.

1. 명세와 그 case(T1.1), 그다음 그것을 통과하는 코어(T1.2), 파일을 메모리에 두는 WASM 빌드의 코어(T1.3), 저장된
   파일을 다른 파일 시스템으로 옮기는 것(T1.4).
2. binding의 다섯 target을 위한 코어의 인터페이스 계약과 kind마다 하나인 case(T1.5), 그리고 코어와 binding scaffold가
   쓰는 바인딩 crate를 담고 binding의 한 commit에 고정된 하나의 Cargo workspace(T1.6). 열 개의 kind는 binding의 두
   case 꼴을 기다린다. fault를 장전하는 `then`의 단계와, `pick` 뒤의 다른 instance의 호출이다.
3. 문서와 이 checklist의 검사(T1.7).
4. binding이 생성하는 바인딩. 각각 binding이 생성하는 시험과 contract case로 그 언어의 표준 runner에서 시험되며(T1.8),
   생성 바인딩을 거친 `legacy/conformance`의 runner(T1.9), 이 저장소에 대한 binding의 end-to-end 검사(T1.10), Go
   바인딩(T1.11)이 있다. binding은 그 검사가 계약을 거부하는 동안 어떤 target도 생성하지 않으므로 넷은 binding을
   기다린다.
5. checkout의 도구, GitHub의 검사, release(T1.12), 그다음 첫 release 0.0.1(T1.13). 첫 release는
   http-media-type의 release 0.0.1에 그 tag로 의존하고 바인딩(T1.8~T1.11)을 기다린다. 플랫폼별 바인딩 패키지는 그다음
   version에 온다(T1.19).
