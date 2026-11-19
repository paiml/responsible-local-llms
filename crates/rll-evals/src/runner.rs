//! The seeded runner: every case, once per seed, at one temperature, reported as pass rates.

use crate::schema::Case;
use rll_client::{ChatRequest, Client, Error};
use std::collections::BTreeMap;
use std::fmt;

/// Whatever answers a prompt for a given seed and temperature.
pub trait Responder {
    fn answer(&self, prompt: &str, seed: u32, temperature: f32) -> Result<String, Error>;
}

/// A real runtime (apr, Ollama or llama.cpp) behind `rll-client`.
pub struct ClientResponder {
    pub client: Client,
    pub model: String,
}

impl Responder for ClientResponder {
    fn answer(&self, prompt: &str, seed: u32, temperature: f32) -> Result<String, Error> {
        let mut req = ChatRequest::new(&self.model, prompt);
        req.seed = seed;
        req.temperature = temperature;
        Ok(self.client.chat(&req)?.content)
    }
}

/// Offline stand-in for a sampling model: several recorded answers per prompt.
///
/// Temperature 0 always returns the first recording (greedy decoding). Any temperature above 0
/// picks a recording from the seed with a fixed hash, so the same `(prompt, seed)` always gives
/// the same pick and different seeds spread over the variants. The size of the temperature is
/// ignored: this models "sampling is on", not a distribution.
#[derive(Default)]
pub struct RecordedVariants {
    by_prompt: BTreeMap<String, Vec<String>>,
}

impl RecordedVariants {
    pub fn new() -> RecordedVariants {
        RecordedVariants::default()
    }

    /// Record the answers one prompt can produce; the first is the greedy answer.
    pub fn with(mut self, prompt: &str, answers: &[&str]) -> RecordedVariants {
        let list = answers.iter().map(|a| a.to_string()).collect();
        self.by_prompt.insert(prompt.to_string(), list);
        self
    }
}

fn mix(mut x: u64) -> u64 {
    // splitmix64 finalizer: a fixed, dependency-free hash.
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

impl Responder for RecordedVariants {
    fn answer(&self, prompt: &str, seed: u32, temperature: f32) -> Result<String, Error> {
        let list = match self.by_prompt.get(prompt) {
            Some(l) if !l.is_empty() => l,
            _ => return Err(Error::BadResponse(format!("no recording for {prompt:?}"))),
        };
        let pick = if temperature > 0.0 {
            (mix(fnv(prompt) ^ u64::from(seed)) % list.len() as u64) as usize
        } else {
            0
        };
        Ok(list[pick].clone())
    }
}

/// Why a run could not produce a report.
#[derive(Debug, PartialEq, Eq)]
pub enum RunError {
    NoCases,
    NoSeeds,
    Model(Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::NoCases => write!(f, "no cases to run"),
            RunError::NoSeeds => write!(f, "no seeds: a pass rate needs at least one trial"),
            RunError::Model(e) => write!(f, "model call failed: {e}"),
        }
    }
}

impl std::error::Error for RunError {}

impl From<Error> for RunError {
    fn from(e: Error) -> RunError {
        RunError::Model(e)
    }
}

/// One case's result over all seeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseStat {
    pub id: String,
    pub passes: u32,
    pub trials: u32,
}

impl CaseStat {
    /// Fraction of trials that passed every property.
    pub fn pass_rate(&self) -> f64 {
        f64::from(self.passes) / f64::from(self.trials)
    }

    /// Flaky: it passes sometimes, not always.
    pub fn is_flaky(&self) -> bool {
        self.passes > 0 && self.passes < self.trials
    }
}

/// The result of one run. Equal inputs give an equal report.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub temperature: f32,
    pub seeds: Vec<u32>,
    pub cases: Vec<CaseStat>,
}

impl Report {
    /// Passes over trials, across every case.
    pub fn suite_pass_rate(&self) -> f64 {
        let passes: u32 = self.cases.iter().map(|c| c.passes).sum();
        let trials: u32 = self.cases.iter().map(|c| c.trials).sum();
        f64::from(passes) / f64::from(trials)
    }

    /// The cases that pass sometimes and fail sometimes.
    pub fn flaky(&self) -> Vec<&CaseStat> {
        self.cases.iter().filter(|c| c.is_flaky()).collect()
    }

    /// A regression gate: the suite pass rate must reach `min_rate`.
    pub fn gate(&self, min_rate: f64) -> bool {
        self.suite_pass_rate() >= min_rate
    }
}

