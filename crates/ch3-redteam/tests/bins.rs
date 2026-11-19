//! Run the demo bin as a subprocess, offline, and check it prints its contract line.

use std::process::Command;

fn run(envs: &[(&str, &str)]) -> (bool, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_lesson5_redteam"));
    c.env_remove("RLL_LIVE")
        .env_remove("RLL_RUNTIME")
        .env_remove("RLL_BASE_URL")
        .env_remove("RLL_MODEL");
    for (k, v) in envs {
        c.env(k, v);
    }
    let o = c.output().expect("spawn demo bin");
    (
        o.status.success(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

#[test]
fn lesson5_prints_its_contract_for_each_runtime() {
    for rt in ["apr", "ollama", "llamacpp"] {
        let (ok, out, _) = run(&[("RLL_RUNTIME", rt)]);
        assert!(ok, "{rt}");
        assert!(out.contains("recorded, illustrative"));
        assert!(out.contains("contract: red-team gate OK"));
    }
}

#[test]
fn a_bad_runtime_name_stops_lesson5() {
    let (ok, _, err) = run(&[("RLL_RUNTIME", "vllm")]);
    assert!(!ok);
    assert!(err.contains("bad RLL_* environment"));
}

#[test]
fn lesson5_live_mode_without_a_server_fails_loudly() {
    let (ok, _, err) = run(&[("RLL_LIVE", "1"), ("RLL_BASE_URL", "http://127.0.0.1:1")]);
    assert!(!ok);
    assert!(err.contains("demo run failed"));
}
