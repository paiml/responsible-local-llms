//! Lesson 2.4: determinism and flakiness.
//!
//! Provable contract: the same seeds and temperature repeat the same report exactly, greedy decoding gives pass rates of 0 or 1 only, and (offline) sampling shows a pass rate strictly between 0 and 1.
//!
//! Offline by default (recorded answers); `RLL_LIVE=1` asks the runtime named by `RLL_RUNTIME`.

fn main() {
    let cfg = rll_client::Config::from_env().expect("bad RLL_* environment");
    let live = ch1_sovereign::live_from(std::env::var("RLL_LIVE").ok());
    let summary =
        ch2_evals::flakiness::run(&cfg, live, &mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: pass rates, not single runs OK");
}
