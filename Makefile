SHELL := /bin/bash
.SUFFIXES:
.DELETE_ON_ERROR:

.PHONY: test coverage lint lint-rust lint-shell pv hero hero-write live gate one-gate

# Offline: recorded fixtures only.
test:
	cargo test --workspace
	scripts/test-whisper-pipeline.sh
	scripts/test-live-suite-partial.sh

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
	scripts/check-evals-table.sh
	scripts/demo-bins-jsonl.sh --check
	pv lint contracts --binding contracts/binding.yaml --crate-dir crates/ch1-sovereign
	pv lint contracts --binding contracts/ch3-guard/binding.yaml --crate-dir crates/ch3-guard
	@for g in $(PV_GATES); do \
	  if pv lint contracts --gate $$g --format json | grep -q '^  "verdict": "Pass"'; then \
	    echo "pv gate $$g: Pass"; else echo "pv gate $$g: FAIL"; exit 1; fi; \
	done
	pv extract contracts --check

# Live: all three real runtimes at once (llama.cpp and Ollama from pinned images, apr on PATH);
# a run without all three fails, because lesson 1.2 needs them together.
live:
	scripts/live-suite.sh $(RUNTIMES)

gate: one-gate lint test coverage pv hero

# ONE GATE (operator ruling 2026-11-19): every job that runs on push to main is a required pull
# request check. crates/one-gate parses each workflow as YAML and decides by a fail-closed allow-list
# (.github/one-gate-allow.txt, rendered check names, .github/required-status-checks.txt); its
# tests (cargo test, coverage) hold the evasions. pmat comply has no hook for this today.
one-gate:
	cargo run --quiet -p one-gate -- .

# The README hero (ADS-DEMO-001 R-9): rll-hero redraws docs/hero.png from the repo's own offline demos;
# `hero` goes RED when the image is missing, stale or altered, its evidence/hero.jsonl row is stale, or
# the README does not open with it. After a change to any demo that feeds it: `make hero-write`.
hero:
	cargo run --quiet -p rll-hero -- check

hero-write:
	cargo run --quiet -p rll-hero -- write
