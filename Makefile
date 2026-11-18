SHELL := /bin/bash
.SUFFIXES:
.DELETE_ON_ERROR:

.PHONY: test coverage lint lint-rust lint-shell gate

# Offline: recorded fixtures only.
test:
	cargo test --workspace
	scripts/test-whisper-pipeline.sh

# 100% of demo and client lines, bins included (they run as subprocesses in tests/bins.rs).
coverage:
	cargo llvm-cov --workspace --fail-under-lines 100

lint: lint-rust lint-shell

lint-rust:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings

# CI has no shellcheck on the host and never installs one: it runs the pinned
# image instead (SHELLCHECK_DOCKER=1).
lint-shell:
ifeq ($(SHELLCHECK_DOCKER),1)
	docker run --rm -v "$(CURDIR):/mnt" -w /mnt koalaman/shellcheck-alpine:v0.10.0 sh -c 'shellcheck scripts/*.sh'
else
	shellcheck scripts/*.sh
endif

gate: lint test coverage
