//! Lesson 2.3: structured output you can trust.
//!
//! Provable contract: an accepted reply always validates against the JSON schema, a refusal, prose or a wrong value is sent back with the reasons and retried within three attempts, and a case whose retries run out yields no value instead of a bad one.
//!
//! Offline by default (illustrative replies in fixtures/structured.jsonl); `RLL_LIVE=1` asks the runtime named by `RLL_RUNTIME`.

fn main() {
    let cfg = rll_client::Config::from_env().expect("bad RLL_* environment");
    let live = ch1_sovereign::live_from(std::env::var("RLL_LIVE").ok());
    let summary =
        ch2_evals::structured::run(&cfg, live, &mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: trust the reply or get no reply OK");
}
