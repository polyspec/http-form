# Every check of this repository is a target here. `make conformance` builds the three bindings and runs the cases of
# contracts/fields.json through each of them, then compares every runner with the expectations and with each other.

CARGO ?= cargo
PHP ?= php
PYTHON ?= python3
NODE ?= node
RESULTS := target/conformance

.PHONY: build test conformance differential clippy check

build:
	$(CARGO) build --release --locked
	cp target/release/libhttp_forms_python.so python/http_forms/_http_forms.abi3.so
	mkdir -p node/lib php/lib
	cp target/release/libhttp_forms_node.so node/lib/http-forms.linux-x64-gnu.node
	cp target/release/libhttp_forms_php.so php/lib/http_forms.so

test:
	$(CARGO) test --locked -p http-forms

clippy:
	$(CARGO) clippy --locked --all-targets -- -D warnings

conformance: build
	mkdir -p $(RESULTS)
	$(PHP) -d extension=$(CURDIR)/php/lib/http_forms.so conformance/php_runner.php contracts/fields.json > $(RESULTS)/php.json
	PYTHONPATH=python $(PYTHON) conformance/python_runner.py contracts/fields.json > $(RESULTS)/python.json
	$(NODE) conformance/node_runner.mjs contracts/fields.json $(CURDIR)/node/lib/http-forms.linux-x64-gnu.node > $(RESULTS)/node.json
	$(PYTHON) conformance/check.py contracts/fields.json php=$(RESULTS)/php.json python=$(RESULTS)/python.json node=$(RESULTS)/node.json

# REFERENCE is a directory that holds the package polyspec.hyper of hyper's Python server.
differential: build
	test -n "$(REFERENCE)" || (echo "make differential REFERENCE=<directory of polyspec/hyper/*.py>" && exit 2)
	PYTHONPATH=python $(PYTHON) conformance/differential.py $(REFERENCE) $${BODIES:-200000}

check: clippy test conformance
