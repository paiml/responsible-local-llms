//! Lesson 4.1: a local agent in Rust, with a typed tool interface.
//!
//! Provable contract: the model's well-formed tool call runs, and prose, an unknown tool, a wrong type, an extra field and a broken object are all refused with the workspace untouched.
//!
//! Offline by default (recorded model output); `RLL_LIVE=1` asks the runtime named by `RLL_RUNTIME`.

fn main() {
    let cfg = rll_client::Config::from_env().expect("bad RLL_* environment");
    let live = ch1_sovereign::live_from(std::env::var("RLL_LIVE").ok());
    let summary =
        ch4_agents::agent::run(&cfg, live, &mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: typed tool calls only OK");
}
