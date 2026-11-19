//! Lesson 4.3: excessive agency. An agent may do what its policy allows and nothing else:
//! an allow-list of tools, a path jail, and a dry-run mode that changes nothing.

use crate::tool::{ToolCall, Workspace, TOOL_NAMES};
use std::collections::BTreeSet;
use std::fmt;

/// What happened to one call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Executed(String),
    DryRun(String),
    /// The policy (or the parser) said no; nothing ran.
    Refused(String),
    /// The policy allowed it and the tool itself failed.
    Failed(String),
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Outcome::Executed(s) => write!(f, "executed: {s}"),
            Outcome::DryRun(s) => write!(f, "dry-run: {s}"),
            Outcome::Refused(s) => write!(f, "refused: {s}"),
            Outcome::Failed(s) => write!(f, "failed: {s}"),
        }
    }
}

/// A path is inside the jail when it is relative and never climbs out.
pub fn safe_path(path: &str) -> Result<(), String> {
    let bad = path.is_empty()
        || path.starts_with('/')
        || path.contains(['\0', '\\'])
        || path.split('/').any(|part| part == "..");
    if bad {
        Err(format!("path {path:?} leaves the sandbox"))
    } else {
        Ok(())
    }
}

/// What an agent may do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    allowed: BTreeSet<String>,
    jail: bool,
    dry_run: bool,
}

impl Policy {
    /// Every tool, no jail, no dry run: the agent of lesson 4.1, with no limits at all.
    pub fn open() -> Policy {
        Policy::only(&TOOL_NAMES)
    }

    /// Only these tools.
    pub fn only(tools: &[&str]) -> Policy {
        Policy {
            allowed: tools.iter().map(|t| t.to_string()).collect(),
            jail: false,
            dry_run: false,
        }
    }

    /// Paths must stay inside the workspace.
    pub fn jailed(mut self) -> Policy {
        self.jail = true;
        self
    }

    /// Allowed calls are described, not run.
    pub fn dry(mut self) -> Policy {
        self.dry_run = true;
        self
    }

    pub fn check(&self, call: &ToolCall) -> Result<(), String> {
        if !self.allowed.contains(call.name()) {
            return Err(format!("tool {} is not on the allow-list", call.name()));
        }
        match call.path() {
            Some(p) if self.jail => safe_path(p),
            _ => Ok(()),
        }
    }
}

/// A workspace behind a policy. `executed` lists every call that actually ran.
#[derive(Debug, Clone)]
pub struct Guarded {
    pub ws: Workspace,
    pub policy: Policy,
    pub executed: Vec<ToolCall>,
}

impl Guarded {
    pub fn new(ws: Workspace, policy: Policy) -> Guarded {
        Guarded {
            ws,
            policy,
            executed: Vec::new(),
        }
    }

    pub fn run(&mut self, call: &ToolCall) -> Outcome {
        if let Err(reason) = self.policy.check(call) {
            return Outcome::Refused(reason);
        }
        if self.policy.dry_run {
            return Outcome::DryRun(format!("would run {}", call.json()));
        }
        self.executed.push(call.clone());
        match self.ws.execute(call) {
            Ok(text) => Outcome::Executed(text),
            Err(text) => Outcome::Failed(text),
        }
    }
}

/// What the lesson measures.
pub struct Summary {
    pub tool_refused: bool,
    pub path_refused: bool,
    pub refusals_changed_nothing: bool,
    pub dry_run_described: bool,
    pub dry_run_changed_nothing: bool,
    pub write_landed: bool,
}

impl Summary {
    /// Off the allow-list and outside the jail are refused with nothing changed, dry-run
    /// describes a call without running it, and an allowed call lands exactly as written.
    pub fn contract_holds(&self) -> bool {
        self.tool_refused
            && self.path_refused
            && self.refusals_changed_nothing
            && self.dry_run_described
            && self.dry_run_changed_nothing
            && self.write_landed
    }
}

