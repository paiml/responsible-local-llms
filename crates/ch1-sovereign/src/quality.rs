//! Lesson 1.3, the other half: quantization can cost answer quality. The size
//! and speed demo cannot see that, so this module scores answers against the
//! `evals/quantization-quality.jsonl` set, one score per quantization level.

use crate::client_for;
use rll_client::{ChatRequest, Config, Error};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Write;

/// The planned set, one case per line: a prompt plus the properties its answer must have.
pub const CASES_JSONL: &str = include_str!("../../../evals/quantization-quality.jsonl");
/// Answers recorded per level for offline runs. Illustrative, not a measurement.
pub const RECORDED_JSONL: &str =
    include_str!("../../../evals/fixtures/quantization-quality.recorded.jsonl");
/// The levels lesson 1.3 compares, smallest first.
pub const LEVELS: [&str; 3] = ["q4_k_m", "q8_0", "f16"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    pub id: String,
    pub prompt: String,
    pub must_contain: Vec<String>,
    pub must_not_contain: Vec<String>,
}

impl Case {
    /// A case passes when the answer holds every required property: all `must_contain`
    /// strings present, no `must_not_contain` string present (case-insensitive).
    pub fn passes(&self, answer: &str) -> bool {
        let a = answer.to_lowercase();
        self.must_contain
            .iter()
            .all(|s| a.contains(&s.to_lowercase()))
            && !self
                .must_not_contain
                .iter()
                .any(|s| a.contains(&s.to_lowercase()))
    }
}

fn bad(msg: String) -> Error {
    Error::Json(msg)
}

fn text(v: &Value, key: &str, line: &str) -> Result<String, Error> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| bad(format!("missing string \"{key}\" in {line}")))
}

