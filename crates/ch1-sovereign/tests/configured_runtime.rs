//! The demo bins against whatever runtime the environment names.
//!
//! Offline (`cargo test`, no `RLL_*` set) they read recorded fixtures. The live CI job
//! (`scripts/live-suite.sh`) runs this same file with `RLL_LIVE=1`, `RLL_RUNTIME`,
//! `RLL_BASE_URL` and `RLL_MODEL` pointing at a real local server, so one suite judges both.

use std::process::Command;

fn run_inheriting_env(exe: &str) -> String {
    let o = Command::new(exe).output().expect("spawn demo bin");
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    println!("{exe}\n{out}{err}");
    assert!(o.status.success(), "{exe} failed:\n{err}");
    out
}

#[test]
fn lesson3_contract_holds_on_the_configured_runtime() {
    let out = run_inheriting_env(env!("CARGO_BIN_EXE_lesson3_quantize"));
    assert!(out.contains("contract: smaller quantization is smaller, speed is measured OK"));
}

#[test]
fn lesson4_contract_holds_on_the_configured_runtime() {
    let out = run_inheriting_env(env!("CARGO_BIN_EXE_lesson4_economics"));
    assert!(out.contains("contract: break-even is exact OK"));
}
