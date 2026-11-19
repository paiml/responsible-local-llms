//! Lesson 5.3: route by sensitivity. Sensitive prompts stay on the local runtime; the
//! rest may go hosted. The tests prove the route: the hosted side is a recorder, so every
//! byte that would have left the machine is on the record.
//!
//! The "hosted" transport is a STAND-IN: it makes no network call and its reply is an
//! ILLUSTRATIVE recorded body, not a measurement of any provider. The local side is the
//! `rll-client` client (recorded fixtures offline, a real runtime with `RLL_LIVE=1`).
//! The detector is a teaching heuristic, not a data-loss-prevention product, and it makes
//! no legal claim about what counts as personal data.

use rll_client::chat::{Reply, Transport};
use rll_client::fixtures::Fixture;
use rll_client::http::HttpTransport;
use rll_client::{ChatRequest, Client, Config, Error};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Local,
    Hosted,
}

/// The kinds of sensitive content found in `text`, in a fixed order.
pub fn sensitive_kinds(text: &str) -> Vec<&'static str> {
    let mut kinds = vec![];
    if has_email(text) {
        kinds.push("email");
    }
    if has_ssn_shape(text) {
        kinds.push("ssn-shape");
    }
    if has_card(text) {
        kinds.push("card-number");
    }
    let lower = text.to_lowercase();
    if ["patient", "diagnosis", "prescription"]
        .iter()
        .any(|w| lower.contains(w))
    {
        kinds.push("health");
    }
    if ["password:", "api_key", "-----begin"]
        .iter()
        .any(|w| lower.contains(w))
    {
        kinds.push("secret");
    }
    kinds
}

fn trimmed_tokens(text: &str) -> impl Iterator<Item = &str> {
    text.split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()))
}

fn has_email(text: &str) -> bool {
    trimmed_tokens(text).any(|t| match t.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && !domain.contains('@')
                && domain.contains('.')
                && !domain.ends_with('.')
        }
        None => false,
    })
}

fn has_ssn_shape(text: &str) -> bool {
    trimmed_tokens(text).any(|t| {
        let parts: Vec<&str> = t.split('-').collect();
        parts.len() == 3
            && [3, 2, 4]
                .iter()
                .zip(&parts)
                .all(|(n, p)| p.len() == *n && p.bytes().all(|b| b.is_ascii_digit()))
    })
}

/// 13 to 19 digits, single spaces or dashes allowed between digits, Luhn-valid.
fn has_card(text: &str) -> bool {
    let mut digits = String::new();
    let mut pending_sep = false;
    for c in text.chars().chain(std::iter::once('\n')) {
        if c.is_ascii_digit() {
            pending_sep = false;
            digits.push(c);
        } else if (c == ' ' || c == '-') && !digits.is_empty() && !pending_sep {
            pending_sep = true;
        } else {
            if (13..=19).contains(&digits.len()) && luhn(&digits) {
                return true;
            }
            digits.clear();
            pending_sep = false;
        }
    }
    false
}

fn luhn(digits: &str) -> bool {
    let sum: u32 = digits
        .bytes()
        .rev()
        .enumerate()
        .map(|(i, b)| {
            let d = u32::from(b - b'0');
            if i % 2 == 1 {
                let x = d * 2;
                if x > 9 {
                    x - 9
                } else {
                    x
                }
            } else {
                d
            }
        })
        .sum();
    sum.is_multiple_of(10)
}

/// Sensitive prompts always go local. Others go hosted only if the policy allows it.
pub fn decide(text: &str, hosted_allowed: bool) -> (Route, Vec<&'static str>) {
    let kinds = sensitive_kinds(text);
    let route = if kinds.is_empty() && hosted_allowed {
        Route::Hosted
    } else {
        Route::Local
    };
    (route, kinds)
}

/// Every request body "sent" to the hosted side, in order.
pub type Wire = Rc<RefCell<Vec<String>>>;

/// The hosted stand-in: records the body, answers with an ILLUSTRATIVE canned reply.
pub struct HostedRecorder {
    pub wire: Wire,
}

pub const HOSTED_REPLY: &str = r#"{"choices":[{"message":{"content":"[illustrative hosted reply]"}}],"usage":{"completion_tokens":4}}"#;

impl Transport for HostedRecorder {
    fn post_json(&self, _url: &str, body: &str) -> Result<Reply, Error> {
        self.wire.borrow_mut().push(body.to_string());
        Ok(Reply {
            body: HOSTED_REPLY.to_string(),
            elapsed: Duration::from_millis(1),
        })
    }
}

pub struct Router {
    local: Client,
    hosted: Client,
    pub hosted_allowed: bool,
    model: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub route: Route,
    pub kinds: Vec<&'static str>,
    pub content: String,
}

