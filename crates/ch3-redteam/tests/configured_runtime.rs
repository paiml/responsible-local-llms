//! The demo bins against whatever runtime the environment names.
//!
//! Offline (`cargo test`, no `RLL_*` set) they read recorded answers. The live suite runs this
//! same file with `RLL_LIVE=1` and `RLL_RUNTIME`, `RLL_BASE_URL`, `RLL_MODEL` set.

use std::process::Command;

#[test]
fn lesson4_contract_holds_on_the_configured_runtime() {
    let exe = env!("CARGO_BIN_EXE_lesson5_redteam");
    let o = Command::new(exe).output().expect("spawn demo bin");
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    println!("{exe}\n{out}{err}");
    assert!(o.status.success(), "{exe} failed:\n{err}");
    assert!(out.contains("contract: red-team gate OK"));
}
