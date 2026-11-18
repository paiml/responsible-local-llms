SHELL := /bin/bash
.SUFFIXES:
.DELETE_ON_ERROR:

.PHONY: test coverage lint gate

# Offline: recorded fixtures only.
test:
	cargo test --workspace
	scripts/test-whisper-pipeline.sh

# 100% of demo and client lines, bins included (they run as subprocesses in tests/bins.rs).
coverage:
	cargo llvm-cov --workspace --fail-under-lines 100

lint:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	shellcheck scripts/*.sh

gate: lint test coverage
