<!-- doc-id: readme-contracts -->
<!-- source-sha256: f38abfb1cd4a6ba48aa1ca9f6a38a04eefb5d10f07bc9aa0d359346bc878f458 -->
# 계약

[`fields.json`](fields.json)과 [`forms.json`](forms.json)은 [docs/spec/forms.md](../docs/spec/forms.md)의 규칙의 case를 담는다. `packages/rust/interface.json`이 `cases`에서 연결하는 group은 binding의 case 규약(G-5)에 따라 case마다 그 member를 한 번 호출한다. `forms.json`의 group `body`와 `file_over_binding`, `fields.json`의 group `streamed`는 호출이 둘 이상인 scenario의 규칙 corpus이며, 코어의 Rust 시험만 이 corpus를 읽고 인터페이스 계약은 이 group들을 연결하지 않는다.
