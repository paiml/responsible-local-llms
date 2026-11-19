//! Lesson 5.2: the EU AI Act for developers, documentation generated from the repo.
//!
//! Provable contract: the generated document's risk tier is the highest tier of the declared uses, and every obligation row cites an evidence file that exists or says MISSING with a reason.
//!
//! Pass a repo directory as the first argument; the default is the recorded sample repo.

fn main() {
    let root = std::env::args().nth(1).map_or_else(
        || std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/sample-repo"),
        std::path::PathBuf::from,
    );
    let summary =
        ch5_governance::aiact::run(&root, &mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(&root), "contract violated");
    println!("contract: the document claims no evidence it lacks OK");
}
