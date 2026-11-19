//! Recorded replies, so every test and demo runs offline.

use crate::chat::{Reply, Transport};
use crate::error::Error;
use crate::runtime::Runtime;
use std::time::Duration;

/// A transport that always returns one recorded body.
pub struct Fixture {
    body: String,
    elapsed: Duration,
}

impl Fixture {
    pub fn new(body: &str, elapsed: Duration) -> Fixture {
        Fixture {
            body: body.to_string(),
            elapsed,
        }
    }

    /// The recording for a runtime, with the wall time it was captured at.
    pub fn for_runtime(rt: Runtime) -> Fixture {
        match rt {
            Runtime::Apr => Fixture::new(
                include_str!("../fixtures/apr.chat.json"),
                Duration::from_millis(500),
            ),
            Runtime::Ollama => Fixture::new(
                include_str!("../fixtures/ollama.chat.json"),
                Duration::from_millis(800),
            ),
            Runtime::LlamaCpp => Fixture::new(
                include_str!("../fixtures/llamacpp.chat.json"),
                Duration::from_millis(650),
            ),
        }
    }
}

impl Transport for Fixture {
    fn post_json(&self, _url: &str, _body: &str) -> Result<Reply, Error> {
        Ok(Reply {
            body: self.body.clone(),
            elapsed: self.elapsed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_runtime_has_a_valid_recording() {
        for rt in Runtime::ALL {
            let r = Fixture::for_runtime(rt).post_json("u", "{}").unwrap();
            let v: serde_json::Value = serde_json::from_str(&r.body).unwrap();
            assert!(v["choices"][0]["message"]["content"].is_string());
            assert!(r.elapsed > Duration::ZERO);
        }
    }
}
