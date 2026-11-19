//! Chapter 4 demo logic (agents and tool use, safely). Each lesson is a module with a `run`
//! function; the demo bins in `src/bin/` call it, assert the contract and print the verdict.
//!
//! - `tool`: the typed tool interface and the in-memory workspace the tools act on.
//! - `agency` (4.3) and `agent` (4.1): the policy that limits a call, and the agent loop step.
//! - `mcp` (4.2): a minimal MCP-style server and client on loopback.
//! - `abuse` (4.4): injection through tool output, scored with the `rll-evals` runner.
//! - `approval` (4.5): human approval for destructive calls and a tamper-evident audit log.
//!
//! The workspace is in memory and the "sandbox" is a path jail plus an allow-list. It shows the
//! policy, it is not an operating-system sandbox.

pub mod abuse;
pub mod agency;
pub mod agent;
pub mod approval;
pub mod mcp;
pub mod tool;