impl Router {
    pub fn new(local: Client, wire: Wire, hosted_allowed: bool, model: &str) -> Router {
        let hosted = Client::new(
            local.runtime,
            "https://hosted.invalid",
            Box::new(HostedRecorder { wire }),
        );
        Router {
            local,
            hosted,
            hosted_allowed,
            model: model.to_string(),
        }
    }

    /// Route and answer. A failed local call is an error: a sensitive prompt is never
    /// retried on the hosted side.
    pub fn ask(&self, text: &str) -> Result<Answer, Error> {
        let (route, kinds) = decide(text, self.hosted_allowed);
        let client = match route {
            Route::Local => &self.local,
            Route::Hosted => &self.hosted,
        };
        let c = client.chat(&ChatRequest::new(&self.model, text))?;
        Ok(Answer {
            route,
            kinds,
            content: c.content,
        })
    }
}

/// A local client: recorded replies offline, a real runtime when `live`.
pub fn local_client(cfg: &Config, live: bool) -> Client {
    let transport: Box<dyn Transport> = if live {
        Box::new(HttpTransport)
    } else {
        Box::new(Fixture::for_runtime(cfg.runtime))
    };
    Client::new(cfg.runtime, &cfg.base_url, transport)
}

/// The recorded corpus: (text, expected route).
pub fn corpus() -> Vec<(String, Route)> {
    include_str!("../fixtures/prompts.jsonl")
        .lines()
        .map(|l| {
            let v: serde_json::Value =
                serde_json::from_str(l).expect("fixtures/prompts.jsonl is valid");
            let route = if v["expect"] == "local" {
                Route::Local
            } else {
                Route::Hosted
            };
            (v["text"].as_str().expect("text").to_string(), route)
        })
        .collect()
}

pub struct Summary {
    pub rows: Vec<(String, Route, Route)>,
    pub wire: Vec<String>,
}

impl Summary {
    /// Every prompt took its expected route, no prompt expected local ever appears on the
    /// hosted wire, the wire holds exactly the hosted routes, and both routes occur.
    pub fn contract_holds(&self) -> bool {
        let routed = self.rows.iter().all(|(_, want, got)| want == got);
        let no_leak = self
            .rows
            .iter()
            .filter(|(_, want, _)| *want == Route::Local)
            .all(|(text, _, _)| !self.wire.iter().any(|w| w.contains(&escape(text))));
        let hosted_n = self
            .rows
            .iter()
            .filter(|(_, _, got)| *got == Route::Hosted)
            .count();
        let both = hosted_n > 0 && hosted_n < self.rows.len();
        routed && no_leak && self.wire.len() == hosted_n && both
    }
}

/// The text as it appears inside a JSON string body.
fn escape(text: &str) -> String {
    let quoted = serde_json::Value::String(text.to_string()).to_string();
    quoted[1..quoted.len() - 1].to_string()
}

