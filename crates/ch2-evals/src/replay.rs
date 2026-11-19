//! Recorded answers for the offline demos. Every fixture row must say whether it is
//! `recorded` (real model output) or `illustrative` (hand-written for the lesson), so an
//! invented answer can never pass as a measurement.

use rll_client::Error;
use rll_evals::Responder;
use serde_json::Value;
use std::collections::BTreeMap;

/// Parse a jsonl fixture; every row needs a `provenance` starting `recorded` or `illustrative`.
pub fn rows(text: &str) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value =
            serde_json::from_str(line).map_err(|e| format!("fixture line {}: {e}", i + 1))?;
        let label = v.get("provenance").and_then(Value::as_str).unwrap_or("");
        if !(label.starts_with("recorded") || label.starts_with("illustrative")) {
            return Err(format!(
                "fixture line {}: provenance must start with 'recorded' or 'illustrative'",
                i + 1
            ));
        }
        out.push(v);
    }
    if out.is_empty() {
        return Err("fixture is empty".into());
    }
    Ok(out)
}

/// A string field of a fixture row.
pub fn field<'a>(row: &'a Value, key: &str) -> Result<&'a str, String> {
    row.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("fixture row has no string field {key:?}"))
}

/// Replays recorded answers. A prompt is matched by the longest recorded prompt it starts with
/// (a retry prompt starts with the original), and the seed argument is the attempt number.
#[derive(Debug, Default)]
pub struct Replay {
    by_prompt: BTreeMap<String, Vec<String>>,
}

impl Replay {
    pub fn with(mut self, prompt: &str, answers: Vec<String>) -> Replay {
        self.by_prompt.insert(prompt.to_string(), answers);
        self
    }
}

impl Responder for Replay {
    fn answer(&self, prompt: &str, seed: u32, _temperature: f32) -> Result<String, Error> {
        let list = self
            .by_prompt
            .iter()
            .filter(|(k, _)| prompt.starts_with(k.as_str()))
            .max_by_key(|(k, _)| k.len())
            .map(|(_, v)| v)
            .ok_or_else(|| Error::BadResponse(format!("no recording for {prompt:?}")))?;
        list.get(seed as usize)
            .cloned()
            .ok_or_else(|| Error::BadResponse(format!("no recorded answer for attempt {seed}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = r#"{"provenance":"illustrative: x","a":"b"}"#;

    #[test]
    fn rows_accept_both_labels_and_skip_blank_lines() {
        let text = format!("{OK}\n\n{}\n", r#"{"provenance":"recorded: qwen run 1"}"#);
        assert_eq!(rows(&text).unwrap().len(), 2);
    }

    #[test]
    fn rows_refuse_an_unlabelled_fixture() {
        let e = rows(r#"{"a":"b"}"#).unwrap_err();
        assert!(e.contains("provenance"), "{e}");
        let e = rows(r#"{"provenance":"measured","a":"b"}"#).unwrap_err();
        assert!(e.contains("line 1"), "{e}");
    }

    #[test]
    fn rows_refuse_bad_json_and_empty_fixtures() {
        assert!(rows("{nope").unwrap_err().contains("line 1"));
        assert!(rows("\n\n").unwrap_err().contains("empty"));
    }

    #[test]
    fn field_reads_strings_only() {
        let v: Value = serde_json::from_str(r#"{"a":"b","n":1}"#).unwrap();
        assert_eq!(field(&v, "a").unwrap(), "b");
        assert!(field(&v, "n").is_err());
        assert!(field(&v, "missing").is_err());
    }

    #[test]
    fn replay_matches_the_longest_prefix_and_uses_the_seed_as_attempt() {
        let r = Replay::default()
            .with("Q", vec!["short".into()])
            .with("Q longer", vec!["first".into(), "second".into()]);
        assert_eq!(r.answer("Q", 0, 0.0).unwrap(), "short");
        assert_eq!(
            r.answer("Q longer, then feedback", 1, 0.0).unwrap(),
            "second"
        );
    }

    #[test]
    fn replay_errors_on_an_unknown_prompt_or_a_missing_attempt() {
        let r = Replay::default().with("Q", vec!["only".into()]);
        assert!(matches!(
            r.answer("other", 0, 0.0),
            Err(Error::BadResponse(_))
        ));
        assert!(matches!(r.answer("Q", 1, 0.0), Err(Error::BadResponse(_))));
    }
}
