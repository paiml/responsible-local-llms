//! The property checker: judge one answer against a case.

use crate::schema::{Case, Property};

impl Property {
    /// `Ok(())` when the answer has this property, else a one-line reason.
    pub fn check(&self, answer: &str) -> Result<(), String> {
        let lower = answer.to_lowercase();
        match self {
            Property::Contains { value } => {
                if lower.contains(&value.to_lowercase()) {
                    Ok(())
                } else {
                    Err(format!("missing {value:?}"))
                }
            }
            Property::NotContains { value } => {
                if lower.contains(&value.to_lowercase()) {
                    Err(format!("contains forbidden {value:?}"))
                } else {
                    Ok(())
                }
            }
            Property::MaxWords { n } => {
                let words = answer.split_whitespace().count();
                if words <= *n as usize {
                    Ok(())
                } else {
                    Err(format!("{words} words, limit {n}"))
                }
            }
            Property::OneOf { values } => {
                let t = lower.trim().trim_end_matches('.').trim();
                if values.iter().any(|v| v.to_lowercase() == t) {
                    Ok(())
                } else {
                    Err(format!("{t:?} is not one of {values:?}"))
                }
            }
            Property::IsJson {} => serde_json::from_str::<serde_json::Value>(answer.trim())
                .map(|_| ())
                .map_err(|e| format!("not JSON: {e}")),
        }
    }
}

impl Case {
    /// Every failed property, in order. Empty means the answer passes the case.
    pub fn check(&self, answer: &str) -> Vec<String> {
        self.properties
            .iter()
            .filter_map(|p| p.check(answer).err())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(json: &str) -> Property {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn contains_ignores_case_and_wording() {
        let c = p(r#"{"kind":"contains","value":"Paris"}"#);
        assert!(c.check("The capital is PARIS, of course.").is_ok());
        assert_eq!(c.check("Lyon").unwrap_err(), "missing \"Paris\"");
    }

    #[test]
    fn not_contains_refuses_the_forbidden_word() {
        let c = p(r#"{"kind":"not_contains","value":"Lyon"}"#);
        assert!(c.check("Paris.").is_ok());
        assert_eq!(
            c.check("Not lyon, Paris").unwrap_err(),
            "contains forbidden \"Lyon\""
        );
    }

    #[test]
    fn max_words_counts_whitespace_separated_words() {
        let c = p(r#"{"kind":"max_words","n":3}"#);
        assert!(c.check("one two  three").is_ok());
        assert_eq!(
            c.check("one two three four").unwrap_err(),
            "4 words, limit 3"
        );
        assert!(c.check("").is_ok());
    }

    #[test]
    fn one_of_trims_a_final_full_stop() {
        let c = p(r#"{"kind":"one_of","values":["42","forty-two"]}"#);
        assert!(c.check(" 42. ").is_ok());
        assert!(c.check("Forty-Two").is_ok());
        assert!(c.check("The answer is 42").is_err());
    }

    #[test]
    fn is_json_accepts_only_json() {
        let c = p(r#"{"kind":"is_json"}"#);
        assert!(c.check(r#" {"a":1} "#).is_ok());
        assert!(c.check("{oops").unwrap_err().starts_with("not JSON"));
    }

    #[test]
    fn a_case_reports_every_failed_property() {
        let case = crate::parse_jsonl(
            r#"{"id":"a","prompt":"p","properties":[{"kind":"contains","value":"x"},{"kind":"max_words","n":1},{"kind":"is_json"}]}"#,
        )
        .unwrap()
        .remove(0);
        assert_eq!(case.check("x").len(), 1); // not JSON
        assert_eq!(case.check("y z").len(), 3);
        assert!(case.check("\"x\"").is_empty());
    }
}
