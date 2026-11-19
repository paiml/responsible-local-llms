//! Lesson 4.5: human approval and an audit trail. A destructive call waits for a yes; every
//! call, allowed or not, leaves exactly one line in a hash-chained log that shows an edit.

use crate::agency::{Guarded, Outcome, Policy};
use crate::tool::{ToolCall, Workspace};
use serde::Serialize;
use std::collections::VecDeque;
use std::io::Write;

/// Whoever says yes or no to a destructive call.
pub trait Approver {
    fn approve(&mut self, call: &ToolCall) -> bool;
}

/// A scripted human for tests and demos: answers in order, and no answer left means no.
pub struct Scripted {
    answers: VecDeque<bool>,
    pub asked: usize,
}

impl Scripted {
    pub fn new(answers: &[bool]) -> Scripted {
        Scripted {
            answers: answers.iter().copied().collect(),
            asked: 0,
        }
    }
}

impl Approver for Scripted {
    fn approve(&mut self, _call: &ToolCall) -> bool {
        self.asked += 1;
        self.answers.pop_front().unwrap_or(false)
    }
}

fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// One line of the audit log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    pub seq: usize,
    pub tool: String,
    pub args: String,
    /// `allowed`, `approved`, `denied` or `refused`.
    pub decision: String,
    pub outcome: String,
    pub prev: u64,
    pub hash: u64,
}

impl Entry {
    fn digest(&self) -> u64 {
        fnv(&format!(
            "{}|{}|{}|{}|{}|{}",
            self.prev, self.seq, self.tool, self.args, self.decision, self.outcome
        ))
    }
}

/// Each entry carries the hash of the one before it, so an edited or removed line breaks the
/// chain. (A chain shows tampering; keeping the log safe from a writer is a separate job.)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditLog {
    pub entries: Vec<Entry>,
}

impl AuditLog {
    pub fn record(&mut self, call: &ToolCall, decision: &str, outcome: &Outcome) {
        let mut e = Entry {
            seq: self.entries.len(),
            tool: call.name().into(),
            args: call.json(),
            decision: decision.into(),
            outcome: outcome.to_string(),
            prev: self.entries.last().map_or(0, |p| p.hash),
            hash: 0,
        };
        e.hash = e.digest();
        self.entries.push(e);
    }

    /// True when every entry has its position, links to the one before and hashes to its own digest.
    pub fn verify(&self) -> bool {
        let mut prev = 0;
        self.entries.iter().enumerate().all(|(i, e)| {
            let good = e.seq == i && e.prev == prev && e.hash == e.digest();
            prev = e.hash;
            good
        })
    }

