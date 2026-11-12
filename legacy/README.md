# legacy

The hand-written PHP, Python and Node bindings of an earlier API (`php/`, `python/`, `node/`) and the runners that
ran the cases through them (`conformance/`). They are not built, not tested and not released: the bindings are the
thin packages of `packages/` that bindgen writes from `packages/rust/interface.json`. They stay until the generated
bindings pass V-6 (2), as a reference of the earlier API, which the `changes` of the contract list. `conformance/fields.json` is the copy of `contracts/fields.json` in the form that the runners read, fixed when the cases took the case convention of binding (G-5); a runner takes it as its argument.
