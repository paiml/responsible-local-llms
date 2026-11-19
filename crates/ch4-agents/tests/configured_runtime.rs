//! The model-facing demo bins against whatever runtime the environment names.
//!
//! Offline (`cargo test`, no `RLL_*` set) they read recorded answers. The live job runs this
//! same file with `RLL_LIVE=1` and `RLL_RUNTIME`, `RLL_BASE_URL`, `RLL_MODEL` set.

use std::process::Command;

fn check(exe: &str, contract: &str) {
    let o = Command::new(exe).output().expect("spawn demo bin");
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    println!("{exe}\n{out}{err}");
    assert!(o.status.success(), "{exe} failed:\n{err}");
    assert!(out.contains(contract));
}

#[test]
fn lesson1_contract_holds_on_the_configured_runtime() {
    check(
        env!("CARGO_BIN_EXE_lesson1_typed_agent"),
        "contract: typed tool calls only OK",
    );
}

#[test]
fn lesson4_contract_holds_on_the_configured_runtime() {
    check(
        env!("CARGO_BIN_EXE_lesson4_tool_abuse"),
        "contract: tool output is data OK",
    );
}
