//! Error type shared by the client.

use std::fmt;

/// Everything the client can fail on.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// `RLL_RUNTIME` named something other than apr, ollama or llamacpp.
    UnknownRuntime(String),
    /// The base URL is not `http://host:port[/path]`.
    BadUrl(String),
    /// A socket read or write failed.
    Io(String),
    /// The server answered with a status other than 200.
    Http(u16),
    /// The HTTP reply could not be parsed.
    BadResponse(String),
    /// The JSON body did not match the chat schema.
    Json(String),
    /// The reply held no choices.
    EmptyChoices,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnknownRuntime(s) => write!(f, "unknown runtime: {s}"),
            Error::BadUrl(s) => write!(f, "bad url: {s}"),
            Error::Io(s) => write!(f, "io error: {s}"),
            Error::Http(c) => write!(f, "http status {c}"),
            Error::BadResponse(s) => write!(f, "bad response: {s}"),
            Error::Json(s) => write!(f, "json error: {s}"),
            Error::EmptyChoices => write!(f, "reply held no choices"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_every_variant() {
        let all = [
            (Error::UnknownRuntime("x".into()), "unknown runtime: x"),
            (Error::BadUrl("u".into()), "bad url: u"),
            (Error::Io("i".into()), "io error: i"),
            (Error::Http(500), "http status 500"),
            (Error::BadResponse("b".into()), "bad response: b"),
            (Error::Json("j".into()), "json error: j"),
            (Error::EmptyChoices, "reply held no choices"),
        ];
        for (e, text) in all {
            assert_eq!(e.to_string(), text);
        }
    }
}
