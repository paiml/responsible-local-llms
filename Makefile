SHELL := /bin/bash
.SUFFIXES:
.DELETE_ON_ERROR:

.PHONY: test coverage lint lint-rust lint-shell pv live gate

# Offline: recorded fixtures only.
test:
	cargo test --workspace
	scripts/test-whisper-pipeline.sh

# 100% of demo and client lines, bins included (they run as subprocesses in tests/bins.rs).
coverage:
	cargo llvm-cov --workspace --show-missing-lines --fail-under-lines 100

lint: lint-rust lint-shell

lint-rust:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings

# CI checks the host has shellcheck 0.10.0 (the version the docker image pinned) and never installs one.
lint-shell:
ifeq ($(SHELLCHECK_DOCKER),1)
	docker run --rm -v "$(CURDIR):/mnt" -w /mnt koalaman/shellcheck-alpine:v0.10.0 sh -c 'shellcheck scripts/*.sh'
else
	shellcheck scripts/*.sh
endif

# Provable contracts and the pv ontology (pv = aprender-contracts-cli, on PATH; never installed here).
# `pv lint` prints PASS even when one gate fails, so each ontology gate's verdict is read on its own.
PV_GATES := sigma shapes
pv:
	scripts/demo-bins-jsonl.sh --check
	pv lint contracts --binding contracts/binding.yaml --crate-dir crates/ch1-sovereign
	@for g in $(PV_GATES); do \
	  if pv lint contracts --gate $$g --format json | grep -q '^  "verdict": "Pass"'; then \
	    echo "pv gate $$g: Pass"; else echo "pv gate $$g: FAIL"; exit 1; fi; \
	done
	pv extract contracts --check

# Live: real local runtimes (llama.cpp and Ollama from pinned images, apr if on PATH).
live:
	scripts/live-suite.sh $(RUNTIMES)

gate: lint test coverage pv
