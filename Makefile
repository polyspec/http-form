# Every check of this repository is a target here.
#
# The bindings are generated from the interface contract (plan G-6); the hand-written bindings of an earlier API are in
# legacy/, which no target builds.

CARGO ?= cargo
# Extra arguments of every cargo build of a binding.
CARGOFLAGS ?=
# Python 3.14 or later, the requires-python of packages/python/pyproject.toml.
PYTHON ?= python3.14
NODE ?= node
NPM ?= npm
# The cli of a binding checkout (packages/ts/src/cli.ts), which make bindings and make bindings-check run.
BINDING ?=
# The runtimes and tools of the five targets (binding BD-27). PHP is the binary whose headers the extension is built
# against (PHP_CONFIG) and that loads it; ext-php-rs reads both as paths. MATURIN builds the wheel of packages/python.
# GO is the Go of the go directive of packages/go/go.mod and WASM_BINDGEN the CLI of the release of the wasm-bindgen
# crate of packages/ts/wasm/Cargo.toml (binding GN-15, GN-18); test-go and node-package compare both releases before
# they use them.
PHP ?= $(shell command -v php)
PHP_CONFIG ?= $(shell command -v php-config)
PHPUNIT ?= phpunit
MATURIN ?= maturin
GO ?= go
GOFMT ?= gofmt
WASM_BINDGEN ?= wasm-bindgen
export PHP PHP_CONFIG
export CARGO_TARGET_DIR ?= $(CURDIR)/target
VAR := $(CURDIR)/var
# The file name extension of a library that Cargo builds as a cdylib: dylib on macOS, so on Linux.
CDYLIB := $(if $(filter Darwin,$(shell uname -s)),dylib,so)

# The core crate is named, because the binding crates of the workspace need make bindings first (plan 9c, binding L-2).
CORE := polyspec-http-form
BINDING_CRATES := polyspec-http-form-node polyspec-http-form-python polyspec-http-form-php-ext

.PHONY: test clippy fmt wasm-core check differential bindings bindings-check lint-bindings test-bindings test-crates \
	test-python node-package test-node test-wasm test-php test-go

test:
	$(CARGO) test --locked --release -p $(CORE)

clippy:
	$(CARGO) clippy --locked -p $(CORE) --all-targets -- -D warnings

fmt:
	$(CARGO) fmt --check -p $(CORE)

# The core as the WASM binding builds it (binding GN-4a): for wasm32-unknown-unknown with the features of
# core.features.wasm of the contract, which hold no fs (spec F-28), and with fault-test, which a WASM crate turns on
# for a contract with faults (binding GN-5).
wasm-core:
	$(CARGO) clippy --locked -p $(CORE) --target wasm32-unknown-unknown --no-default-features -- -D warnings
	$(CARGO) clippy --locked -p $(CORE) --target wasm32-unknown-unknown --no-default-features --features fault-test \
		-- -D warnings

check: fmt clippy test wasm-core

# HYPER is the directory packages/server/python/src of hyper. The test reads a module of hyper that is not public, so
# its result is a reference only (docs/spec/forms.md, Verification); PYTHONPATH must hold the packages that hyper
# imports.
differential:
	test -n "$(HYPER)" || (echo "make differential HYPER=<hyper>/packages/server/python/src" && exit 2)
	$(PYTHON) conformance/differential.py $(HYPER) $${BODIES:-200000}

# bindings writes the generated folders of the binding packages, which Git ignores; bindings-check runs the contract
# check, which also compares the tracked binding packages with what binding scaffold writes (binding L-2, CK-19) and the
# polyspec-binding-core that Cargo.lock pins with the version of binding (CK-20).
bindings:
	@test -n "$(BINDING)" || { echo "bindings: set BINDING to the cli of binding"; exit 1; }
	$(NODE) $(BINDING) generate --root $(CURDIR)

bindings-check:
	@test -n "$(BINDING)" || { echo "bindings-check: set BINDING to the cli of binding"; exit 1; }
	$(NODE) $(BINDING) check $(CURDIR)/packages/rust/interface.json

# The lint of the binding crates, after make bindings writes their generated folders.
lint-bindings: bindings
	$(CARGO) clippy --locked $(foreach crate,$(BINDING_CRATES),-p $(crate)) --all-targets -- -D warnings

# Each of the five targets of the contract (binding BD-27) runs the tests that binding generates on the standard runner
# of its language: unittest, node --test (Node, and WASM through the subpath ./wasm of the same installed package),
# PHPUnit and go test. No target is left out: a target that the generator does not take fails with its cause.
test-bindings: lint-bindings test-crates test-python test-node test-wasm test-php test-go

# The Rust tests of the binding crates.
test-crates: bindings
	$(CARGO) test --locked $(foreach crate,$(BINDING_CRATES),-p $(crate))

