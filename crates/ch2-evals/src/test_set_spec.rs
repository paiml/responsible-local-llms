//! Lesson 2.1: a test set is a spec. A case holds a prompt and the properties its answer
//! must have, so a correct answer in different words passes and a wrong one fails.

use ch1_sovereign::client_for;
use rll_client::{ChatRequest, Error, Runtime};
use rll_evals::{parse_jsonl, Case};
use std::io::Write;

/// The eval set the lesson reads (`evals/spec.jsonl`).
pub const SPEC: &str = include_str!("../../../evals/spec.jsonl");
/// What a brittle exact-string test would demand.
pub const EXACT: &str = "Paris is the capital of France";
/// A wrong answer the properties must refuse.
pub const WRONG: &str = "Lyon is the capital of France.";

/// The lesson's one case.
pub fn case() -> Case {
    parse_jsonl(SPEC)
        .expect("evals/spec.jsonl is a valid eval set")
        .remove(0)
}

/// One runtime's answer, judged two ways.
pub struct Row {
    pub runtime: Runtime,
    pub answer: String,
    pub failures: Vec<String>,
    pub exact_match: bool,
}

pub struct Summary {
    pub rows: Vec<Row>,
    /// Failures the properties report for `WRONG`.
    pub wrong_failures: Vec<String>,
}

impl Summary {
    /// Every runtime's answer satisfies the case, and the wrong answer does not.
    pub fn contract_holds(&self) -> bool {
        self.rows.len() == Runtime::ALL.len()
            && self.rows.iter().all(|r| r.failures.is_empty())
            && !self.wrong_failures.is_empty()
    }
}

pub fn run(live: bool, out: &mut impl Write) -> Result<Summary, Error> {
    let case = case();
    let mut rows = Vec::new();
    for rt in Runtime::ALL {
        let client = client_for(rt, None, live);
        let answer = client
            .chat(&ChatRequest::new("qwen3-0.6b", &case.prompt))?
            .content;
        let failures = case.check(&answer);
        let exact_match = answer == EXACT;
        let _ = writeln!(
            out,
            "{:<9} properties {}  exact-string {}  {}",
            rt.name(),
            if failures.is_empty() { "PASS" } else { "FAIL" },
            if exact_match { "PASS" } else { "FAIL" },
            answer
        );
        rows.push(Row {
            runtime: rt,
            answer,
            failures,
            exact_match,
        });
    }
    let wrong_failures = case.check(WRONG);
    let _ = writeln!(out, "wrong answer {WRONG:?} -> {wrong_failures:?}");
    Ok(Summary {
        rows,
        wrong_failures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offline() -> Summary {
        run(false, &mut Vec::new()).unwrap()
    }

    #[test]
    fn the_spec_is_the_committed_eval_set() {
        let c = case();
        assert_eq!(c.id, "capital-france");
        assert_eq!(c.properties.len(), 3);
    }

    #[test]
    fn offline_run_satisfies_the_contract() {
        let mut buf = Vec::new();
        let s = run(false, &mut buf).unwrap();
        assert!(s.contract_holds());
        let text = String::from_utf8(buf).unwrap();
        for rt in Runtime::ALL {
            assert!(text.contains(rt.name()));
        }
        assert!(text.contains("properties PASS"));
        assert!(text.contains("exact-string FAIL"));
    }

    #[test]
    fn exact_string_is_brittle_where_properties_are_not() {
        // The recordings end with a full stop, so the exact-string test refuses a right answer.
        let s = offline();
        assert!(s.rows.iter().all(|r| !r.exact_match));
        assert!(s.rows.iter().all(|r| r.failures.is_empty()));
    }

    #[test]
    fn contract_fails_when_a_runtime_is_missing() {
        let mut s = offline();
        s.rows.pop();
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_when_an_answer_breaks_a_property() {
        let mut s = offline();
        s.rows[0].failures = case().check(WRONG);
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_when_the_wrong_answer_is_accepted() {
        let mut s = offline();
        s.wrong_failures.clear();
        assert!(!s.contract_holds());
        assert!(!case().check(WRONG).is_empty());
    }
}
