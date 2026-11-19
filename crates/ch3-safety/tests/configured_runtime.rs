//! The demo bins against whatever runtime the environment names.
//!
//! Offline (`cargo test`, no `RLL_*` set) they read recorded answers. The live suite runs this
//! same file with `RLL_LIVE=1` and `RLL_RUNTIME`, `RLL_BASE_URL`, `RLL_MODEL` set.

use std::process::Command;

fn holds(exe: &str, line: &str) {
    let o = Command::new(exe).output().expect("spawn demo bin");
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    println!("{exe}\n{out}{err}");
    assert!(o.status.success(), "{exe} failed:\n{err}");
    assert!(out.contains(line));
}

#[test]
fn lesson1_contract_holds_on_the_configured_runtime() {
    holds(
        env!("CARGO_BIN_EXE_lesson1_bias_probes"),
        "contract: swapped names get the same decision OK",
    );
}

#[test]
fn lesson2_contract_holds_on_the_configured_runtime() {
    holds(
        env!("CARGO_BIN_EXE_lesson2_prompt_injection"),
        "contract: injected documents are blocked OK",
    );
}
