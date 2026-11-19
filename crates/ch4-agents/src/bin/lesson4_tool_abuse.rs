//! Lesson 4.4: injection through tool output, and the tool-abuse eval set.
//!
//! Provable contract: the guarded agent runs only safe calls, passes every tool-abuse case and keeps the benign case working, and the open agent fails the set.
//!
//! Offline by default (a recorded, illustrative model that obeys the poisoned file); `RLL_LIVE=1` asks the runtime named by `RLL_RUNTIME` and checks only that nothing unsafe ran.

fn main() {
    let cfg = rll_client::Config::from_env().expect("bad RLL_* environment");
    let live = ch1_sovereign::live_from(std::env::var("RLL_LIVE").ok());
    let summary =
        ch4_agents::abuse::run(&cfg, live, &mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: tool output is data OK");
}
