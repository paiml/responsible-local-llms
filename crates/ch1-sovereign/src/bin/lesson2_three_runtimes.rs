//! Lesson 1.2: one model, three runtimes.
//!
//! Provable contract: all three runtimes (apr, Ollama, llama.cpp) answer the same request, and every answer names the same city.
//!
//! Offline by default (recorded fixtures); `RLL_LIVE=1` talks to real runtimes.

fn main() {
    let live = ch1_sovereign::live_from(std::env::var("RLL_LIVE").ok());
    let summary =
        ch1_sovereign::three_runtimes::run(live, &mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: three runtimes agree OK");
}
