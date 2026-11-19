//! Lesson 1.3: quantization can cost answer quality.
//!
//! Provable contract: every quantization level is scored on the same non-empty case set from evals/quantization-quality.jsonl, and a case passes only when its answer holds every required property.
//!
//! Offline by default (recorded answers); `RLL_LIVE=1` asks the real runtime, and `RLL_QUANT` names the level it serves.

fn main() {
    let cfg = rll_client::Config::from_env().expect("bad RLL_* environment");
    let live = ch1_sovereign::live_from(std::env::var("RLL_LIVE").ok());
    let level = std::env::var("RLL_QUANT").unwrap_or_else(|_| "q4_k_m".to_string());
    let summary = ch1_sovereign::quality::run(&cfg, live, &level, &mut std::io::stdout())
        .expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: quality is measured per quantization level OK");
}
