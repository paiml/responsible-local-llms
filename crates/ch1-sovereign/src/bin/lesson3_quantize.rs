//! Lesson 1.3: size and quantize for your machine.
//!
//! Provable contract: file size strictly grows with bits per weight (q4_k_m < q8_0 < f16), and the measured tokens/s is finite and positive.
//!
//! Offline by default (recorded fixtures); `RLL_LIVE=1` talks to real runtimes.

fn main() {
    let cfg = rll_client::Config::from_env().expect("bad RLL_* environment");
    let live = ch1_sovereign::live_from(std::env::var("RLL_LIVE").ok());
    let summary =
        ch1_sovereign::quantize::run(&cfg, live, &mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: smaller quantization is smaller, speed is measured OK");
}
