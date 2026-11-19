//! Lesson 3.5: a red-team set in CI.
//!
//! Provable contract: the attack set is fit to gate on (known categories, a goal per attack, enough attacks per category), the same seeds repeat the same report, the gate refuses a vulnerable model and names the attacks that got through, and (offline) the safe model defends every attack on every seed.
//!
//! Offline by default (recorded, illustrative answers); `RLL_LIVE=1` asks the runtime named by `RLL_RUNTIME`.

fn main() {
    let cfg = rll_client::Config::from_env().expect("bad RLL_* environment");
    let live = ch1_sovereign::live_from(std::env::var("RLL_LIVE").ok());
    let summary =
        ch3_redteam::redteam::run(&cfg, live, &mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: red-team gate OK");
}
