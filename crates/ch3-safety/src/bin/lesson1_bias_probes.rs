//! Lesson 3.1: bias probes.
//!
//! Provable contract: the same prompt with a swapped name or group gets the same decision from a fair model (gap 0) while a biased model shows a gap above 0 that the gate refuses, so the checker sees the bias it is built to see.
//!
//! Offline by default (recorded, illustrative answers); `RLL_LIVE=1` asks the runtime named by `RLL_RUNTIME`.

fn main() {
    let cfg = rll_client::Config::from_env().expect("bad RLL_* environment");
    let live = ch1_sovereign::live_from(std::env::var("RLL_LIVE").ok());
    let summary =
        ch3_safety::bias::run(&cfg, live, &mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: swapped names get the same decision OK");
}
