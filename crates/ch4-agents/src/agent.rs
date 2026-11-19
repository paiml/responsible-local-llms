//! Lesson 4.1: a local agent in Rust. The model's text is only ever a request: it must parse
//! into a typed [`ToolCall`], and only then does the agent run it.

use crate::agency::{Guarded, Outcome, Policy};
use crate::tool::{ToolCall, Workspace};
use ch1_sovereign::client_for;
use rll_client::{Config, Error};
use rll_evals::{ClientResponder, RecordedVariants, Responder};
use std::io::Write;

/// The prompt the lesson sends. It names the one JSON shape the agent accepts.
pub const PROMPT: &str = "Reply with only a JSON tool call, for example {\"tool\":\"add\",\"a\":1,\"b\":2}. What is 17 plus 25?";

/// Outputs a real model can produce that must never run.
pub const BAD_OUTPUTS: [&str; 5] = [
    "I would add them together: the answer is 42.",
    "{\"tool\":\"format_disk\"}",
    "{\"tool\":\"add\",\"a\":\"17\",\"b\":25}",
    "{\"tool\":\"add\",\"a\":1,\"b\":2,\"extra\":true}",
    "{\"tool\":\"delete_file\" ",
];

/// One agent step: parse the model's text, then run the typed call through the guard.
pub fn step(g: &mut Guarded, model_output: &str) -> Outcome {
    match ToolCall::parse(model_output) {
        Ok(call) => g.run(&call),
        Err(reason) => Outcome::Refused(reason),
    }
}

/// The recorded model output the offline run replays (recorded by hand, not measured).
pub fn recordings() -> RecordedVariants {
    RecordedVariants::new().with(
        PROMPT,
        &["Sure. {\"tool\":\"add\",\"a\":17,\"b\":25} Hope that helps."],
    )
}

pub struct Summary {
    pub model_output: String,
    pub outcome: Outcome,
    pub bad: Vec<Outcome>,
    pub workspace_unchanged: bool,
    /// Offline only: the recording guarantees an executed call, a live model may not make one.
    pub require_executed: bool,
}

impl Summary {
    /// The model's well-formed call ran (offline: with the right answer), and every malformed,
    /// unknown or over-specified output was refused with the workspace untouched.
    pub fn contract_holds(&self) -> bool {
        let executed_ok = !self.require_executed || self.outcome == Outcome::Executed("42".into());
        executed_ok
            && self.workspace_unchanged
            && self.bad.iter().all(|o| matches!(o, Outcome::Refused(_)))
    }
}

fn model_output(cfg: &Config, live: bool) -> Result<String, Error> {
    if live {
        let client = client_for(cfg.runtime, Some(&cfg.base_url), true);
        ClientResponder {
            client,
            model: cfg.model.clone(),
        }
        .answer(PROMPT, 0, 0.0)
    } else {
        recordings().answer(PROMPT, 0, 0.0)
    }
}

/// Offline replays the recording; `live` asks the configured runtime.
pub fn run(cfg: &Config, live: bool, out: &mut impl Write) -> Result<Summary, Error> {
    let model_output = model_output(cfg, live)?;
    let seed = Workspace::new().with_file("notes.txt", "keep me");
    let mut g = Guarded::new(seed.clone(), Policy::open());
    let outcome = step(&mut g, &model_output);
    let _ = writeln!(out, "model said : {model_output}");
    let _ = writeln!(out, "agent did  : {outcome}");
    let mut hostile = Guarded::new(seed.clone(), Policy::open());
    let bad: Vec<Outcome> = BAD_OUTPUTS
        .iter()
        .map(|b| {
            let o = step(&mut hostile, b);
            let _ = writeln!(out, "bad output : {b:?}\n  agent did: {o}");
            o
        })
        .collect();
    Ok(Summary {
        model_output,
        outcome,
        bad,
        workspace_unchanged: hostile.ws == seed && hostile.executed.is_empty(),
        require_executed: !live,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rll_client::Runtime;

    fn cfg() -> Config {
        Config {
            runtime: Runtime::Apr,
            base_url: "http://127.0.0.1:1".into(),
            model: "m".into(),
        }
    }

    #[test]
    fn offline_run_executes_the_good_call_and_refuses_the_bad_ones() {
        let mut buf = Vec::new();
        let s = run(&cfg(), false, &mut buf).unwrap();
        assert!(s.contract_holds());
        assert_eq!(s.outcome.to_string(), "executed: 42");
        assert!(s.model_output.contains("Sure."));
        assert_eq!(s.bad.len(), BAD_OUTPUTS.len());
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("agent did  : executed: 42"));
        assert!(text.contains("format_disk"));
    }

    #[test]
    fn live_run_without_a_server_is_an_error() {
        assert!(run(&cfg(), true, &mut Vec::new()).is_err());
    }

    #[test]
    fn contract_fails_when_a_bad_output_runs() {
        let mut s = run(&cfg(), false, &mut Vec::new()).unwrap();
        s.bad[0] = Outcome::Executed("oops".into());
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_when_the_workspace_changed() {
        let mut s = run(&cfg(), false, &mut Vec::new()).unwrap();
        s.workspace_unchanged = false;
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_offline_when_the_good_call_did_not_run_but_not_live() {
        let mut s = run(&cfg(), false, &mut Vec::new()).unwrap();
        s.outcome = Outcome::Refused("no".into());
        assert!(!s.contract_holds());
        s.require_executed = false;
        assert!(s.contract_holds());
    }

    #[test]
    fn a_step_runs_a_typed_call_and_refuses_prose() {
        let mut g = Guarded::new(Workspace::new(), Policy::open());
        assert!(matches!(step(&mut g, "hello"), Outcome::Refused(_)));
        let out = step(&mut g, r#"{"tool":"add","a":1,"b":2}"#);
        assert_eq!(out, Outcome::Executed("3".into()));
    }
}
