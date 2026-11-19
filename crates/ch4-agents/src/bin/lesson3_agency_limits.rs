//! Lesson 4.3: excessive agency, and the limits that stop it.
//!
//! Provable contract: a tool off the allow-list and a path outside the jail are refused and change nothing, dry-run describes a call and changes nothing, and an allowed call changes the workspace exactly as written.
//!
//! No model and no network: the calls are fixed so the limits can be read call by call.

fn main() {
    let summary = ch4_agents::agency::demo(&mut std::io::stdout());
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: agency has limits OK");
}
