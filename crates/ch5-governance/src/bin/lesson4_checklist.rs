//! Lesson 5.4: a responsible-AI checklist in your repo, checked by code.
//!
//! Provable contract: the repo's RESPONSIBLE-AI.md has every required item ticked with an evidence file that exists, and the recorded bad checklist fails for each kind of defect.

fn main() {
    let summary =
        ch5_governance::checklist::run(&ch5_governance::repo_root(), &mut std::io::stdout())
            .expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: a ticked box needs a file OK");
}