/// Send the whole corpus through a router with hosted routing allowed.
pub fn run(cfg: &Config, live: bool, out: &mut dyn std::io::Write) -> Result<Summary, Error> {
    let wire: Wire = Rc::default();
    let router = Router::new(local_client(cfg, live), wire.clone(), true, &cfg.model);
    let mut rows = vec![];
    for (text, want) in corpus() {
        let a = router.ask(&text)?;
        writeln!(out, "{:?} {:?}: {}", a.route, a.kinds, text).ok();
        rows.push((text, want, a.route));
    }
    let wire = wire.borrow().clone();
    writeln!(out, "bodies sent to the hosted stand-in: {}", wire.len()).ok();
    Ok(Summary { rows, wire })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rll_client::Runtime;

    fn cfg() -> Config {
        Config::from_lookup(|_| None).unwrap()
    }

    fn router(allowed: bool) -> (Router, Wire) {
        let wire: Wire = Rc::default();
        (
            Router::new(local_client(&cfg(), false), wire.clone(), allowed, "m"),
            wire,
        )
    }

    #[test]
    fn detects_each_kind() {
        assert_eq!(sensitive_kinds("mail a.b@example.com now"), ["email"]);
        assert_eq!(sensitive_kinds("id 000-12-3456."), ["ssn-shape"]);
        assert_eq!(
            sensitive_kinds("card 4111-1111-1111-1111 ok"),
            ["card-number"]
        );
        assert_eq!(sensitive_kinds("The PATIENT is fine"), ["health"]);
        assert_eq!(sensitive_kinds("API_KEY=abc"), ["secret"]);
        assert!(sensitive_kinds("what is a GGUF file").is_empty());
    }

    #[test]
    fn near_misses_are_not_flagged() {
        assert!(sensitive_kinds("a@b").is_empty());
        assert!(sensitive_kinds("@example.com").is_empty());
        assert!(sensitive_kinds("a@b.").is_empty());
        assert!(sensitive_kinds("a@b@c.com").is_empty());
        assert!(sensitive_kinds("12-34-5678").is_empty());
        assert!(sensitive_kinds("000-1x-3456").is_empty());
        assert!(sensitive_kinds("0000-12-3456").is_empty());
        assert!(sensitive_kinds("4111 1111 1111 1112").is_empty());
        assert!(sensitive_kinds("order 1234567890123").is_empty());
        assert!(sensitive_kinds("41111  1111 1111 111").is_empty());
        assert!(sensitive_kinds("-").is_empty());
    }

    #[test]
    fn a_card_number_needs_thirteen_to_nineteen_digits() {
        assert!(has_card("4111111111111111"));
        assert!(!has_card("411111111111")); // 12 digits
        assert!(!has_card("41111111111111111111")); // 20 digits
        assert!(luhn("4111111111111111"));
        assert!(!luhn("4111111111111112"));
    }

    #[test]
    fn decide_keeps_sensitive_local_whatever_the_policy() {
        assert_eq!(decide("mail a@b.co", true).0, Route::Local);
        assert_eq!(decide("plain question", true).0, Route::Hosted);
        assert_eq!(decide("plain question", false).0, Route::Local);
    }

    #[test]
    fn hosted_wire_never_sees_a_sensitive_prompt() {
        let (r, wire) = router(true);
        let a = r.ask("Refund card 4111 1111 1111 1111").unwrap();
        assert_eq!((a.route, a.kinds), (Route::Local, vec!["card-number"]));
        assert!(wire.borrow().is_empty());
        let a = r.ask("what is a GGUF file").unwrap();
        assert_eq!(a.route, Route::Hosted);
        assert_eq!(a.content, "[illustrative hosted reply]");
        assert_eq!(wire.borrow().len(), 1);
    }

    #[test]
    fn with_hosting_off_everything_stays_local() {
        let (r, wire) = router(false);
        assert_eq!(r.ask("what is a GGUF file").unwrap().route, Route::Local);
        assert!(wire.borrow().is_empty());
    }

    struct Down;
    impl Transport for Down {
        fn post_json(&self, _: &str, _: &str) -> Result<Reply, Error> {
            Err(Error::Io("local runtime down".into()))
        }
    }

    #[test]
    fn a_dead_local_runtime_fails_closed_and_never_falls_back_to_hosted() {
        let wire: Wire = Rc::default();
        let local = Client::new(Runtime::Apr, "http://127.0.0.1:1", Box::new(Down));
        let r = Router::new(local, wire.clone(), true, "m");
        assert_eq!(
            r.ask("patient notes").unwrap_err(),
            Error::Io("local runtime down".into())
        );
        assert!(wire.borrow().is_empty());
    }

    #[test]
    fn the_recorded_corpus_satisfies_the_contract() {
        let mut out = Vec::new();
        let s = run(&cfg(), false, &mut out).unwrap();
        assert!(s.contract_holds());
        assert_eq!(s.rows.len(), 8);
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("Local [\"card-number\"]"));
        assert!(text.contains("bodies sent to the hosted stand-in: 3"));
    }

    #[test]
    fn a_failing_local_runtime_fails_the_run() {
        let mut c = cfg();
        c.base_url = "http://127.0.0.1:1".into();
        assert!(run(&c, true, &mut Vec::new()).is_err());
    }

    fn summary() -> Summary {
        run(&cfg(), false, &mut Vec::new()).unwrap()
    }

    #[test]
    fn contract_fails_when_a_sensitive_prompt_reaches_the_wire() {
        let mut s = summary();
        let leaked = s
            .rows
            .iter()
            .find(|(_, w, _)| *w == Route::Local)
            .unwrap()
            .0
            .clone();
        s.wire
            .push(format!("{{\"content\":\"{}\"}}", escape(&leaked)));
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_when_a_prompt_takes_the_wrong_route() {
        let mut s = summary();
        s.rows[0].2 = Route::Local;
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_when_the_wire_and_routes_disagree_or_one_route_is_unused() {
        let mut s = summary();
        s.wire.pop();
        assert!(!s.contract_holds());
        let mut s = summary();
        s.rows.retain(|(_, w, _)| *w == Route::Local);
        s.wire.clear();
        assert!(!s.contract_holds());
    }

    #[test]
    fn escape_matches_json_encoding() {
        assert_eq!(escape("a\"b\n"), "a\\\"b\\n");
    }
}
