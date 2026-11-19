//! Chapter 1 demo logic. Each lesson is a module with a `run` function; the
//! demo bins in `src/bin/` call it, assert the contract and print the verdict.

pub mod economics;
pub mod quality;
pub mod quantize;
pub mod three_runtimes;

use rll_client::fixtures::Fixture;
use rll_client::http::HttpTransport;
use rll_client::{Client, Runtime, Transport};

/// A client for one runtime: recorded fixtures offline, real HTTP when `live`.
pub fn client_for(rt: Runtime, base_url: Option<&str>, live: bool) -> Client {
    let transport: Box<dyn Transport> = if live {
        Box::new(HttpTransport)
    } else {
        Box::new(Fixture::for_runtime(rt))
    };
    Client::new(rt, base_url.unwrap_or(rt.default_base_url()), transport)
}

/// `RLL_LIVE=1` switches the demos from fixtures to real runtimes.
pub fn live_from(value: Option<String>) -> bool {
    value.as_deref() == Some("1")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rll_client::ChatRequest;

    #[test]
    fn live_flag_parses() {
        assert!(live_from(Some("1".into())));
        assert!(!live_from(Some("0".into())));
        assert!(!live_from(None));
    }

    #[test]
    fn offline_client_answers_from_fixture() {
        let c = client_for(Runtime::Ollama, None, false);
        let r = c.chat(&ChatRequest::new("m", "p")).unwrap();
        assert!(r.content.contains("Paris"));
        assert_eq!(c.base_url, Runtime::Ollama.default_base_url());
    }

    #[test]
    fn live_client_keeps_its_base_url() {
        let c = client_for(Runtime::Apr, Some("http://h:9"), true);
        assert_eq!(c.endpoint(), "http://h:9/v1/chat/completions");
    }
}
