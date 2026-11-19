//! Lesson 4.2: tools over MCP, a local server and an agent on loopback.
//!
//! Provable contract: the MCP server listens on 127.0.0.1 only, lists exactly the typed tools, runs a call with the same answer as the local agent, and answers an unknown tool or garbage with a JSON-RPC error.
//!
//! No model and no network beyond 127.0.0.1: the model output is recorded.

fn main() {
    let summary = ch4_agents::mcp::run(&mut std::io::stdout()).expect("demo run failed");
    assert!(summary.contract_holds(), "contract violated");
    println!("contract: tools over loopback MCP OK");
}