/// The lesson: one policy, four calls, each shown with what became of it.
pub fn demo(out: &mut impl std::io::Write) -> Summary {
    let seed = Workspace::new().with_file("notes.txt", "keep me");
    let policy = Policy::only(&["read_file", "write_file"]).jailed();
    let delete = ToolCall::DeleteFile {
        path: "notes.txt".into(),
    };
    let escape = ToolCall::ReadFile {
        path: "../../etc/passwd".into(),
    };
    let write = ToolCall::WriteFile {
        path: "notes.txt".into(),
        content: "changed".into(),
    };
    let mut g = Guarded::new(seed.clone(), policy.clone());
    let first = g.run(&delete);
    let second = g.run(&escape);
    let refusals_changed_nothing = g.ws == seed && g.executed.is_empty();
    let mut dry = Guarded::new(seed.clone(), policy.dry());
    let preview = dry.run(&write);
    let landed = g.run(&write);
    for (what, outcome) in [
        ("delete notes.txt", &first),
        ("read ../../etc/passwd", &second),
        ("write notes.txt (dry run)", &preview),
        ("write notes.txt", &landed),
    ] {
        let _ = writeln!(out, "{what:<26} -> {outcome}");
    }
    Summary {
        tool_refused: matches!(first, Outcome::Refused(_)),
        path_refused: matches!(second, Outcome::Refused(_)),
        refusals_changed_nothing,
        dry_run_described: matches!(preview, Outcome::DryRun(_)),
        dry_run_changed_nothing: dry.ws == seed && dry.executed.is_empty(),
        write_landed: matches!(landed, Outcome::Executed(_))
            && g.ws.files().get("notes.txt").map(String::as_str) == Some("changed")
            && g.executed.len() == 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(p: &str) -> ToolCall {
        ToolCall::ReadFile { path: p.into() }
    }

    fn del(p: &str) -> ToolCall {
        ToolCall::DeleteFile { path: p.into() }
    }

    #[test]
    fn paths_that_climb_or_go_absolute_are_outside_the_jail() {
        for bad in ["", "/etc/passwd", "../x", "a/../../x", "a\\b", "a\0b"] {
            assert!(safe_path(bad).is_err(), "{bad:?}");
        }
        for good in ["notes.txt", "dir/notes.txt", "a..b"] {
            assert!(safe_path(good).is_ok(), "{good:?}");
        }
    }

    #[test]
    fn the_allow_list_refuses_a_tool_and_the_jail_refuses_a_path() {
        let p = Policy::only(&["read_file"]).jailed();
        assert!(p.check(&read("a")).is_ok());
        assert!(p.check(&del("a")).unwrap_err().contains("allow-list"));
        assert!(p.check(&read("../a")).unwrap_err().contains("sandbox"));
        assert!(p.check(&ToolCall::Add { a: 1, b: 1 }).is_err());
        let unjailed = Policy::only(&["read_file"]);
        assert!(unjailed.check(&read("../a")).is_ok());
    }

    #[test]
    fn a_refusal_changes_nothing() {
        let ws = Workspace::new().with_file("a", "x");
        let mut g = Guarded::new(ws.clone(), Policy::only(&["read_file"]).jailed());
        let out = g.run(&del("a"));
        assert!(matches!(out, Outcome::Refused(_)));
        assert_eq!(g.ws, ws);
        assert!(g.executed.is_empty());
    }

    #[test]
    fn dry_run_describes_and_changes_nothing() {
        let ws = Workspace::new().with_file("a", "x");
        let mut g = Guarded::new(ws.clone(), Policy::open().dry());
        let out = g.run(&del("a"));
        assert_eq!(
            out.to_string(),
            r#"dry-run: would run {"tool":"delete_file","path":"a"}"#
        );
        assert_eq!(g.ws, ws);
        assert!(g.executed.is_empty());
    }

    #[test]
    fn an_allowed_call_runs_once_and_a_tool_failure_is_not_a_refusal() {
        let mut g = Guarded::new(Workspace::new().with_file("a", "x"), Policy::open());
        assert_eq!(g.run(&del("a")).to_string(), "executed: deleted a");
        assert_eq!(g.run(&del("a")).to_string(), "failed: no such file a");
        assert_eq!(g.executed.len(), 2);
        assert!(g.ws.files().is_empty());
        assert_eq!(Outcome::Refused("r".into()).to_string(), "refused: r");
    }

    #[test]
    fn the_lesson_holds_its_contract_and_each_clause_can_fail() {
        let mut buf = Vec::new();
        let s = demo(&mut buf);
        assert!(s.contract_holds());
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("-> refused: tool delete_file is not on the allow-list"));
        assert!(text.contains("-> executed: wrote 7 bytes to notes.txt"));
        type Break = fn(&mut Summary);
        let breaks: [Break; 6] = [
            |s| s.tool_refused = false,
            |s| s.path_refused = false,
            |s| s.refusals_changed_nothing = false,
            |s| s.dry_run_described = false,
            |s| s.dry_run_changed_nothing = false,
            |s| s.write_landed = false,
        ];
        for b in breaks {
            let mut s = demo(&mut Vec::new());
            b(&mut s);
            assert!(!s.contract_holds());
        }
    }
}
