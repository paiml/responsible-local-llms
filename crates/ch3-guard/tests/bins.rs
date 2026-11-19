//! Run each demo bin as a subprocess and check it prints its contract line.

use std::process::Command;

fn run(exe: &str) -> (bool, String) {
    let o = Command::new(exe).output().expect("spawn demo bin");
    (
        o.status.success(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
    )
}

#[test]
fn lesson33_prints_its_contract() {
    let (ok, out) = run(env!("CARGO_BIN_EXE_lesson33_pii_redaction"));
    assert!(ok);
    assert!(out.contains("[EMAIL]"));
    assert!(out.contains("contract: PII is redacted exactly and never logged OK"));
}

#[test]
fn lesson34_prints_its_contract() {
    let (ok, out) = run(env!("CARGO_BIN_EXE_lesson34_guardrails"));
    assert!(ok);
    assert!(out.contains("rule=input-injection"));
    assert!(out.contains("contract: every guardrail trips on its case and not on its near miss OK"));
}
