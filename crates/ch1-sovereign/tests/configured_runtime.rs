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

#[test]
fn lesson3_quality_holds_on_the_configured_runtime() {
    let out = run_inheriting_env(env!("CARGO_BIN_EXE_lesson3_quality"));
    assert!(out.contains("contract: quality is measured per quantization level OK"));
}

/// Lesson 1.2 asks all three runtimes at once, so it reads `RLL_BASE_URL_APR`, `_OLLAMA` and
/// `_LLAMACPP`. The live suite sets them once all three are up; this test is what turns the
/// suite red when a runtime answers with an empty reply (issue #55).
#[test]
fn lesson2_three_runtimes_agree_on_the_configured_runtimes() {
    let out = run_inheriting_env(env!("CARGO_BIN_EXE_lesson2_three_runtimes"));
    assert!(out.contains("contract: three runtimes agree OK"));
}
