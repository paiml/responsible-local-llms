//! Lesson 5.1: read the licence before you run the model, and check it in code.
//!
//! Provable contract: a model card with no licence is denied, a licence is allowed only if the table marks it permissive, and the recorded cards produce allow, review and deny verdicts.

fn main() {
    let summary =
        ch5_governance::licence::run(1_000, &mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: no licence, no model OK");
}
