//! The eval-set schema: one JSON object per line.
//!
//! ```json
//! {"id":"capital-france","prompt":"What is the capital of France?","properties":[{"kind":"contains","value":"Paris"},{"kind":"max_words","n":20}]}
//! ```

use serde::Deserialize;
use std::collections::BTreeSet;
use std::fmt;

/// One property an answer must have.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Property {
    /// The answer contains `value`, ignoring case.
    Contains { value: String },
    /// The answer does not contain `value`, ignoring case.
    NotContains { value: String },
    /// The answer has at most `n` words.
    MaxWords { n: u32 },
    /// The whole answer, trimmed of spaces and a final full stop, is one of `values` (ignoring case).
    OneOf { values: Vec<String> },
    /// The answer parses as JSON.
    IsJson {},
}

/// One test case: a prompt and the properties its answer must have.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    pub prompt: String,
    pub properties: Vec<Property>,
}

/// A schema failure, with the 1-based line it came from (0 for a whole-set failure).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError {
    pub line: usize,
    pub reason: String,
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "eval set: {}", self.reason)
        } else {
            write!(f, "eval set line {}: {}", self.line, self.reason)
        }
    }
}

impl std::error::Error for SchemaError {}

impl Property {
    fn validate(&self) -> Result<(), String> {
        match self {
            Property::Contains { value } | Property::NotContains { value } => {
                if value.trim().is_empty() {
                    return Err("contains/not_contains needs a non-empty value".into());
                }
            }
            Property::MaxWords { n } => {
                if *n == 0 {
                    return Err("max_words needs n >= 1".into());
                }
            }
            Property::OneOf { values } => {
                if values.is_empty() || values.iter().any(|v| v.trim().is_empty()) {
                    return Err("one_of needs at least one non-empty value".into());
                }
            }
            Property::IsJson {} => {}
        }
        Ok(())
    }
}

impl Case {
    /// A case needs an id of `[a-z0-9_-]`, a prompt, and at least one valid property.
    pub fn validate(&self) -> Result<(), String> {
        let id_ok = !self.id.is_empty()
            && self
                .id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
        if !id_ok {
            return Err(format!("id {:?} must be non-empty [a-z0-9_-]", self.id));
        }
        if self.prompt.trim().is_empty() {
            return Err("prompt is empty".into());
        }
        if self.properties.is_empty() {
            return Err("a case with no properties tests nothing".into());
        }
        self.properties.iter().try_for_each(Property::validate)
    }
}

/// Parse and validate a jsonl eval set. Blank lines are skipped; ids must be unique and the
/// set must hold at least one case.
pub fn parse_jsonl(text: &str) -> Result<Vec<Case>, SchemaError> {
    let mut cases = Vec::new();
    let mut seen = BTreeSet::new();
    for (i, raw) in text.lines().enumerate() {
        let line = i + 1;
        if raw.trim().is_empty() {
            continue;
        }
        let err = |reason: String| SchemaError { line, reason };
        let case: Case = serde_json::from_str(raw).map_err(|e| err(e.to_string()))?;
        case.validate().map_err(err)?;
        if !seen.insert(case.id.clone()) {
            return Err(err(format!("duplicate id {:?}", case.id)));
        }
        cases.push(case);
    }
    if cases.is_empty() {
        return Err(SchemaError {
            line: 0,
            reason: "no cases".into(),
        });
    }
    Ok(cases)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{"id":"a","prompt":"p","properties":[{"kind":"contains","value":"x"},{"kind":"not_contains","value":"y"},{"kind":"max_words","n":3},{"kind":"one_of","values":["a","b"]},{"kind":"is_json"}]}"#;

    fn err_of(text: &str) -> SchemaError {
        parse_jsonl(text).unwrap_err()
    }

    #[test]
    fn parses_every_property_kind() {
        let cases = parse_jsonl(GOOD).unwrap();
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].properties.len(), 5);
    }

    #[test]
    fn blank_lines_are_skipped_and_line_numbers_count_them() {
        let text = format!("\n{GOOD}\n\n{{\"id\":\"b\"}}\n");
        let e = err_of(&text);
        assert_eq!(e.line, 4);
        assert!(e.to_string().starts_with("eval set line 4: "));
    }

    #[test]
    fn malformed_json_names_its_line() {
        assert_eq!(err_of("not json").line, 1);
    }

    #[test]
    fn unknown_fields_and_kinds_are_refused() {
        assert!(parse_jsonl(
            r#"{"id":"a","prompt":"p","properties":[{"kind":"is_json"}],"extra":1}"#
        )
        .is_err());
        assert!(parse_jsonl(r#"{"id":"a","prompt":"p","properties":[{"kind":"vibes"}]}"#).is_err());
        assert!(parse_jsonl(
            r#"{"id":"a","prompt":"p","properties":[{"kind":"max_words","n":2,"x":1}]}"#
        )
        .is_err());
    }

    #[test]
    fn an_empty_set_is_refused() {
        let e = err_of("\n  \n");
        assert_eq!((e.line, e.reason.as_str()), (0, "no cases"));
        assert_eq!(e.to_string(), "eval set: no cases");
    }

    #[test]
    fn duplicate_ids_are_refused() {
        let e = err_of(&format!("{GOOD}\n{GOOD}"));
        assert_eq!(e.line, 2);
        assert!(e.reason.contains("duplicate id"));
    }

    #[test]
    fn bad_ids_prompts_and_empty_property_lists_are_refused() {
        for bad in [
            r#"{"id":"","prompt":"p","properties":[{"kind":"is_json"}]}"#,
            r#"{"id":"Has Caps","prompt":"p","properties":[{"kind":"is_json"}]}"#,
            r#"{"id":"a","prompt":"  ","properties":[{"kind":"is_json"}]}"#,
            r#"{"id":"a","prompt":"p","properties":[]}"#,
        ] {
            assert_eq!(err_of(bad).line, 1, "{bad}");
        }
    }

    #[test]
    fn bad_property_values_are_refused() {
        for bad in [
            r#"{"kind":"contains","value":" "}"#,
            r#"{"kind":"not_contains","value":""}"#,
            r#"{"kind":"max_words","n":0}"#,
            r#"{"kind":"one_of","values":[]}"#,
            r#"{"kind":"one_of","values":["a",""]}"#,
        ] {
            let line = format!(r#"{{"id":"a","prompt":"p","properties":[{bad}]}}"#);
            assert_eq!(err_of(&line).line, 1, "{bad}");
        }
    }
}
