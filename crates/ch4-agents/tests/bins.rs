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
fn lesson1_prints_its_contract_for_each_runtime() {
    for rt in ["apr", "ollama", "llamacpp"] {
        let (ok, out, _) = run(
            env!("CARGO_BIN_EXE_lesson1_typed_agent"),
            &[("RLL_RUNTIME", rt)],
        );
        assert!(ok, "{rt}");
        assert!(out.contains("contract: typed tool calls only OK"));
        assert!(out.contains("agent did  : executed: 42"));
    }
}

#[test]
fn lesson2_prints_its_contract() {
    let (ok, out, _) = run(env!("CARGO_BIN_EXE_lesson2_mcp_loopback"), &[]);
    assert!(ok);
    assert!(out.contains("contract: tools over loopback MCP OK"));
}

#[test]
fn lesson3_prints_its_contract() {
    let (ok, out, _) = run(env!("CARGO_BIN_EXE_lesson3_agency_limits"), &[]);
    assert!(ok);
    assert!(out.contains("contract: agency has limits OK"));
}

#[test]
fn lesson4_prints_its_contract_and_says_the_model_is_recorded() {
    let (ok, out, _) = run(env!("CARGO_BIN_EXE_lesson4_tool_abuse"), &[]);
    assert!(ok);
    assert!(out.contains("RECORDED, illustrative"));
    assert!(out.contains("contract: tool output is data OK"));
}

#[test]
fn lesson5_prints_its_contract() {
    let (ok, out, _) = run(env!("CARGO_BIN_EXE_lesson5_approval_audit"), &[]);
    assert!(ok);
    assert!(out.contains("\"decision\":\"denied\""));
    assert!(out.contains("contract: approved and audited OK"));
}

#[test]
fn a_bad_runtime_name_stops_the_model_lessons() {
    for exe in [
        env!("CARGO_BIN_EXE_lesson1_typed_agent"),
        env!("CARGO_BIN_EXE_lesson4_tool_abuse"),
    ] {
        let (ok, _, err) = run(exe, &[("RLL_RUNTIME", "vllm")]);
        assert!(!ok);
        assert!(err.contains("bad RLL_* environment"));
    }
}

#[test]
fn live_mode_without_a_server_fails_loudly() {
    for exe in [
        env!("CARGO_BIN_EXE_lesson1_typed_agent"),
        env!("CARGO_BIN_EXE_lesson4_tool_abuse"),
    ] {
        let (ok, _, err) = run(
            exe,
            &[("RLL_LIVE", "1"), ("RLL_BASE_URL", "http://127.0.0.1:1")],
        );
        assert!(!ok);
        assert!(err.contains("demo run failed"));
    }
}
