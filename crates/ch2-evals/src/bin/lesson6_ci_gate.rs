//! Lesson 2.6: evals in CI.
//!
//! Provable contract: the eval suite clears the gate on every runtime (apr, Ollama, llama.cpp), and the same gate goes red for a model that got worse.
//!
//! Offline by default (recorded answers); `RLL_LIVE=1` asks the runtime named by `RLL_RUNTIME`.

fn main() {
    let cfg = rll_client::Config::from_env().expect("bad RLL_* environment");
    let live = ch1_sovereign::live_from(std::env::var("RLL_LIVE").ok());
    let summary =
        ch2_evals::ci_gate::run(&cfg, live, &mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: a gate that can fail OK");
}
