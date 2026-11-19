//! One OpenAI-compatible chat client for three local runtimes.
//!
//! `RLL_RUNTIME=apr|ollama|llamacpp` picks the runtime and `RLL_BASE_URL`
//! overrides where it listens. Tests run offline against recorded fixtures.

pub mod chat;
pub mod error;
pub mod fixtures;
pub mod http;
pub mod runtime;

pub use chat::{direct, ChatRequest, Client, Completion, Reply, Transport};
pub use error::Error;
pub use runtime::{Config, Runtime};
