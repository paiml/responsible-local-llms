//! Lesson 3.3: secrets and PII out of prompts and logs.
//!
//! Provable contract: for every sample, detection finds exactly the expected kinds, redaction leaves exactly the expected text and a second redaction changes nothing, no redacted text or log line holds a detected value, and clean text passes through unchanged.
//!
//! Offline and pure text: the samples are the `illustrative` strings in evals/pii.jsonl.

fn main() {
    let summary = ch3_guard::pii_demo::run(&mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: PII is redacted exactly and never logged OK");
}
