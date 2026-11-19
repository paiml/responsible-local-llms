//! Run each demo bin as a subprocess, offline, and check it prints its contract line.

use std::process::Command;

const LESSON1: &str = env!("CARGO_BIN_EXE_lesson1_bias_probes");
const LESSON2: &str = env!("CARGO_BIN_EXE_lesson2_prompt_injection");

fn run(exe: &str, envs: &[(&str, &str)]) -> (bool, String, String) {
    let mut c = Command::new(exe);
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
fn lesson1_prints_its_contract() {
    let (ok, out, _) = run(LESSON1, &[]);
    assert!(ok);
    assert!(out.contains("illustrative"));
    assert!(out.contains("contract: swapped names get the same decision OK"));
}

#[test]
fn lesson2_prints_its_contract() {
    let (ok, out, _) = run(LESSON2, &[]);
    assert!(ok);
    assert!(out.contains("illustrative"));
    assert!(out.contains("contract: injected documents are blocked OK"));
}

#[test]
fn a_bad_runtime_name_stops_each_lesson() {
    for exe in [LESSON1, LESSON2] {
        let (ok, _, err) = run(exe, &[("RLL_RUNTIME", "vllm")]);
        assert!(!ok);
        assert!(err.contains("bad RLL_* environment"));
    }
}

#[test]
fn live_mode_without_a_server_fails_loudly() {
    for exe in [LESSON1, LESSON2] {
        let (ok, _, err) = run(
            exe,
            &[("RLL_LIVE", "1"), ("RLL_BASE_URL", "http://127.0.0.1:1")],
        );
        assert!(!ok);
        assert!(err.contains("demo run failed"));
    }
}
