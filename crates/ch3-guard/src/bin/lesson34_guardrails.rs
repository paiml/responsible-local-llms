//! Lesson 3.4: guardrails as code.
//!
//! Provable contract: every guardrail rule has a case that trips it and a near miss that does not, every case gets the action written for it, a filter passes on exactly the redacted text, and no log line or passed-on text holds a listed secret.
//!
//! Offline and pure text: the samples are the `illustrative` strings in evals/guardrails.jsonl.

fn main() {
    let summary = ch3_guard::guard_demo::run(&mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: every guardrail trips on its case and not on its near miss OK");
}
