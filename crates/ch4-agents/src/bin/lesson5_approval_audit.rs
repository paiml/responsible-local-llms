//! Lesson 4.5: human approval and an audit trail.
//!
//! Provable contract: every call leaves exactly one audit line, a destructive call waits for a human yes and a denied one changes nothing, and the hash chain verifies until a line is edited or removed.
//!
//! No model and no network: the human is scripted (yes, then no).

fn main() {
    let summary = ch4_agents::approval::run(&mut std::io::stdout());
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: approved and audited OK");
}
