//! Lesson 5.4: a responsible-AI checklist that lives in the repo and is checked by code.
//!
//! Format, one item per line:
//!   `- [x] <id> | evidence: <path from the repo root>`   a done item must name a file
//!   `- [ ] <id>`                                          an open item
//! A box ticked without a file that exists is a failure, not a pass.

use std::path::Path;

/// The items every release must show, in the order the lesson reads them.
pub const REQUIRED: [&str; 5] = [
    "licence-check-in-code",
    "ai-act-doc-generator",
    "sensitive-routing-tested",
    "contracts-bound",
    "ci-gate",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: String,
    pub done: bool,
    pub evidence: Option<String>,
}

/// Parse the checklist lines; a line that starts like an item but is not one is an error.
pub fn parse(text: &str) -> Result<Vec<Item>, String> {
    let mut items = vec![];
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        let (done, rest) = if let Some(r) = line.strip_prefix("- [x] ") {
            (true, r)
        } else if let Some(r) = line.strip_prefix("- [ ] ") {
            (false, r)
        } else if line.starts_with("- [") {
            return Err(format!("line {}: not `- [x] id` or `- [ ] id`", n + 1));
        } else {
            continue;
        };
        let (id, evidence) = match rest.split_once('|') {
            Some((id, ev)) => (
                id.trim(),
                ev.trim()
                    .strip_prefix("evidence:")
                    .map(|p| p.trim().to_string()),
            ),
            None => (rest.trim(), None),
        };
        items.push(Item {
            id: id.to_string(),
            done,
            evidence,
        });
    }
    Ok(items)
}

/// Every reason the checklist does not pass; empty means it passes.
pub fn problems(items: &[Item], root: &Path) -> Vec<String> {
    let mut out = vec![];
    for req in REQUIRED {
        let found: Vec<&Item> = items.iter().filter(|i| i.id == req).collect();
        match found.as_slice() {
            [] => out.push(format!("{req}: missing from the checklist")),
            [i] if !i.done => out.push(format!("{req}: not done")),
            [i] => match &i.evidence {
                None => out.push(format!("{req}: ticked with no evidence path")),
                Some(p) if !root.join(p).exists() => {
                    out.push(format!("{req}: evidence `{p}` does not exist"))
                }
                Some(_) => {}
            },
            _ => out.push(format!("{req}: listed more than once")),
        }
    }
    for i in items.iter().filter(|i| !REQUIRED.contains(&i.id.as_str())) {
        out.push(format!("{}: not a required item", i.id));
    }
    out
}

#[derive(Debug)]
pub struct Summary {
    pub good: Vec<String>,
    pub bad: Vec<String>,
}

fn list(out: &mut dyn std::io::Write, ps: &[String]) {
    for p in ps {
        writeln!(out, "  {p}").ok();
    }
}

pub const BAD_FIXTURE: &str = include_str!("../fixtures/checklists/bad.md");

/// Check the repo's own `RESPONSIBLE-AI.md` (must pass) and the recorded bad one (must fail).
pub fn run(root: &Path, out: &mut dyn std::io::Write) -> Result<Summary, String> {
    let path = root.join("RESPONSIBLE-AI.md");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let good = problems(&parse(&text)?, root);
    let bad = problems(&parse(BAD_FIXTURE)?, root);
    writeln!(out, "RESPONSIBLE-AI.md: {} problems", good.len()).ok();
    list(out, &good);
    writeln!(out, "recorded bad checklist: {} problems", bad.len()).ok();
    list(out, &bad);
    Ok(Summary { good, bad })
}

impl Summary {
    /// The repo's checklist passes and the bad one fails for every kind of reason.
    pub fn contract_holds(&self) -> bool {
        let kinds = [
            "missing from the checklist",
            "not done",
            "ticked with no evidence path",
            "does not exist",
            "more than once",
            "not a required item",
        ];
        self.good.is_empty() && kinds.iter().all(|k| self.bad.iter().any(|p| p.contains(k)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo_root;

    fn item(id: &str, done: bool, ev: Option<&str>) -> Item {
        Item {
            id: id.into(),
            done,
            evidence: ev.map(String::from),
        }
    }

    #[test]
    fn parses_done_open_and_ignores_prose() {
        let items =
            parse("# t\nprose\n- [x] a | evidence: p/q.md\n- [ ] b\n- [x] c | note\n").unwrap();
        assert_eq!(items[0], item("a", true, Some("p/q.md")));
        assert_eq!(items[1], item("b", false, None));
        assert_eq!(items[2], item("c", true, None));
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn a_malformed_item_is_an_error_with_its_line() {
        assert_eq!(
            parse("ok\n- [X] a\n").unwrap_err(),
            "line 2: not `- [x] id` or `- [ ] id`"
        );
    }

    #[test]
    fn the_repos_own_checklist_passes() {
        assert_eq!(
            run(&repo_root(), &mut Vec::new()).unwrap().good,
            Vec::<String>::new()
        );
    }

    #[test]
    fn the_recorded_bad_checklist_fails_for_each_reason() {
        let mut out = Vec::new();
        let s = run(&repo_root(), &mut out).unwrap();
        assert!(s.contract_holds());
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("ai-act-doc-generator: ticked with no evidence path"));
        assert!(text.contains("sensitive-routing-tested: not done"));
        assert!(
            text.contains("contracts-bound: evidence `contracts/no-such-file.yaml` does not exist")
        );
        assert!(text.contains("ci-gate: listed more than once"));
        assert!(text.contains("ship-it-anyway: not a required item"));
        assert!(text.contains("licence-check-in-code: missing from the checklist"));
    }

    #[test]
    fn an_empty_checklist_misses_every_required_item() {
        let p = problems(&[], &repo_root());
        assert_eq!(p.len(), REQUIRED.len());
    }

    #[test]
    fn contract_fails_if_the_good_checklist_has_a_problem_or_the_bad_one_a_gap() {
        let mut s = run(&repo_root(), &mut Vec::new()).unwrap();
        let ok = s.contract_holds();
        s.good.push("x".into());
        assert!(ok && !s.contract_holds());
        let mut s = run(&repo_root(), &mut Vec::new()).unwrap();
        s.bad.retain(|p| !p.contains("not done"));
        assert!(!s.contract_holds());
    }

    #[test]
    fn a_missing_checklist_file_is_an_error() {
        let e = run(
            &std::env::temp_dir().join("ch5-no-such-root"),
            &mut Vec::new(),
        )
        .unwrap_err();
        assert!(e.contains("RESPONSIBLE-AI.md"));
    }

    #[test]
    fn a_malformed_repo_checklist_is_an_error() {
        let d = std::env::temp_dir().join("ch5-bad-checklist");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("RESPONSIBLE-AI.md"), "- [?] a\n").unwrap();
        assert!(run(&d, &mut Vec::new()).unwrap_err().contains("line 1"));
    }
}
