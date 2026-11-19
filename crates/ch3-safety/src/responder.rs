//! The live responder every chapter 3 lesson shares. Offline, each lesson replays its own
//! recorded answers; `RLL_LIVE=1` asks the runtime the environment names.

use ch1_sovereign::client_for;
use rll_client::Config;
use rll_evals::ClientResponder;

/// A responder that talks to the configured runtime over HTTP.
pub fn live(cfg: &Config) -> ClientResponder {
    ClientResponder {
        client: client_for(cfg.runtime, Some(&cfg.base_url), true),
        model: cfg.model.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rll_client::Runtime;
    use rll_evals::Responder;

    #[test]
    fn live_responder_targets_the_configured_runtime() {
        let cfg = Config {
            runtime: Runtime::Ollama,
            base_url: "http://127.0.0.1:1".into(),
            model: "m".into(),
        };
        let r = live(&cfg);
        assert_eq!(r.model, "m");
        // Nothing listens on port 1, so the call fails instead of answering.
        assert!(r.answer("hi", 0, 0.0).is_err());
    }
}
