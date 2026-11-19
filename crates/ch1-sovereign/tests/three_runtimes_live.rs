//! Lesson 1.2 against all three runtimes at once.
//!
//! Its own file, not `configured_runtime.rs`: that file runs once per runtime with one
//! `RLL_BASE_URL`, while this demo asks apr, Ollama and llama.cpp in a single run. Offline it
//! reads the recorded fixtures. The live suite runs it once, after all three servers are up,
//! with `RLL_BASE_URL_APR`, `RLL_BASE_URL_OLLAMA` and `RLL_BASE_URL_LLAMACPP` set. A runtime
//! that answers with an empty reply turns this test, and so the suite, red (issue #55).

use std::process::Command;

#[test]
fn lesson2_three_runtimes_agree_on_the_configured_runtimes() {
    let exe = env!("CARGO_BIN_EXE_lesson2_three_runtimes");
    let o = Command::new(exe).output().expect("spawn demo bin");
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    println!("{exe}\n{out}{err}");
    assert!(o.status.success(), "{exe} failed:\n{err}");
    assert!(out.contains("contract: three runtimes agree OK"));
}
