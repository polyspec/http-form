# Every check of this repository is a target here.
#
# The bindings are generated from the interface contract (plan G-6); the hand-written bindings of an earlier API are in
# legacy/, which no target builds.

CARGO ?= cargo
PYTHON ?= python3
NODE ?= node
# The cli of a bindgen checkout (packages/ts/src/cli.ts), which make bindings and make bindings-check run.
BINDGEN ?=

# The core crate is named, because the binding crates of the workspace need make bindings first (plan 9c, bindgen L-2).
CORE := polyspec-http-form
BINDING_CRATES := polyspec-http-form-node polyspec-http-form-python polyspec-http-form-php-ext

.PHONY: test clippy fmt check differential bindings bindings-check lint-bindings test-bindings

test:
	$(CARGO) test --locked --release -p $(CORE)

clippy:
	$(CARGO) clippy --locked -p $(CORE) --all-targets -- -D warnings

fmt:
	$(CARGO) fmt --check -p $(CORE)

check: fmt clippy test

# HYPER is the directory packages/server/python/src of hyper. The test reads a module of hyper that is not public, so
# its result is a reference only (docs/spec/forms.md, Verification); PYTHONPATH must hold the packages that hyper
# imports.
differential:
	test -n "$(HYPER)" || (echo "make differential HYPER=<hyper>/packages/server/python/src" && exit 2)
	$(PYTHON) conformance/differential.py $(HYPER) $${BODIES:-200000}

# bindings writes the generated folders of packages/ts, packages/python and packages/php-ext, which Git ignores;
# bindings-check runs the contract check, which also compares the tracked binding packages with what bindgen scaffold
# writes (bindgen L-2, CK-19) and the polyspec-bindgen-core that Cargo.lock pins with the version of bindgen (CK-20).
bindings:
	@test -n "$(BINDGEN)" || { echo "bindings: set BINDGEN to the cli of bindgen"; exit 1; }
	$(NODE) $(BINDGEN) generate --root $(CURDIR)

bindings-check:
	@test -n "$(BINDGEN)" || { echo "bindings-check: set BINDGEN to the cli of bindgen"; exit 1; }
	$(NODE) $(BINDGEN) check $(CURDIR)/packages/rust/interface.json

# The lint and the tests of the binding crates, after make bindings writes their generated folders.
lint-bindings: bindings
	$(CARGO) clippy --locked $(foreach crate,$(BINDING_CRATES),-p $(crate)) --all-targets -- -D warnings

test-bindings: bindings
	$(CARGO) test --locked $(foreach crate,$(BINDING_CRATES),-p $(crate))
