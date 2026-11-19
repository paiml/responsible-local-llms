//! Lesson 5.3: route by sensitivity; sensitive data stays local, the rest may go hosted.
//!
//! Provable contract: every prompt takes its expected route, no sensitive prompt appears in any body sent to the hosted side, and the hosted wire holds exactly the prompts routed hosted.
//!
//! Offline by default (recorded replies; the hosted side is a recorder with no network);
//! `RLL_LIVE=1` sends the local half to a real runtime.

fn main() {
    let cfg = rll_client::Config::from_env().expect("bad RLL_* environment");
    let live = ch5_governance::live_from(std::env::var("RLL_LIVE").ok());
    let summary =
        ch5_governance::router::run(&cfg, live, &mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: sensitive prompts never leave the machine OK");
}
