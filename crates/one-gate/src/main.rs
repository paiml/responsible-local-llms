//! `one-gate [ROOT]`: exit 1 and print one line per violation when a main-push job is not a
//! required pull request check. ROOT defaults to the current directory.

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let root = std::env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    let violations = one_gate::check(&root);
    for v in &violations {
        eprintln!("one-gate: FAIL: {v}");
    }
    if violations.is_empty() {
        println!("one-gate: PASS");
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
