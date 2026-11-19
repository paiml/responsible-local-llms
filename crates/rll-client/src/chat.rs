//! OpenAI-compatible chat completions over a pluggable transport.

use crate::error::Error;
use crate::runtime::Runtime;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// A deterministic request: temperature 0 and a fixed seed.
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: f32,
    pub seed: u32,
    pub max_tokens: u32,
    pub stream: bool,
}

impl ChatRequest {
    pub fn new(model: &str, prompt: &str) -> ChatRequest {
        ChatRequest {
            model: model.to_string(),
            messages: vec![Message {
                role: "user".into(),
                content: prompt.into(),
            }],
            temperature: 0.0,
            seed: 7,
            max_tokens: 64,
            stream: false,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: RespMessage,
}

#[derive(Debug, Deserialize)]
struct RespMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    completion_tokens: u32,
}

/// What a transport hands back: the body and how long the call took.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub body: String,
    pub elapsed: Duration,
}

/// Sends a JSON body to a URL and returns the reply body.
pub trait Transport {
    fn post_json(&self, url: &str, body: &str) -> Result<Reply, Error>;
}

/// A parsed answer plus its measured speed.
#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    pub content: String,
    pub completion_tokens: u32,
    pub elapsed: Duration,
}

impl Completion {
    /// Generated tokens per second of wall time.
    pub fn tokens_per_second(&self) -> f64 {
        f64::from(self.completion_tokens) / self.elapsed.as_secs_f64()
    }
}

/// One client for every runtime.
pub struct Client {
    pub runtime: Runtime,
    pub base_url: String,
    transport: Box<dyn Transport>,
}

impl Client {
    pub fn new(runtime: Runtime, base_url: &str, transport: Box<dyn Transport>) -> Client {
        Client {
            runtime,
            base_url: base_url.trim_end_matches('/').to_string(),
            transport,
        }
    }

    /// The chat endpoint all three runtimes share.
    pub fn endpoint(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url)
    }

    pub fn chat(&self, req: &ChatRequest) -> Result<Completion, Error> {
        let body = serde_json::to_string(req).expect("ChatRequest always serializes");
        let reply = self.transport.post_json(&self.endpoint(), &body)?;
        let parsed: ChatResponse =
            serde_json::from_str(&reply.body).map_err(|e| Error::Json(e.to_string()))?;
        let choice = parsed
            .choices
            .into_iter()
            .next()
            .ok_or(Error::EmptyChoices)?;
        let content = choice.message.content;
        let completion_tokens = match parsed.usage {
            Some(u) => u.completion_tokens,
            None => content.split_whitespace().count() as u32,
        };
        Ok(Completion {
            content,
            completion_tokens,
            elapsed: reply.elapsed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::Fixture;

    fn client(body: &str) -> Client {
        let f = Fixture::new(body, Duration::from_millis(500));
        Client::new(Runtime::Apr, "http://x:1/", Box::new(f))
    }

    #[test]
    fn request_is_deterministic() {
        let r = ChatRequest::new("m", "hi");
        assert_eq!((r.temperature, r.seed, r.stream), (0.0, 7, false));
        assert_eq!(r.messages[0].role, "user");
    }

    #[test]
    fn endpoint_trims_trailing_slash() {
        assert_eq!(client("{}").endpoint(), "http://x:1/v1/chat/completions");
    }

    #[test]
    fn chat_uses_reported_usage() {
        let c = client(
            r#"{"choices":[{"message":{"content":"a b"}}],"usage":{"completion_tokens":10}}"#,
        )
        .chat(&ChatRequest::new("m", "p"))
        .unwrap();
        assert_eq!(c.completion_tokens, 10);
        assert_eq!(c.content, "a b");
        assert!((c.tokens_per_second() - 20.0).abs() < 1e-9);
    }

    #[test]
    fn chat_counts_words_without_usage() {
        let c = client(r#"{"choices":[{"message":{"content":"one two three"}}]}"#)
            .chat(&ChatRequest::new("m", "p"))
            .unwrap();
        assert_eq!(c.completion_tokens, 3);
    }

    #[test]
    fn chat_rejects_empty_choices() {
        let e = client(r#"{"choices":[]}"#)
            .chat(&ChatRequest::new("m", "p"))
            .unwrap_err();
        assert_eq!(e, Error::EmptyChoices);
    }

    #[test]
    fn chat_rejects_bad_json() {
        let e = client("not json")
            .chat(&ChatRequest::new("m", "p"))
            .unwrap_err();
        assert!(matches!(e, Error::Json(_)));
    }
}