    pub fn to_jsonl(&self) -> String {
        self.entries
            .iter()
            .map(|e| serde_json::to_string(e).expect("an Entry always serializes"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// The policy first, then a human for destructive calls, then the tool; always a log line.
pub struct Gate<A: Approver> {
    pub guarded: Guarded,
    pub approver: A,
    pub log: AuditLog,
}

impl<A: Approver> Gate<A> {
    pub fn new(ws: Workspace, policy: Policy, approver: A) -> Gate<A> {
        Gate {
            guarded: Guarded::new(ws, policy),
            approver,
            log: AuditLog::default(),
        }
    }

    pub fn handle(&mut self, call: &ToolCall) -> Outcome {
        let (decision, outcome) = match self.guarded.policy.check(call) {
            Err(reason) => ("refused", Outcome::Refused(reason)),
            Ok(()) if call.destructive() && !self.approver.approve(call) => {
                ("denied", Outcome::Refused("human approval denied".into()))
            }
            Ok(()) => {
                let d = if call.destructive() {
                    "approved"
                } else {
                    "allowed"
                };
                (d, self.guarded.run(call))
            }
        };
        self.log.record(call, decision, &outcome);
        outcome
    }
}

pub struct Summary {
    pub calls: usize,
    pub asked: usize,
    pub log: AuditLog,
    pub notes_survived: bool,
    pub report_written: bool,
    pub tampered_edit_caught: bool,
    pub tampered_removal_caught: bool,
}

impl Summary {
    /// One log line per call, the human was asked for exactly the two policy-allowed destructive
    /// calls, the denied one changed nothing, the approved one ran, the chain verifies, and an
    /// edit or a removal breaks it.
    pub fn contract_holds(&self) -> bool {
        self.log.entries.len() == self.calls
            && self.asked == 2
            && self.notes_survived
            && self.report_written
            && self.log.verify()
            && self.tampered_edit_caught
            && self.tampered_removal_caught
    }
}

pub fn run(out: &mut impl Write) -> Summary {
    let ws = Workspace::new().with_file("notes.txt", "keep me");
    let policy = Policy::open().jailed();
    let mut gate = Gate::new(ws, policy, Scripted::new(&[true, false]));
    let calls = [
        r#"{"tool":"add","a":17,"b":25}"#,
        r#"{"tool":"read_file","path":"notes.txt"}"#,
        r#"{"tool":"write_file","path":"report.txt","content":"Q3 summary"}"#,
        r#"{"tool":"delete_file","path":"notes.txt"}"#,
        r#"{"tool":"delete_file","path":"../../etc/passwd"}"#,
    ];
    for text in calls {
        let call = ToolCall::parse(text).expect("the demo calls parse");
        let outcome = gate.handle(&call);
        let _ = writeln!(out, "{:<12} -> {outcome}", call.name());
    }
    let _ = writeln!(out, "--- audit log ---\n{}", gate.log.to_jsonl());
    let mut edited = gate.log.clone();
    edited.entries[3].decision = "approved".into();
    let mut removed = gate.log.clone();
    removed.entries.remove(3);
    Summary {
        calls: calls.len(),
        asked: gate.approver.asked,
        notes_survived: gate.guarded.ws.files().contains_key("notes.txt"),
        report_written: gate.guarded.ws.files().contains_key("report.txt"),
        tampered_edit_caught: !edited.verify(),
        tampered_removal_caught: !removed.verify(),
        log: gate.log,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn del(p: &str) -> ToolCall {
        ToolCall::DeleteFile { path: p.into() }
    }

    #[test]
    fn the_lesson_holds_its_contract_and_prints_the_log() {
        let mut buf = Vec::new();
        let s = run(&mut buf);
        assert!(s.contract_holds());
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("delete_file  -> refused: human approval denied"));
        assert!(text.contains(r#""decision":"denied""#));
        let decisions: Vec<&str> = s.log.entries.iter().map(|e| e.decision.as_str()).collect();
        assert_eq!(
            decisions,
            ["allowed", "allowed", "approved", "denied", "refused"]
        );
    }

    #[test]
    fn each_contract_clause_can_fail() {
        type Break = fn(&mut Summary);
        let breaks: [Break; 7] = [
            |s| s.calls += 1,
            |s| s.asked = 3,
            |s| s.notes_survived = false,
            |s| s.report_written = false,
            |s| s.log.entries[0].outcome = "forged".into(),
            |s| s.tampered_edit_caught = false,
            |s| s.tampered_removal_caught = false,
        ];
        for b in breaks {
            let mut s = run(&mut Vec::new());
            b(&mut s);
            assert!(!s.contract_holds());
        }
    }

    #[test]
    fn a_denied_call_is_not_run_and_no_answer_means_no() {
        let ws = Workspace::new().with_file("a", "x");
        let mut gate = Gate::new(ws.clone(), Policy::open(), Scripted::new(&[]));
        let out = gate.handle(&del("a"));
        assert_eq!(out, Outcome::Refused("human approval denied".into()));
        assert_eq!(gate.guarded.ws, ws);
        assert_eq!(gate.approver.asked, 1);
        assert_eq!(gate.log.entries.len(), 1);
    }

    #[test]
    fn a_refused_call_never_asks_the_human() {
        let mut gate = Gate::new(
            Workspace::new(),
            Policy::only(&["read_file"]),
            Scripted::new(&[true]),
        );
        gate.handle(&del("a"));
        assert_eq!(gate.approver.asked, 0);
        assert_eq!(gate.log.entries[0].decision, "refused");
    }

    #[test]
    fn a_dry_run_is_logged_and_the_chain_detects_a_reordered_line() {
        let mut gate = Gate::new(
            Workspace::new(),
            Policy::open().dry(),
            Scripted::new(&[true]),
        );
        let out = gate.handle(&del("a"));
        assert!(matches!(out, Outcome::DryRun(_)));
        gate.handle(&ToolCall::Add { a: 1, b: 2 });
        assert!(gate.log.verify());
        gate.log.entries.swap(0, 1);
        assert!(!gate.log.verify());
        assert!(AuditLog::default().verify());
        assert_eq!(AuditLog::default().to_jsonl(), "");
    }
}
