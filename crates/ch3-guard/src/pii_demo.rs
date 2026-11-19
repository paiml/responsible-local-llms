//! Lesson 3.3: secrets and PII out of prompts and logs.

use crate::pii::{detect, kinds_label, log_line, redact, Kind};
use crate::valid_source;
use serde::Deserialize;
use std::io::Write;

/// The sample set: hand-written synthetic strings, labelled `illustrative` in the file.
pub const SAMPLES: &str = include_str!("../../../evals/pii.jsonl");

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    source: String,
    text: String,
    kinds: Vec<String>,
    redacted: String,
}

/// What the run found.
#[derive(Debug, Default)]
pub struct Summary {
    pub cases: usize,
    pub clean_cases: usize,
    pub kinds_seen: Vec<Kind>,
    pub mismatches: Vec<String>,
    pub leaks: usize,
}

impl Summary {
    /// Every case detected exactly, redacted exactly, idempotently, with nothing left in the
    /// redaction or the log line; clean text is present and every kind is exercised.
    pub fn contract_holds(&self) -> bool {
        self.cases > 0
            && self.clean_cases > 0
            && self.mismatches.is_empty()
            && self.leaks == 0
            && Kind::ALL.iter().all(|k| self.kinds_seen.contains(k))
    }
}

fn parse(jsonl: &str) -> Result<Vec<Case>, String> {
    let mut cases = Vec::new();
    for (n, line) in jsonl.lines().enumerate() {
        let c: Case = serde_json::from_str(line).map_err(|e| format!("line {}: {e}", n + 1))?;
        if !valid_source(&c.source) {
            return Err(format!(
                "line {}: source must be illustrative or recorded",
                n + 1
            ));
        }
        cases.push(c);
    }
    Ok(cases)
}

/// Run the bundled sample set.
pub fn run(out: &mut impl Write) -> Result<Summary, String> {
    run_on(SAMPLES, |t| redact(t).0, out)
}

/// Run any sample set through any redactor (the falsifier tests feed broken ones).
pub fn run_on(
    jsonl: &str,
    redactor: fn(&str) -> String,
    out: &mut impl Write,
) -> Result<Summary, String> {
    let cases = parse(jsonl)?;
    let mut s = Summary::default();
    for c in &cases {
        let want: Vec<Kind> = c
            .kinds
            .iter()
            .map(|k| Kind::from_label(k).ok_or_else(|| format!("{}: unknown kind {k}", c.id)))
            .collect::<Result<_, _>>()?;
        let findings = detect(&c.text);
        let got: Vec<Kind> = findings.iter().map(|f| f.kind).collect();
        let clean = redactor(&c.text);
        let again = redactor(&clean);
        let log = log_line(&c.text);
        let _ = writeln!(out, "{:<24} {:<22} {}", c.id, kinds_label(&findings), clean);
        let _ = writeln!(out, "{:<24} log: {}", "", log);
        s.cases += 1;
        s.clean_cases += usize::from(want.is_empty());
        s.kinds_seen.extend(&got);
        if got != want {
            s.mismatches
                .push(format!("{}: detected {got:?}, wanted {want:?}", c.id));
        }
        if clean != c.redacted {
            s.mismatches.push(format!(
                "{}: redacted {clean:?}, wanted {:?}",
                c.id, c.redacted
            ));
        }
        if again != clean {
            s.mismatches
                .push(format!("{}: redaction is not idempotent", c.id));
        }
        s.leaks += findings
            .iter()
            .filter(|f| {
                clean.contains(&c.text[f.start..f.end]) || log.contains(&c.text[f.start..f.end])
            })
            .count();
    }
    let _ = writeln!(
        out,
        "{} cases, {} clean, {} mismatches, {} leaks",
        s.cases,
        s.clean_cases,
        s.mismatches.len(),
        s.leaks
    );
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn go(jsonl: &str) -> Result<Summary, String> {
        run_on(jsonl, |t| redact(t).0, &mut Vec::new())
    }

    #[test]
    fn the_bundled_set_satisfies_the_contract() {
        let mut out = Vec::new();
        let s = run(&mut out).unwrap();
        assert!(s.contract_holds(), "{:?}", s.mismatches);
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("13 cases, 4 clean, 0 mismatches, 0 leaks"));
        assert!(text.contains("log: prompt chars="));
    }

    #[test]
    fn every_sample_is_labelled_illustrative() {
        assert!(SAMPLES
            .lines()
            .all(|l| l.contains("\"source\":\"illustrative\"")));
    }

    fn case(kinds: &str, redacted: &str) -> String {
        format!(
            r#"{{"id":"x","source":"illustrative","text":"mail a@b.io","kinds":{kinds},"redacted":"{redacted}"}}"#
        )
    }

    #[test]
    fn falsifier_a_missed_kind_is_a_mismatch() {
        let s = go(&case("[]", "mail [EMAIL]")).unwrap();
        assert!(!s.contract_holds());
        assert!(s.mismatches[0].contains("detected [Email], wanted []"));
    }

    #[test]
    fn falsifier_a_wrong_redaction_is_a_mismatch() {
        let s = go(&case(r#"["email"]"#, "mail a@b.io")).unwrap();
        assert!(!s.contract_holds());
        assert!(s.mismatches[0].contains("redacted"));
    }

    #[test]
    fn falsifier_a_set_missing_a_kind_or_clean_text_fails() {
        // One correct case, but five kinds and the clean text are never exercised.
        let s = go(&case(r#"["email"]"#, "mail [EMAIL]")).unwrap();
        assert!(s.mismatches.is_empty() && s.leaks == 0);
        assert!(!s.contract_holds());
        assert!(!go("").unwrap().contract_holds());
    }

    #[test]
    fn falsifier_a_leak_fails_the_contract() {
        let s = Summary {
            leaks: 1,
            ..complete()
        };
        assert!(!s.contract_holds());
        assert!(complete().contract_holds());
    }

    fn complete() -> Summary {
        Summary {
            cases: 1,
            clean_cases: 1,
            kinds_seen: Kind::ALL.to_vec(),
            mismatches: vec![],
            leaks: 0,
        }
    }

    #[test]
    fn falsifier_a_non_idempotent_redactor_is_caught() {
        let s = run_on(SAMPLES, |t| format!("{}!", redact(t).0), &mut Vec::new()).unwrap();
        assert!(!s.contract_holds());
        assert!(s.mismatches.iter().any(|m| m.contains("not idempotent")));
    }

    #[test]
    fn falsifier_a_redactor_that_does_nothing_leaks() {
        let s = run_on(SAMPLES, |t| t.to_string(), &mut Vec::new()).unwrap();
        assert!(s.leaks > 0);
        assert!(!s.contract_holds());
    }

    #[test]
    fn bad_input_is_an_error_not_a_pass() {
        assert!(go("not json").unwrap_err().starts_with("line 1:"));
        let bad_source = case("[]", "x").replace("illustrative", "measured");
        assert!(go(&bad_source).unwrap_err().contains("source must be"));
        assert!(go(&case(r#"["passport"]"#, "x"))
            .unwrap_err()
            .contains("unknown kind passport"));
    }
}
