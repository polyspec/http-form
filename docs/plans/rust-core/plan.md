<!-- doc-id: plan-rust-core -->
# Rust core

[한국어](plan.ko.md)

http-form reads the forms of HTTP requests by the rules of [the specification](../../spec/forms.md): the fields of a
body of `application/x-www-form-urlencoded` and the parts and stored files of a body of `multipart/form-data`, each
within its limits. Every rule is written once, in the Rust crate `polyspec-http-form` of `packages/rust`, and tested
once, by the cases of `contracts/`. The packages of PHP, Python, Node, the browser (WASM) and Go hold no code of their
own: binding generates each of them from the interface contract `packages/rust/interface.json`, and binding scaffold
writes their binding crates, so a rule never has a second copy that can differ from the first. The hand-written
bindings of an earlier API stay in `legacy/` until the generated bindings pass their tests.

The checklist follows the order in which each step needs the one before it:

1. The specification and its cases (T1.1), then the core that passes them (T1.2), the core of the WASM build, which
   holds the files in memory (T1.3), and the move of a stored file to another file system (T1.4).
2. The interface contract of the core for the five targets of binding, with a case for every kind (T1.5), and one
   Cargo workspace with the core and the binding crates that binding scaffold writes, pinned to one commit of binding
   (T1.6). Ten kinds wait for two case forms of binding: a step of `then` that arms faults and a call of another
   instance after a `pick`.
3. The check of the documents and of this checklist (T1.7).
4. The bindings that binding generates, each tested by the tests that binding generates and by the contract cases on
   the standard runner of its language (T1.8), the runners of `legacy/conformance` through the generated bindings
   (T1.9), the end-to-end check of binding on this repository (T1.10) and the Go binding (T1.11). They wait for
   binding, which generates no target while its check refuses the contract.
5. The tools of the checkout, the checks on GitHub and the release (T1.12), then the first release 0.0.1 (T1.13), which
   depends on the release 0.0.2 of http-media-type by its tag, waits for the bindings (T1.8 to T1.11, T1.23) and holds
   the packages of the bindings per platform, which binding 0.0.3 generates the release recipes of and the workflow
   `assets.yml` builds on the runner of each platform (T1.19).
