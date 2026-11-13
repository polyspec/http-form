<!-- doc-id: readme-contracts -->
# Contracts

[`fields.json`](fields.json) and [`forms.json`](forms.json) hold the cases of the rules of [docs/spec/forms.md](../docs/spec/forms.md). A group that `packages/rust/interface.json` links in its `cases` calls its member once per case in the case convention of binding (G-5); the groups `body` and `file_over_binding` of `forms.json` and `streamed` of `fields.json` are a rule corpus of scenarios of more than one call that only the Rust tests of the core read, and the interface contract does not link them.
