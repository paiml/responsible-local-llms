//! One error type for the chapter's demos.

use rll_client::Error;
use std::fmt;

/// Why a safety demo could not produce a measurement.
#[derive(Debug, PartialEq, Eq)]
pub enum SafetyError {
    /// The eval set does not have the shape the lesson needs.
    Shape(String),
    /// The model call failed.
    Model(Error),
}

impl fmt::Display for SafetyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SafetyError::Shape(m) => write!(f, "eval set shape: {m}"),
            SafetyError::Model(e) => write!(f, "model call failed: {e}"),
        }
    }
}

impl std::error::Error for SafetyError {}

impl From<Error> for SafetyError {
    fn from(e: Error) -> SafetyError {
        SafetyError::Model(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_say_what_failed() {
        let shape = SafetyError::Shape("no probes".into());
        assert_eq!(shape.to_string(), "eval set shape: no probes");
        let model: SafetyError = Error::BadResponse("boom".into()).into();
        assert!(model.to_string().starts_with("model call failed: "));
        assert!(matches!(model, SafetyError::Model(_)));
    }
}
