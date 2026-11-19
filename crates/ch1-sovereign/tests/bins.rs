//! Run each demo bin as a subprocess, offline, and check it prints its contract line.

use std::process::Command;

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
fn lesson2_prints_its_contract() {
    let (ok, out, _) = run(env!("CARGO_BIN_EXE_lesson2_three_runtimes"), &[]);
    assert!(ok);
    assert!(out.contains("contract: three runtimes agree OK"));
}

#[test]
fn lesson3_prints_its_contract_for_each_runtime() {
    for rt in ["apr", "ollama", "llamacpp"] {
        let (ok, out, _) = run(
            env!("CARGO_BIN_EXE_lesson3_quantize"),
            &[("RLL_RUNTIME", rt)],
        );
        assert!(ok, "{rt}");
        assert!(out.contains("contract: smaller quantization is smaller, speed is measured OK"));
    }
}

#[test]
fn lesson4_prints_its_contract() {
    let (ok, out, _) = run(env!("CARGO_BIN_EXE_lesson4_economics"), &[]);
    assert!(ok);
    assert!(out.contains("contract: break-even is exact OK"));
}

#[test]
fn a_bad_runtime_name_stops_the_demo() {
    let (ok, _, err) = run(
        env!("CARGO_BIN_EXE_lesson3_quantize"),
        &[("RLL_RUNTIME", "vllm")],
    );
    assert!(!ok);
    assert!(err.contains("bad RLL_* environment"));
}

#[test]
fn live_mode_without_a_server_fails_loudly() {
    let (ok, _, err) = run(
        env!("CARGO_BIN_EXE_lesson4_economics"),
        &[("RLL_LIVE", "1"), ("RLL_BASE_URL", "http://127.0.0.1:1")],
    );
    assert!(!ok);
    assert!(err.contains("demo run failed"));
}

#[test]
fn lesson2_live_mode_without_a_server_fails_loudly() {
    let (ok, _, err) = run(
        env!("CARGO_BIN_EXE_lesson2_three_runtimes"),
        &[("RLL_LIVE", "1"), ("RLL_BASE_URL", "http://127.0.0.1:1")],
    );
    assert!(!ok);
    assert!(err.contains("demo run failed"));
}