/// Ask every case once per seed and count the passes.
pub fn run(
    cases: &[Case],
    responder: &dyn Responder,
    seeds: &[u32],
    temperature: f32,
) -> Result<Report, RunError> {
    if cases.is_empty() {
        return Err(RunError::NoCases);
    }
    if seeds.is_empty() {
        return Err(RunError::NoSeeds);
    }
    let mut stats = Vec::with_capacity(cases.len());
    for case in cases {
        let mut passes = 0;
        for &seed in seeds {
            let answer = responder.answer(&case.prompt, seed, temperature)?;
            if case.check(&answer).is_empty() {
                passes += 1;
            }
        }
        stats.push(CaseStat {
            id: case.id.clone(),
            passes,
            trials: seeds.len() as u32,
        });
    }
    Ok(Report {
        temperature,
        seeds: seeds.to_vec(),
        cases: stats,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_jsonl;
    use rll_client::fixtures::Fixture;
    use rll_client::Runtime;
    use std::time::Duration;

    fn cases() -> Vec<Case> {
        parse_jsonl(
            r#"{"id":"a","prompt":"pa","properties":[{"kind":"contains","value":"yes"}]}
{"id":"b","prompt":"pb","properties":[{"kind":"contains","value":"yes"}]}"#,
        )
        .unwrap()
    }

    fn model() -> RecordedVariants {
        RecordedVariants::new()
            .with("pa", &["yes", "yes"]) // always right
            .with("pb", &["yes", "no", "no", "yes", "no"]) // greedy right, sampling flaky
    }

    #[test]
    fn greedy_decoding_is_all_or_nothing() {
        let r = run(&cases(), &model(), &[1, 2, 3, 4], 0.0).unwrap();
        assert!(r
            .cases
            .iter()
            .all(|c| c.passes == 0 || c.passes == c.trials));
        assert!(r.flaky().is_empty());
        assert!((r.suite_pass_rate() - 1.0).abs() < 1e-12);
        assert!(r.gate(1.0));
    }

    #[test]
    fn sampling_makes_a_flaky_case_and_the_same_seeds_repeat_it() {
        let seeds: Vec<u32> = (0..20).collect();
        let a = run(&cases(), &model(), &seeds, 0.8).unwrap();
        let b = run(&cases(), &model(), &seeds, 0.8).unwrap();
        assert_eq!(a, b);
        let flaky = a.flaky();
        assert_eq!(flaky.len(), 1);
        assert_eq!(flaky[0].id, "b");
        let rate = flaky[0].pass_rate();
        assert!(rate > 0.0 && rate < 1.0);
        assert!(!a.gate(1.0));
        assert!(a.gate(a.suite_pass_rate()));
    }

    #[test]
    fn different_seeds_can_change_the_report() {
        let a = run(&cases(), &model(), &[0, 1, 2, 3, 4, 5], 0.8).unwrap();
        let b = run(&cases(), &model(), &[10, 11, 12, 13, 14, 15], 0.8).unwrap();
        assert_ne!(a.seeds, b.seeds);
        assert_ne!(a, b);
    }

    #[test]
    fn a_never_passing_case_is_not_flaky() {
        let c = CaseStat {
            id: "x".into(),
            passes: 0,
            trials: 5,
        };
        assert!(!c.is_flaky());
        assert_eq!(c.pass_rate(), 0.0);
    }

    #[test]
    fn empty_inputs_are_refused() {
        assert_eq!(
            run(&[], &model(), &[1], 0.0).unwrap_err(),
            RunError::NoCases
        );
        assert_eq!(
            run(&cases(), &model(), &[], 0.0).unwrap_err(),
            RunError::NoSeeds
        );
    }

    #[test]
    fn a_missing_recording_is_a_model_error() {
        let m = RecordedVariants::new().with("pa", &["yes"]).with("pb", &[]);
        let e = run(&cases(), &m, &[1], 0.0).unwrap_err();
        assert_eq!(
            e,
            RunError::Model(Error::BadResponse("no recording for \"pb\"".into()))
        );
        let m = RecordedVariants::new();
        assert!(matches!(
            run(&cases(), &m, &[1], 0.0),
            Err(RunError::Model(_))
        ));
    }

    #[test]
    fn run_errors_display() {
        assert_eq!(RunError::NoCases.to_string(), "no cases to run");
        assert!(RunError::NoSeeds.to_string().contains("no seeds"));
        assert_eq!(
            RunError::from(Error::EmptyChoices).to_string(),
            "model call failed: reply held no choices"
        );
    }

    #[test]
    fn client_responder_sends_seed_and_temperature_and_returns_content() {
        let body = r#"{"choices":[{"message":{"content":"yes please"}}]}"#;
        let client = Client::new(
            Runtime::Apr,
            "http://x:1",
            Box::new(Fixture::new(body, Duration::from_millis(10))),
        );
        let r = ClientResponder {
            client,
            model: "m".into(),
        };
        assert_eq!(r.answer("pa", 9, 0.5).unwrap(), "yes please");
        let report = run(&cases(), &r, &[1, 2], 0.0).unwrap();
        assert_eq!(report.suite_pass_rate(), 1.0);
    }

    #[test]
    fn client_responder_surfaces_transport_errors() {
        let client = Client::new(
            Runtime::Apr,
            "http://x:1",
            Box::new(Fixture::new("not json", Duration::from_millis(10))),
        );
        let r = ClientResponder {
            client,
            model: "m".into(),
        };
        assert!(matches!(r.answer("p", 1, 0.0), Err(Error::Json(_))));
    }
}
