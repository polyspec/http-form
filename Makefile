# Every check of this repository is a target here.
#
# The bindings are generated from the interface contract (plan G-6); the hand-written bindings in php/, python/ and
# node/ are not workspace members and are not built, so the targets that ran the cases through them are gone until
# the generated bindings exist.

CARGO ?= cargo
PYTHON ?= python3

.PHONY: test clippy fmt check differential

test:
	$(CARGO) test --locked --release -p polyspec-http-forms

clippy:
	$(CARGO) clippy --locked --all-targets -- -D warnings

fmt:
	$(CARGO) fmt --check

check: fmt clippy test

# HYPER is the directory packages/server/python/src of hyper. The test reads a module of hyper that is not public, so
# its result is a reference only (docs/spec/forms.md, Verification); PYTHONPATH must hold the packages that hyper
# imports.
differential:
	test -n "$(HYPER)" || (echo "make differential HYPER=<hyper>/packages/server/python/src" && exit 2)
	$(PYTHON) conformance/differential.py $(HYPER) $${BODIES:-200000}
