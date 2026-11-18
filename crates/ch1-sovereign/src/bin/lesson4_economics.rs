//! Lesson 1.4: local economics.
//!
//! Provable contract: at the computed break-even volume local and hosted cost are equal, hosted is cheaper below it and local is cheaper above it.
//!
//! Offline by default (recorded fixtures); `RLL_LIVE=1` talks to real runtimes.

fn main() {
    let cfg = rll_client::Config::from_env().expect("bad RLL_* environment");
    let live = ch1_sovereign::live_from(std::env::var("RLL_LIVE").ok());
    let summary =
        ch1_sovereign::economics::run(&cfg, live, &mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: break-even is exact OK");
}