# The wheel with the test features, installed into a virtual environment, where the generated tests (G-5) run.
test-python: bindings
	$(PYTHON) -m venv --clear $(VAR)/venv
	rm -rf $(VAR)/wheels
	cd packages/python && $(MATURIN) build --locked --features panic-test,fault-test \
		--interpreter $(VAR)/venv/bin/python --out $(VAR)/wheels $(CARGOFLAGS)
	$(VAR)/venv/bin/python -m pip install -q --no-index --force-reinstall $(VAR)/wheels/*.whl
	cd $(VAR) && venv/bin/python -m unittest discover -s $(CURDIR)/packages/python/generated/tests \
		-t $(CURDIR)/packages/python/generated/tests

# The package and the platform package of the addon, packed and installed into var/node. The addon is the npm package
# of this machine's platform, which the package names in optionalDependencies; the recipe packs it as a release does
# and installs both tarballs, without the other platforms' packages. The package also holds the WASM module and its
# glue, which wasm-bindgen writes into packages/ts/wasm/generated before the package is packed (binding GN-18, P-5).
PLATFORM := $(shell $(NODE) -p 'process.platform + "-" + process.arch + (process.platform === "linux" ? "-gnu" : "")')
NODE_ADDON := http-form.$(PLATFORM).node
WASM_BINDGEN_RELEASE := $(shell sed -n 's/^wasm-bindgen = "=\(.*\)"$$/\1/p' packages/ts/wasm/Cargo.toml)
node-package: bindings
	test "$$($(WASM_BINDGEN) --version)" = "wasm-bindgen $(WASM_BINDGEN_RELEASE)"
	$(CARGO) build $(CARGOFLAGS) -p $(CORE)-wasm --locked --target wasm32-unknown-unknown --features panic-test
	$(WASM_BINDGEN) --target web --out-dir packages/ts/wasm/generated --out-name http_form_wasm \
		$(CARGO_TARGET_DIR)/wasm32-unknown-unknown/debug/polyspec_http_form_wasm.wasm
	$(CARGO) build $(CARGOFLAGS) -p $(CORE)-node --locked --features panic-test,fault-test
	rm -rf $(VAR)/node $(VAR)/node-platform && mkdir -p $(VAR)/node $(VAR)/node-platform
	cp $(CARGO_TARGET_DIR)/debug/libpolyspec_http_form_node.$(CDYLIB) $(VAR)/node-platform/$(NODE_ADDON)
	$(NODE) -e 'const p = require("./packages/ts/package.json"); require("fs").writeFileSync("$(VAR)/node-platform/package.json", JSON.stringify({ name: p.name + "-$(PLATFORM)", version: p.version, os: [process.platform], cpu: [process.arch], main: "$(NODE_ADDON)", files: ["$(NODE_ADDON)"] }, null, 2) + "\n")'
	cd $(VAR)/node-platform && $(NPM) pack --pack-destination $(VAR)/node
	cd packages/ts && $(NPM) pack --pack-destination $(VAR)/node
	echo '{ "private": true }' > $(VAR)/node/package.json
	cd $(VAR)/node && $(NPM) install --offline --no-audit --no-fund --omit=optional ./*.tgz

# The generated tests of the Node binding on node --test, against the installed package.
test-node: node-package
	cp packages/ts/generated/tests/generated.test.mjs $(VAR)/node/
	cd $(VAR)/node && $(NODE) --test generated.test.mjs

# The generated tests of the WASM binding through the subpath ./wasm of the installed package (binding P-5);
# --expose-gc serves the generated tests that collect instances (binding GN-18a).
test-wasm: node-package
	cp packages/ts/wasm/generated/tests/generated.test.mjs $(VAR)/node/generated.wasm.test.mjs
	cd $(VAR)/node && $(NODE) --expose-gc --test generated.wasm.test.mjs

# The extension with the test features, built against PHP_CONFIG and loaded into PHP; the stub is linted, then the
# generated tests run on PHPUnit.
test-php: bindings
	$(CARGO) build $(CARGOFLAGS) -p $(CORE)-php-ext --locked --features panic-test,fault-test
	$(PHP) -l packages/php-ext/generated/polyspec_http_form.stub.php
	$(PHP) -d extension=$(CARGO_TARGET_DIR)/debug/libpolyspec_http_form.$(CDYLIB) $(PHPUNIT) packages/php-ext/generated/tests

# The static library with the test features (binding GN-13b), which Go links through CGO_LDFLAGS; gofmt and go vet,
# then the generated tests on go test. GOTOOLCHAIN=local: a go of another release than the go directive fails instead
# of fetching that release.
GO_RELEASE := $(shell sed -n 's/^go \(.*\)$$/\1/p' packages/go/go.mod)
GO_ENV := GOTOOLCHAIN=local CGO_LDFLAGS=-L$(CARGO_TARGET_DIR)/debug GOFLAGS=-tags=polyspec_panic_test,polyspec_codec_test
test-go: bindings
	test "$$($(GO_ENV) $(GO) env GOVERSION)" = "go$(GO_RELEASE)"
	$(CARGO) build $(CARGOFLAGS) -p $(CORE)-go --locked --features panic-test,codec-test
	test -z "$$(cd packages/go && $(GOFMT) -l .)"
	cd packages/go && $(GO_ENV) $(GO) vet ./...
	cd packages/go && $(GO_ENV) $(GO) test -count=1 -v ./...