fn list(v: &Value, key: &str, line: &str) -> Result<Vec<String>, Error> {
    v.get(key)
        .and_then(Value::as_array)
        .and_then(|a| {
            a.iter()
                .map(|x| x.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()
        })
        .ok_or_else(|| bad(format!("missing string list \"{key}\" in {line}")))
}

fn lines(jsonl: &str) -> impl Iterator<Item = &str> {
    jsonl.lines().filter(|l| !l.trim().is_empty())
}

/// Parse a case set. An empty set is an error: a score over zero cases means nothing.
pub fn parse_cases(jsonl: &str) -> Result<Vec<Case>, Error> {
    let mut cases = Vec::new();
    for line in lines(jsonl) {
        let v: Value = serde_json::from_str(line).map_err(|e| bad(e.to_string()))?;
        cases.push(Case {
            id: text(&v, "id", line)?,
            prompt: text(&v, "prompt", line)?,
            must_contain: list(&v, "must_contain", line)?,
            must_not_contain: list(&v, "must_not_contain", line)?,
        });
    }
    if cases.is_empty() {
        return Err(bad("the case set is empty".into()));
    }
    Ok(cases)
}

/// Recorded answers: level -> case id -> answer.
pub fn parse_recorded(jsonl: &str) -> Result<BTreeMap<String, BTreeMap<String, String>>, Error> {
    let mut out: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for line in lines(jsonl) {
        let v: Value = serde_json::from_str(line).map_err(|e| bad(e.to_string()))?;
        out.entry(text(&v, "level", line)?)
            .or_default()
            .insert(text(&v, "id", line)?, text(&v, "answer", line)?);
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelScore {
    pub level: String,
    pub passed: usize,
    pub total: usize,
}

impl LevelScore {
    pub fn rate(&self) -> f64 {
        self.passed as f64 / self.total as f64
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub scores: Vec<LevelScore>,
}

impl Summary {
    /// At least one level was scored; every level saw the same non-empty case count; no level
    /// passed more cases than it saw. Quality is a measured rate, never an assumed one.
    pub fn contract_holds(&self) -> bool {
        let Some(first) = self.scores.first() else {
            return false;
        };
        first.total > 0
            && self
                .scores
                .iter()
                .all(|s| s.total == first.total && s.passed <= s.total)
    }

    /// Pass rate of the first level minus the last (smallest minus largest when offline).
    /// Negative means the smaller file answered worse.
    pub fn quality_delta(&self) -> Option<f64> {
        match (self.scores.first(), self.scores.last()) {
            (Some(a), Some(b)) if self.scores.len() > 1 => Some(a.rate() - b.rate()),
            _ => None,
        }
    }
}

fn score(
    level: &str,
    cases: &[Case],
    mut answer: impl FnMut(&Case) -> Result<String, Error>,
) -> Result<LevelScore, Error> {
    let mut passed = 0;
    for c in cases {
        if c.passes(&answer(c)?) {
            passed += 1;
        }
    }
    Ok(LevelScore {
        level: level.to_string(),
        passed,
        total: cases.len(),
    })
}

/// Score every level in the recorded answers; a missing level or answer is an error.
pub fn score_recorded(
    cases: &[Case],
    recorded: &BTreeMap<String, BTreeMap<String, String>>,
) -> Result<Vec<LevelScore>, Error> {
    let mut scores = Vec::new();
    for lv in LEVELS {
        let answers = recorded
            .get(lv)
            .ok_or_else(|| bad(format!("no recorded answers for {lv}")))?;
        scores.push(score(lv, cases, |c| {
            answers
                .get(&c.id)
                .cloned()
                .ok_or_else(|| bad(format!("no recorded {lv} answer for {}", c.id)))
        })?);
    }
    Ok(scores)
}

/// Offline: score every recorded level. Live: ask the configured runtime, which serves one
/// file, so only `level` (what `RLL_QUANT` names) is scored.
pub fn run(cfg: &Config, live: bool, level: &str, out: &mut impl Write) -> Result<Summary, Error> {
    let cases = parse_cases(CASES_JSONL)?;
    let mut scores = Vec::new();
    if live {
        let client = client_for(cfg.runtime, Some(&cfg.base_url), true);
        scores.push(score(level, &cases, |c| {
            Ok(client
                .chat(&ChatRequest::new(&cfg.model, &c.prompt))?
                .content)
        })?);
    } else {
        scores = score_recorded(&cases, &parse_recorded(RECORDED_JSONL)?)?;
    }
    for s in &scores {
        let _ = writeln!(out, "{:<7} {}/{} cases pass", s.level, s.passed, s.total);
    }
    Ok(Summary { scores })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rll_client::Runtime;

    fn case(must: &[&str], not: &[&str]) -> Case {
        Case {
            id: "c".into(),
            prompt: "p".into(),
            must_contain: must.iter().map(|s| s.to_string()).collect(),
            must_not_contain: not.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn cfg() -> Config {
        Config {
            runtime: Runtime::Apr,
            base_url: "http://127.0.0.1:1".into(),
            model: "m".into(),
        }
    }

    #[test]
    fn checker_demands_every_property() {
        let c = case(&["paris"], &["lyon"]);
        assert!(c.passes("It is PARIS."));
        assert!(!c.passes("Lyon"), "missing required property");
        assert!(!c.passes("Paris, not Lyon"), "forbidden property present");
    }

    #[test]
    fn shipped_set_parses_and_every_case_is_satisfiable() {
        let cases = parse_cases(CASES_JSONL).unwrap();
        assert!(cases.len() >= 8);
        for c in &cases {
            assert!(
                !c.must_contain.is_empty(),
                "{} has no required property",
                c.id
            );
            let answer = c.must_contain.join(" ");
            assert!(c.passes(&answer), "{} cannot be satisfied", c.id);
        }
    }

    #[test]
    fn offline_run_scores_three_levels_and_smaller_is_worse() {
        let mut buf = Vec::new();
        let s = run(&cfg(), false, "q4_k_m", &mut buf).unwrap();
        assert!(s.contract_holds());
        let got: Vec<(&str, usize)> = s
            .scores
            .iter()
            .map(|x| (x.level.as_str(), x.passed))
            .collect();
        assert_eq!(got, [("q4_k_m", 3), ("q8_0", 7), ("f16", 8)]);
        assert!((s.scores[2].rate() - 1.0).abs() < 1e-12);
        assert!((s.quality_delta().unwrap() + 5.0 / 8.0).abs() < 1e-12);
        assert!(String::from_utf8(buf).unwrap().contains("q4_k_m  3/8"));
    }

    #[test]
    fn contract_rejects_empty_uneven_and_overfull_scores() {
        let ls = |l: &str, p, t| LevelScore {
            level: l.into(),
            passed: p,
            total: t,
        };
        assert!(!Summary { scores: vec![] }.contract_holds());
        assert!(!Summary {
            scores: vec![ls("a", 0, 0)]
        }
        .contract_holds());
        assert!(!Summary {
            scores: vec![ls("a", 1, 2), ls("b", 1, 3)]
        }
        .contract_holds());
        assert!(!Summary {
            scores: vec![ls("a", 3, 2)]
        }
        .contract_holds());
        let one = Summary {
            scores: vec![ls("a", 1, 2)],
        };
        assert!(one.contract_holds());
        assert_eq!(one.quality_delta(), None);
        assert_eq!(Summary { scores: vec![] }.quality_delta(), None);
    }

    #[test]
    fn a_broken_answer_set_fails_the_score() {
        // Falsifier: wrong answers everywhere must score zero, not pass.
        let cases = parse_cases(CASES_JSONL).unwrap();
        let s = score("q4_k_m", &cases, |_| Ok("I do not know".into())).unwrap();
        assert_eq!(s.passed, 0);
    }

    #[test]
    fn parse_errors_are_reported() {
        assert!(parse_cases("").is_err());
        assert!(parse_cases("not json").is_err());
        assert!(parse_cases(r#"{"id":"a","prompt":"p","must_contain":["x"]}"#).is_err());
        assert!(
            parse_cases(r#"{"id":"a","prompt":"p","must_contain":[1],"must_not_contain":[]}"#)
                .is_err()
        );
        assert!(parse_cases(r#"{"id":"a","must_contain":[],"must_not_contain":[]}"#).is_err());
        assert!(parse_recorded("not json").is_err());
        assert!(parse_recorded(r#"{"id":"a","level":"x"}"#).is_err());
    }

    #[test]
    fn missing_recorded_levels_and_answers_are_errors() {
        let cases = parse_cases(CASES_JSONL).unwrap();
        let e = score("q4_k_m", &cases, |_| Err(Error::EmptyChoices)).unwrap_err();
        assert_eq!(e, Error::EmptyChoices);
        let mut rec = parse_recorded(RECORDED_JSONL).unwrap();
        rec.get_mut("q8_0").unwrap().remove("sum-17-25");
        let e = score_recorded(&cases, &rec).unwrap_err();
        assert!(matches!(e, Error::Json(m) if m.contains("sum-17-25")));
        rec.remove("q8_0");
        let e = score_recorded(&cases, &rec).unwrap_err();
        assert!(matches!(e, Error::Json(m) if m.contains("q8_0")));
    }

    #[test]
    fn live_run_without_a_server_is_an_error() {
        let mut buf = Vec::new();
        assert!(run(&cfg(), true, "q4_k_m", &mut buf).is_err());
    }
}
