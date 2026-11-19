//! Lesson 2.1: a test set is a spec.
//!
//! Provable contract: every runtime's answer satisfies the case's properties (contains Paris, not Lyon, at most 20 words) while a wrong answer fails them, so the case judges meaning, not wording.
//!
//! Offline by default (recorded fixtures); `RLL_LIVE=1` talks to real runtimes.

fn main() {
    let live = ch1_sovereign::live_from(std::env::var("RLL_LIVE").ok());
    let summary =
        ch2_evals::test_set_spec::run(live, &mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: a test set is a spec OK");
}
