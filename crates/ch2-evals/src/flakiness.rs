//! Lesson 2.4: determinism and flakiness. One run is an anecdote; a pass rate over seeds is a
//! measurement. Greedy decoding (temperature 0) repeats exactly; sampling does not.

use ch1_sovereign::client_for;
use rll_client::Config;
use rll_evals::{
    parse_jsonl, run as run_suite, ClientResponder, RecordedVariants, Report, Responder, RunError,
};
use std::io::Write;

/// The eval set the lesson reads (`evals/flakiness.jsonl`).
pub const SET: &str = include_str!("../../../evals/flakiness.jsonl");
/// Ten fixed seeds: the same ten every time, so a report can be compared run to run.
pub const SEEDS: [u32; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
/// The sampling temperature of the "creative" runs.
pub const TEMPERATURE: f32 = 0.8;

/// Answers the offline run replays. The first answer of each prompt is the greedy one.
pub fn recordings() -> RecordedVariants {
    RecordedVariants::new()
        .with(
            "What is the capital of France? Answer in one sentence.",
            &[
                "Paris is the capital of France.",
                "The capital of France is Paris.",
            ],
        )
        .with(
            "What is 17 plus 25? Answer with just the number.",
            &["42", "It is 42.", "43", "42", "41"],
        )
        .with(
            "What is the chemical symbol for gold? Answer with just the symbol.",
            &["Ag", "Au", "Au.", "Au", "Gold"],
        )
}

pub struct Summary {
    pub greedy: Report,
    pub greedy_again: Report,
    pub sampled: Report,
    pub sampled_again: Report,
    /// Offline only: the recordings guarantee a flaky case, a live model may not produce one.
    pub require_flaky: bool,
}

impl Summary {
    /// The same seeds repeat exactly, greedy pass rates are 0 or 1, and (offline) sampling
    /// produces at least one flaky case.
    pub fn contract_holds(&self) -> bool {
        let greedy_binary = self
            .greedy
            .cases
            .iter()
            .all(|c| c.passes == 0 || c.passes == c.trials);
        self.greedy == self.greedy_again
            && self.sampled == self.sampled_again
            && greedy_binary
            && (!self.require_flaky || !self.sampled.flaky().is_empty())
    }
}

fn show(out: &mut impl Write, label: &str, r: &Report) {
    let _ = writeln!(
        out,
        "{label} (temperature {}, {} seeds)",
        r.temperature,
        r.seeds.len()
    );
    for c in &r.cases {
        let _ = writeln!(
            out,
            "  {:<15} {}/{} = {:.1}{}",
            c.id,
            c.passes,
            c.trials,
            c.pass_rate(),
            if c.is_flaky() { "  FLAKY" } else { "" }
        );
    }
}

fn measure(r: &dyn Responder) -> Result<[Report; 4], RunError> {
    let cases = parse_jsonl(SET).expect("evals/flakiness.jsonl is a valid eval set");
    Ok([
        run_suite(&cases, r, &SEEDS, 0.0)?,
        run_suite(&cases, r, &SEEDS, 0.0)?,
        run_suite(&cases, r, &SEEDS, TEMPERATURE)?,
        run_suite(&cases, r, &SEEDS, TEMPERATURE)?,
    ])
}

/// Offline replays the recordings; `live` asks the configured runtime.
pub fn run(cfg: &Config, live: bool, out: &mut impl Write) -> Result<Summary, RunError> {
    let reports = if live {
        let client = client_for(cfg.runtime, Some(&cfg.base_url), true);
        measure(&ClientResponder {
            client,
            model: cfg.model.clone(),
        })?
    } else {
        measure(&recordings())?
    };
    let [greedy, greedy_again, sampled, sampled_again] = reports;
    show(out, "greedy", &greedy);
    show(out, "sampled", &sampled);
    let _ = writeln!(
        out,
        "flaky under sampling: {}",
        sampled
            .flaky()
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok(Summary {
        greedy,
        greedy_again,
        sampled,
        sampled_again,
        require_flaky: !live,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rll_client::Runtime;

    fn cfg() -> Config {
        Config {
            runtime: Runtime::Apr,
            base_url: Runtime::Apr.default_base_url().into(),
            model: "m".into(),
        }
    }

    fn offline() -> Summary {
        run(&cfg(), false, &mut Vec::new()).unwrap()
    }

    #[test]
    fn offline_run_satisfies_the_contract_and_prints_the_story() {
        let mut buf = Vec::new();
        let s = run(&cfg(), false, &mut buf).unwrap();
        assert!(s.contract_holds());
        assert!(s.require_flaky);
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("greedy (temperature 0, 10 seeds)"));
        assert!(text.contains("sampled (temperature 0.8, 10 seeds)"));
        assert!(text.contains("FLAKY"));
        assert!(text.contains("flaky under sampling: "));
    }

    #[test]
    fn greedy_is_repeatable_but_not_necessarily_right() {
        let s = offline();
        let rate = |id: &str| {
            s.greedy
                .cases
                .iter()
                .find(|c| c.id == id)
                .unwrap()
                .pass_rate()
        };
        assert_eq!(rate("capital-france"), 1.0);
        assert_eq!(rate("sum-17-25"), 1.0);
        assert_eq!(rate("symbol-gold"), 0.0); // repeatably wrong
    }

    #[test]
    fn sampling_has_a_rate_strictly_between_zero_and_one() {
        let s = offline();
        let flaky = s.sampled.flaky();
        assert!(!flaky.is_empty());
        assert!(flaky
            .iter()
            .all(|c| c.pass_rate() > 0.0 && c.pass_rate() < 1.0));
    }

    #[test]
    fn contract_fails_when_a_repeat_differs() {
        let mut s = offline();
        s.sampled_again.cases[0].passes = 0;
        assert!(!s.contract_holds());
        let mut s = offline();
        s.greedy_again.cases[0].passes = 0;
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_when_greedy_is_not_all_or_nothing() {
        let mut s = offline();
        s.greedy.cases[0].passes = 3;
        s.greedy_again.cases[0].passes = 3;
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_when_sampling_never_flakes_offline_but_not_live() {
        let mut s = offline();
        s.sampled = s.greedy.clone();
        s.sampled_again = s.greedy.clone();
        assert!(!s.contract_holds());
        s.require_flaky = false;
        assert!(s.contract_holds());
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
