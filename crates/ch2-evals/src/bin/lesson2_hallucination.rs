//! Lesson 2.2: catch hallucination.
//!
//! Provable contract: an answer passes only if it cites the passage that answers the question and quotes it verbatim with the fact in it, a question the passages cannot answer must be refused, and the checker catches every kind of bad answer (uncited, wrong source, fabricated quote, missing fact, invented answer, wrong refusal).
//!
//! Offline by default (illustrative answers in fixtures/hallucination.jsonl); `RLL_LIVE=1` asks the runtime named by `RLL_RUNTIME` and measures it.

fn main() {
    let cfg = rll_client::Config::from_env().expect("bad RLL_* environment");
    let live = ch1_sovereign::live_from(std::env::var("RLL_LIVE").ok());
    let summary =
        ch2_evals::hallucination::run(&cfg, live, &mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: cite it or fail OK");
}
