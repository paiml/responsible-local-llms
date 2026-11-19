//! Lesson 2.5: Elo in Rust.
//!
//! Provable contract: head-to-head results on the same eval cases give Elo ratings that conserve the total points, come out strongest first in the fixture's order, and give two players expected scores that sum to 1; "good enough" is a threshold on the expected score against the incumbent.
//!
//! Offline only: the answers are illustrative (fixtures/elo.jsonl). The same `matches_from` ranks real models when given real responders.

fn main() {
    let summary = ch2_evals::elo::run(&mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: rank by head to head OK");
}
