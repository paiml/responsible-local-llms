//! Lesson 2.6: evals in CI. The same suite runs on every runtime, and a gate turns the pass
//! rate into a verdict a merge can depend on. A gate that never fails is decoration, so the
//! lesson also feeds it a model that got worse and shows it go red.

use ch1_sovereign::client_for;
use rll_client::{Config, Runtime};
use rll_evals::{
    parse_jsonl, run as run_suite, ClientResponder, RecordedVariants, Report, Responder, RunError,
};
use std::io::Write;

/// The eval set the gate runs (`evals/ci-gate.jsonl`).
pub const SET: &str = include_str!("../../../evals/ci-gate.jsonl");
/// Three fixed seeds at temperature 0: greedy decoding, so a verdict is repeatable.
pub const SEEDS: [u32; 3] = [0, 1, 2];
/// The suite pass rate a runtime must reach. Every case must pass every seed.
pub const MIN_RATE: f64 = 1.0;

/// What one runtime's good build says offline. Phrasing differs per runtime; the properties
/// do not care.
pub fn recordings(rt: Runtime) -> RecordedVariants {
    let (france, planet) = match rt {
        Runtime::Apr => (
            "The capital of France is Paris.",
            "Jupiter is the largest planet in the Solar System.",
        ),
        Runtime::Ollama => ("Paris.", "The largest planet is Jupiter."),
        Runtime::LlamaCpp => (
            "Paris is the capital of France.",
            "It is Jupiter, a gas giant.",
        ),
    };
    RecordedVariants::new()
        .with(
            "What is the capital of France? Answer in one sentence.",
            &[france],
        )
        .with("What is 17 plus 25? Answer with just the number.", &["42"])
        .with(
            "What is the largest planet in the Solar System? Answer in one sentence.",
            &[planet],
        )
}

/// A model update that got worse: wrong on every case.
pub fn regressed_recordings() -> RecordedVariants {
    RecordedVariants::new()
        .with(
            "What is the capital of France? Answer in one sentence.",
            &["Lyon is the capital of France."],
        )
        .with("What is 17 plus 25? Answer with just the number.", &["43"])
        .with(
            "What is the largest planet in the Solar System? Answer in one sentence.",
            &["Saturn is the largest planet."],
        )
}

/// One runtime's measured report.
pub struct Row {
    pub runtime: Runtime,
    pub report: Report,
}

pub struct Summary {
    pub rows: Vec<Row>,
    /// The same suite against the worse model.
    pub regressed: Report,
    pub min_rate: f64,
    /// How many runtimes the run must cover: all three offline, the configured one live.
    pub expected_runtimes: usize,
}

impl Summary {
    /// Every expected runtime clears the gate, and the worse model does not.
    pub fn contract_holds(&self) -> bool {
        self.rows.len() == self.expected_runtimes
            && self.rows.iter().all(|r| r.report.gate(self.min_rate))
            && !self.regressed.gate(self.min_rate)
    }
}

/// The ids of cases whose pass rate fell from `before` to `after`.
pub fn dropped(before: &Report, after: &Report) -> Vec<String> {
    before
        .cases
        .iter()
        .zip(&after.cases)
        .filter(|(b, a)| a.pass_rate() < b.pass_rate())
        .map(|(b, _)| b.id.clone())
        .collect()
}

fn measure(r: &dyn Responder) -> Result<Report, RunError> {
    let cases = parse_jsonl(SET).expect("evals/ci-gate.jsonl is a valid eval set");
    run_suite(&cases, r, &SEEDS, 0.0)
}

fn verdict(r: &Report, min_rate: f64) -> &'static str {
    if r.gate(min_rate) {
        "GATE PASS"
    } else {
        "GATE FAIL"
    }
}

/// Offline replays recordings on all three runtimes; `live` asks the configured runtime.
pub fn run(cfg: &Config, live: bool, out: &mut impl Write) -> Result<Summary, RunError> {
    let responders: Vec<(Runtime, Box<dyn Responder>)> = if live {
        let client = client_for(cfg.runtime, Some(&cfg.base_url), true);
        let model = cfg.model.clone();
        vec![(cfg.runtime, Box::new(ClientResponder { client, model }))]
    } else {
        Runtime::ALL
            .into_iter()
            .map(|rt| (rt, Box::new(recordings(rt)) as Box<dyn Responder>))
            .collect()
    };
    let mut rows = Vec::new();
    for (runtime, responder) in &responders {
        rows.push(Row {
            runtime: *runtime,
            report: measure(responder.as_ref())?,
        });
    }
    let regressed = measure(&regressed_recordings())?;
    let baseline = rows
        .first()
        .map(|r| r.report.clone())
        .expect("at least one runtime");
    for r in &rows {
        let _ = writeln!(
            out,
            "{:<9} {:.2} of {:.2}  {}",
            r.runtime.name(),
            r.report.suite_pass_rate(),
            MIN_RATE,
            verdict(&r.report, MIN_RATE)
        );
    }
    let _ = writeln!(
        out,
        "worse model {:.2} of {:.2}  {}  dropped: {}",
        regressed.suite_pass_rate(),
        MIN_RATE,
        verdict(&regressed, MIN_RATE),
        dropped(&baseline, &regressed).join(", ")
    );
    Ok(Summary {
        rows,
        regressed,
        min_rate: MIN_RATE,
        expected_runtimes: if live { 1 } else { Runtime::ALL.len() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> Config {
        Config {
            runtime: Runtime::Ollama,
            base_url: Runtime::Ollama.default_base_url().into(),
            model: "m".into(),
        }
    }

    fn offline() -> Summary {
        run(&cfg(), false, &mut Vec::new()).unwrap()
    }

    #[test]
    fn offline_covers_all_three_runtimes_and_the_contract_holds() {
        let mut buf = Vec::new();
        let s = run(&cfg(), false, &mut buf).unwrap();
        assert!(s.contract_holds());
        assert_eq!(s.rows.len(), 3);
        let text = String::from_utf8(buf).unwrap();
        for name in ["apr", "ollama", "llamacpp"] {
            assert!(text.contains(name), "{name}");
        }
        assert!(text.contains("GATE PASS"));
        assert!(text.contains("GATE FAIL"));
        assert!(text.contains("dropped: capital-france, sum-17-25, largest-planet"));
    }

    #[test]
    fn the_gate_goes_red_on_a_worse_model() {
        let s = offline();
        assert!(s.regressed.suite_pass_rate() < 0.01);
        assert!(!s.regressed.gate(MIN_RATE));
    }

    #[test]
    fn the_gate_refuses_one_failing_case_on_one_runtime() {
        let mut s = offline();
        s.rows[1].report.cases[0].passes = 2;
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_when_a_runtime_is_missing() {
        let mut s = offline();
        s.rows.pop();
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_when_the_gate_passes_a_worse_model() {
        let mut s = offline();
        s.regressed = s.rows[0].report.clone();
        assert!(!s.contract_holds());
    }

    #[test]
    fn dropped_lists_only_cases_that_fell() {
        let s = offline();
        let mut after = s.rows[0].report.clone();
        after.cases[1].passes = 0;
        assert_eq!(dropped(&s.rows[0].report, &after), vec!["sum-17-25"]);
        assert!(dropped(&after, &s.rows[0].report).is_empty());
    }

    #[test]
    fn live_run_without_a_server_is_an_error() {
        let mut c = cfg();
        c.base_url = "http://127.0.0.1:1".into();
        assert!(matches!(
            run(&c, true, &mut Vec::new()),
            Err(RunError::Model(_))
        ));
    }
}
