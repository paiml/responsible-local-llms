//! Chapter 5 demo logic: governance as code. Each lesson is a module; the demo
//! bins in `src/bin/` call it, assert the contract and print the verdict.
//!
//! Nothing here is legal advice. Every legal or regulatory fact cites a source in
//! [`sources`] with its access date, and anything not confirmed is marked NOT VERIFIED.

pub mod aiact;
pub mod checklist;
pub mod licence;
pub mod router;
pub mod sources;

use std::path::PathBuf;

/// The workspace root, baked in at build time (the checklist bin reads files from it).
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `RLL_LIVE=1` switches the router demo from recorded replies to a real local runtime.
pub fn live_from(value: Option<String>) -> bool {
    value.as_deref() == Some("1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_flag_parses() {
        assert!(live_from(Some("1".into())));
        assert!(!live_from(Some("0".into())));
        assert!(!live_from(None));
    }

    #[test]
    fn repo_root_holds_the_workspace_manifest() {
        assert!(repo_root().join("Cargo.toml").is_file());
    }
}
