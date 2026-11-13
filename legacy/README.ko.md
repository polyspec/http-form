<!-- doc-id: readme-legacy -->
<!-- source-sha256: e8cdd9f1e73cc20e93645795bd40b0c53662084546e0bc120f53b378ca0fa229 -->
# legacy

이전 API의 손으로 쓴 PHP, Python, Node 바인딩(`php/`, `python/`, `node/`)과 case를 그 바인딩으로 실행하던
runner(`conformance/`)이다. 이들은 빌드하지 않고, 시험하지 않고, release하지 않는다. 바인딩은
bindgen이 `packages/rust/interface.json`에서 쓰는 `packages/`의 얇은 package이다. 이들은 생성된 바인딩이
V-6 (2)를 통과할 때까지 이전 API의 참고 자료로 남으며, 계약의 `changes`가 이전 API와의 차이를 나열한다. `conformance/fields.json`은 `contracts/fields.json`을 runner가 읽는 형식으로 둔 사본이며, case가 binding의 case 규약(G-5)을 받아들인 때의 내용으로 고정되었다. runner는 이 파일을 인자로 받는다.
