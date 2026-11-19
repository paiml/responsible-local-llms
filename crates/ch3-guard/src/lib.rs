//! Chapter 3, videos 3.3 and 3.4. Each lesson is a module with a `run` function;
//! the demo bins in `src/bin/` call it, assert the contract and print the verdict.
//!
//! Everything here is pure text in, text out: no runtime is needed, so the PR path
//! and the live path are the same code. The sample strings in `evals/pii.jsonl` and
//! `evals/guardrails.jsonl` are hand-written and labelled `illustrative`; they are
//! not model measurements.

pub mod guard;
pub mod guard_demo;
pub mod pii;
pub mod pii_demo;

/// Where a sample string came from. Only these two labels are accepted in an eval file.
pub(crate) fn valid_source(s: &str) -> bool {
    s == "illustrative" || s == "recorded"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_two_provenance_labels_are_valid() {
        assert!(valid_source("illustrative"));
        assert!(valid_source("recorded"));
        assert!(!valid_source("measured"));
        assert!(!valid_source(""));
    }
}
