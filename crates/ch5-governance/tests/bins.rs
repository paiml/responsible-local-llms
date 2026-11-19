//! Run each demo bin as a subprocess, offline, and check it prints its contract line.

use std::process::Command;

fn run(exe: &str, args: &[&str], envs: &[(&str, &str)]) -> (bool, String, String) {
    let mut c = Command::new(exe);
    c.args(args)
        .env_remove("RLL_LIVE")
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
fn lesson1_prints_its_contract_and_its_sources() {
    let (ok, out, _) = run(env!("CARGO_BIN_EXE_lesson1_licence_check"), &[], &[]);
    assert!(ok);
    assert!(out.contains("missing.md: Deny"));
    assert!(out.contains("[NOT VERIFIED] <https://spdx.org/licenses/MIT.html> accessed 2026-11-19"));
    assert!(out.contains("contract: no licence, no model OK"));
}

#[test]
fn lesson2_documents_the_sample_repo() {
    let (ok, out, _) = run(env!("CARGO_BIN_EXE_lesson2_ai_act_docs"), &[], &[]);
    assert!(ok);
    assert!(out.contains("## Risk tier\n\nLimited, set by: chatbot, synthetic_text."));
    assert!(out.contains("contract: the document claims no evidence it lacks OK"));
}

#[test]
fn lesson2_takes_a_repo_argument_and_refuses_a_repo_without_facts() {
    let sample = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/sample-repo");
    let (ok, out, _) = run(env!("CARGO_BIN_EXE_lesson2_ai_act_docs"), &[sample], &[]);
    assert!(ok && out.contains("contract:"));
    let (ok, _, err) = run(
        env!("CARGO_BIN_EXE_lesson2_ai_act_docs"),
        &["/nonexistent-repo"],
        &[],
    );
    assert!(!ok);
    assert!(err.contains("demo run failed"));
}

#[test]
fn lesson3_prints_its_contract_for_each_runtime() {
    for rt in ["apr", "ollama", "llamacpp"] {
        let (ok, out, _) = run(
            env!("CARGO_BIN_EXE_lesson3_sensitivity_router"),
            &[],
            &[("RLL_RUNTIME", rt)],
        );
        assert!(ok, "{rt}");
        assert!(out.contains("bodies sent to the hosted stand-in: 3"));
        assert!(out.contains("contract: sensitive prompts never leave the machine OK"));
    }
}

#[test]
fn lesson3_live_mode_without_a_server_fails_loudly() {
    let (ok, _, err) = run(
        env!("CARGO_BIN_EXE_lesson3_sensitivity_router"),
        &[],
        &[("RLL_LIVE", "1"), ("RLL_BASE_URL", "http://127.0.0.1:1")],
    );
    assert!(!ok);
    assert!(err.contains("demo run failed"));
}

#[test]
fn a_bad_runtime_name_stops_the_router_demo() {
    let (ok, _, err) = run(
        env!("CARGO_BIN_EXE_lesson3_sensitivity_router"),
        &[],
        &[("RLL_RUNTIME", "vllm")],
    );
    assert!(!ok);
    assert!(err.contains("bad RLL_* environment"));
}

#[test]
fn lesson4_checks_the_repos_own_checklist() {
    let (ok, out, _) = run(env!("CARGO_BIN_EXE_lesson4_checklist"), &[], &[]);
    assert!(ok);
    assert!(out.contains("RESPONSIBLE-AI.md: 0 problems"));
    assert!(out.contains("contract: a ticked box needs a file OK"));
}
