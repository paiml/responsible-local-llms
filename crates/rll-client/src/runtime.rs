//! Runtime selection: which server, where, and which model.

use crate::error::Error;

/// The three runtimes the course compares on the same GGUF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Runtime {
    Apr,
    Ollama,
    LlamaCpp,
}

impl Runtime {
    /// Every runtime, in the order the course introduces them.
    pub const ALL: [Runtime; 3] = [Runtime::Apr, Runtime::Ollama, Runtime::LlamaCpp];

    /// The value `RLL_RUNTIME` takes for this runtime.
    pub fn name(self) -> &'static str {
        match self {
            Runtime::Apr => "apr",
            Runtime::Ollama => "ollama",
            Runtime::LlamaCpp => "llamacpp",
        }
    }

    /// Where `scripts/serve-*.sh` starts this runtime.
    pub fn default_base_url(self) -> &'static str {
        match self {
            Runtime::Apr => "http://127.0.0.1:8081",
            Runtime::Ollama => "http://127.0.0.1:11434",
            Runtime::LlamaCpp => "http://127.0.0.1:8080",
        }
    }

    /// Parse an `RLL_RUNTIME` value.
    pub fn parse(s: &str) -> Result<Runtime, Error> {
        Runtime::ALL
            .into_iter()
            .find(|r| r.name() == s)
            .ok_or_else(|| Error::UnknownRuntime(s.to_string()))
    }
}

/// Client settings read from the environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub runtime: Runtime,
    pub base_url: String,
    pub model: String,
}

impl Config {
    /// Build from any key lookup, so tests need no process environment.
    pub fn from_lookup<F: Fn(&str) -> Option<String>>(lookup: F) -> Result<Config, Error> {
        let runtime = match lookup("RLL_RUNTIME") {
            Some(v) => Runtime::parse(&v)?,
            None => Runtime::Apr,
        };
        let base_url =
            lookup("RLL_BASE_URL").unwrap_or_else(|| runtime.default_base_url().to_string());
        let model = lookup("RLL_MODEL").unwrap_or_else(|| "qwen3-0.6b".to_string());
        Ok(Config {
            runtime,
            base_url,
            model,
        })
    }

    /// Build from the process environment.
    pub fn from_env() -> Result<Config, Error> {
        Config::from_lookup(|k| std::env::var(k).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips_every_runtime() {
        for r in Runtime::ALL {
            assert_eq!(Runtime::parse(r.name()), Ok(r));
        }
    }

    #[test]
    fn parse_rejects_unknown() {
        assert_eq!(
            Runtime::parse("vllm"),
            Err(Error::UnknownRuntime("vllm".into()))
        );
    }

    #[test]
    fn default_urls_are_distinct_ports() {
        let urls: Vec<_> = Runtime::ALL.iter().map(|r| r.default_base_url()).collect();
        assert_eq!(urls.len(), 3);
        assert!(urls[0] != urls[1] && urls[1] != urls[2] && urls[0] != urls[2]);
    }

    #[test]
    fn config_defaults_to_apr() {
        let c = Config::from_lookup(|_| None).unwrap();
        assert_eq!(c.runtime, Runtime::Apr);
        assert_eq!(c.base_url, Runtime::Apr.default_base_url());
        assert_eq!(c.model, "qwen3-0.6b");
    }

    #[test]
    fn config_reads_overrides() {
        let c = Config::from_lookup(|k| match k {
            "RLL_RUNTIME" => Some("ollama".into()),
            "RLL_BASE_URL" => Some("http://h:1".into()),
            "RLL_MODEL" => Some("m".into()),
            _ => None,
        })
        .unwrap();
        assert_eq!(c.runtime, Runtime::Ollama);
        assert_eq!(c.base_url, "http://h:1");
        assert_eq!(c.model, "m");
    }

    #[test]
    fn config_rejects_bad_runtime() {
        let r = Config::from_lookup(|k| (k == "RLL_RUNTIME").then(|| "nope".to_string()));
        assert_eq!(r, Err(Error::UnknownRuntime("nope".into())));
    }

    #[test]
    fn from_env_runs() {
        // Result depends on the caller's environment; the call itself must not panic.
        let _ = Config::from_env();
    }
}
