//! Lesson 3.2: prompt injection.
//!
//! Provable contract: every injected document is blocked before the model sees it and its defended answer passes the eval set, no clean control is blocked, and offline the undefended pipeline is shown obeying at least one attack.
//!
//! Offline by default (recorded, illustrative answers); `RLL_LIVE=1` asks the runtime named by `RLL_RUNTIME`.

fn main() {
    let cfg = rll_client::Config::from_env().expect("bad RLL_* environment");
    let live = ch1_sovereign::live_from(std::env::var("RLL_LIVE").ok());
    let summary =
        ch3_safety::injection::run(&cfg, live, &mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: injected documents are blocked OK");
}
