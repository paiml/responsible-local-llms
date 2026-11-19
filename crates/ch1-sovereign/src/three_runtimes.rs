//! Lesson 2: one model, three runtimes. The same request goes to apr, Ollama
//! and llama.cpp; the answers are compared.

use crate::client_for;
use rll_client::{ChatRequest, Completion, Error, Runtime};
use std::io::Write;

pub const PROMPT: &str = "What is the capital of France? Answer in one sentence.";

pub struct Summary {
    pub answers: Vec<(Runtime, Completion)>,
}

impl Summary {
    /// Every runtime answered, and every answer contains "Paris".
    pub fn contract_holds(&self) -> bool {
        self.answers.len() == Runtime::ALL.len()
            && self
                .answers
                .iter()
                .all(|(_, c)| !c.content.is_empty() && c.content.contains("Paris"))
    }
}

/// Asks all three runtimes; each listens at its `RLL_BASE_URL_*` or its default port.
pub fn run(live: bool, out: &mut impl Write) -> Result<Summary, Error> {
    run_at(live, |k| std::env::var(k).ok(), out)
}

/// `run` with the environment lookup passed in, so tests need no process environment.
pub fn run_at<F: Fn(&str) -> Option<String>>(
    live: bool,
    lookup: F,
    out: &mut impl Write,
) -> Result<Summary, Error> {
    let mut answers = Vec::new();
    for rt in Runtime::ALL {
        let client = client_for(rt, Some(&rt.base_url_from(&lookup)), live);
        let c = client.chat(&ChatRequest::new("qwen3-0.6b", PROMPT))?;
        let _ = writeln!(
            out,
            "{:<9} {:>6.1} tok/s  {}",
            rt.name(),
            c.tokens_per_second(),
            c.content
        );
        answers.push((rt, c));
    }
    Ok(Summary { answers })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_run_satisfies_the_contract() {
        let mut buf = Vec::new();
        let s = run(false, &mut buf).unwrap();
        assert!(s.contract_holds());
        let text = String::from_utf8(buf).unwrap();
        for rt in Runtime::ALL {
            assert!(text.contains(rt.name()));
        }
    }

    #[test]
    fn run_at_reads_a_url_per_runtime() {
        // A live run against per-runtime URLs nobody listens on fails with an error.
        let mut buf = Vec::new();
        let r = run_at(true, |_| Some("http://127.0.0.1:1".into()), &mut buf);
        assert!(r.is_err());
    }

    #[test]
    fn contract_fails_when_a_runtime_is_missing() {
        let mut buf = Vec::new();
        let mut s = run(false, &mut buf).unwrap();
        s.answers.pop();
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_on_a_wrong_answer() {
        let mut buf = Vec::new();
        let mut s = run(false, &mut buf).unwrap();
        s.answers[0].1.content = "Lyon".into();
        assert!(!s.contract_holds());
    }

    #[test]
    fn live_run_without_a_server_is_an_error() {
        // Nothing listens on the default ports in CI; the error must surface, not panic.
        let mut buf = Vec::new();
        assert!(run(true, &mut buf).is_err());
    }
}
