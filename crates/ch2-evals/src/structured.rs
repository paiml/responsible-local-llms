//! Lesson 2.3: structured output you can trust. A reply is accepted only if it parses as JSON
//! and validates against the schema; a refusal, prose or a wrong value is sent back with the
//! reasons, and the retries are bounded.

use crate::replay::{field, rows, Replay};
use ch1_sovereign::client_for;
use rll_client::Config;
use rll_evals::{ClientResponder, Responder, RunError};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::io::Write;

/// The JSON schema every reply must satisfy (`evals/ticket.schema.json`).
pub const SCHEMA: &str = include_str!("../../../evals/ticket.schema.json");
/// The messages to turn into tickets (`evals/structured.jsonl`).
pub const SET: &str = include_str!("../../../evals/structured.jsonl");
/// Illustrative model replies, in attempt order.
pub const FIXTURE: &str = include_str!("../fixtures/structured.jsonl");
/// One first try plus two retries.
pub const MAX_ATTEMPTS: u32 = 3;
/// What the offline recordings do per case: attempts used, `None` when the retries run out.
pub const EXPECTED: [(&str, Option<usize>); 5] = [
    ("clean", Some(1)),
    ("refusal-then-ok", Some(2)),
    ("prose-wrapped", Some(2)),
    ("bad-enum", Some(2)),
    ("always-refuses", None),
];

const KEYWORDS: [&str; 9] = [
    "type",
    "enum",
    "required",
    "properties",
    "additionalProperties",
    "minimum",
    "maximum",
    "minLength",
    "items",
];

/// Validate `value` against the JSON-schema subset the lesson uses. Returns every problem;
/// empty means valid. A keyword this validator does not know is a problem too, so a schema
/// it cannot enforce fails closed instead of passing everything.
pub fn validate(schema: &Value, value: &Value) -> Vec<String> {
    let mut errs = Vec::new();
    check(schema, value, "$", &mut errs);
    errs
}

fn type_ok(want: &str, v: &Value) -> bool {
    match want {
        "object" => v.is_object(),
        "array" => v.is_array(),
        "string" => v.is_string(),
        "integer" => v.is_i64() || v.is_u64(),
        "number" => v.is_number(),
        "boolean" => v.is_boolean(),
        "null" => v.is_null(),
        _ => false,
    }
}

fn check(schema: &Value, v: &Value, path: &str, errs: &mut Vec<String>) {
    let Some(map) = schema.as_object() else {
        errs.push(format!("{path}: schema is not an object"));
        return;
    };
    for key in map.keys() {
        if !KEYWORDS.contains(&key.as_str()) {
            errs.push(format!("{path}: unsupported schema keyword {key:?}"));
        }
    }
    if let Some(t) = map.get("type").and_then(Value::as_str) {
        if !type_ok(t, v) {
            errs.push(format!("{path}: expected {t}"));
            return;
        }
    }
    check_values(map, v, path, errs);
    if let (Some(item), Some(list)) = (map.get("items"), v.as_array()) {
        for (i, x) in list.iter().enumerate() {
            check(item, x, &format!("{path}[{i}]"), errs);
        }
    }
    if let Some(obj) = v.as_object() {
        check_object(map, obj, path, errs);
    }
}

/// The enum, minimum, maximum and minLength keywords.
fn check_values(map: &Map<String, Value>, v: &Value, path: &str, errs: &mut Vec<String>) {
    if let Some(options) = map.get("enum").and_then(Value::as_array) {
        if !options.contains(v) {
            errs.push(format!(
                "{path}: {v} is not one of {}",
                Value::Array(options.clone())
            ));
        }
    }
    if let (Some(min), Some(n)) = (map.get("minimum").and_then(Value::as_f64), v.as_f64()) {
        if n < min {
            errs.push(format!("{path}: {n} is below the minimum {min}"));
        }
    }
    if let (Some(max), Some(n)) = (map.get("maximum").and_then(Value::as_f64), v.as_f64()) {
        if n > max {
            errs.push(format!("{path}: {n} is above the maximum {max}"));
        }
    }
    if let (Some(min), Some(s)) = (map.get("minLength").and_then(Value::as_u64), v.as_str()) {
        if (s.chars().count() as u64) < min {
            errs.push(format!("{path}: shorter than {min} characters"));
        }
    }
}

/// The required, properties and additionalProperties keywords.
fn check_object(
    map: &Map<String, Value>,
    obj: &Map<String, Value>,
    path: &str,
    errs: &mut Vec<String>,
) {
    for name in map
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let name = name.as_str().unwrap_or_default();
        if !obj.contains_key(name) {
            errs.push(format!("{path}: missing required {name:?}"));
        }
    }
    let props = map.get("properties").and_then(Value::as_object);
    for (k, x) in obj {
        match props.and_then(|p| p.get(k)) {
            Some(sub) => check(sub, x, &format!("{path}.{k}"), errs),
            None if map.get("additionalProperties") == Some(&Value::Bool(false)) => {
                errs.push(format!("{path}: unexpected property {k:?}"))
            }
            None => {}
        }
    }
}

/// Pull one JSON value out of a reply, tolerating a ```json fence. Prose or a refusal is an error.
pub fn extract_json(reply: &str) -> Result<Value, String> {
    let t = reply.trim();
    let body = match t.strip_prefix("```") {
        Some(rest) => {
            let rest = rest.split_once('\n').map_or("", |(_, r)| r);
            rest.trim_end().trim_end_matches("```").trim()
        }
        None => t,
    };
    serde_json::from_str(body).map_err(|e| format!("reply is not JSON ({e})"))
}

/// One try: its number (1-based) and what was wrong with it (empty means accepted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    pub number: u32,
    pub problems: Vec<String>,
}

/// The prompt for one message.
pub fn prompt_for(message: &str) -> String {
    format!(
        "Extract a support ticket from this message. Reply with JSON only, with the keys title (string), priority (low, medium or high), tags (array of strings) and estimate_hours (integer, 1 to 40).\n\nMessage: {message}"
    )
}

fn retry_prompt(base: &str, problems: &[String]) -> String {
    let mut p = format!("{base}\n\nYour previous reply was rejected:\n");
    for x in problems {
        p.push_str(&format!("- {x}\n"));
    }
    p.push_str("Reply with the JSON object only.");
    p
}

/// Ask until a reply validates or `max` attempts are used. The attempt number (from 0) is the
/// seed, so a recording can answer attempt by attempt. `None` means the retries ran out: the
/// caller gets no value, never a bad one.
pub fn ask(
    responder: &dyn Responder,
    schema: &Value,
    base_prompt: &str,
    max: u32,
) -> Result<(Option<Value>, Vec<Attempt>), RunError> {
    let mut attempts = Vec::new();
    let mut prompt = base_prompt.to_string();
    for n in 0..max {
        let reply = responder.answer(&prompt, n, 0.0)?;
        let parsed = extract_json(&reply);
        let problems = match &parsed {
            Err(e) => vec![e.clone()],
            Ok(v) => validate(schema, v),
        };
        attempts.push(Attempt {
            number: n + 1,
            problems: problems.clone(),
        });
        if problems.is_empty() {
            return Ok((parsed.ok(), attempts));
        }
        prompt = retry_prompt(base_prompt, &problems);
    }
    Ok((None, attempts))
}

pub struct Outcome {
    pub id: String,
    pub value: Option<Value>,
    pub attempts: Vec<Attempt>,
}

pub struct Summary {
    pub schema: Value,
    pub outcomes: Vec<Outcome>,
    /// The validator refuses an empty object (it is not a validator that accepts everything).
    pub floor_refused: bool,
    /// Offline, the recordings fix how many attempts each case needs.
    pub require_recorded_shape: bool,
}

impl Summary {
    /// Every accepted value validates and its last attempt was clean; a case with no value used
    /// all its attempts and every one failed; attempts never exceed the bound; the validator
    /// refuses an empty object; offline, each case needs exactly the recorded number of attempts.
    pub fn contract_holds(&self) -> bool {
        let sound = self.outcomes.iter().all(|o| {
            let last_clean = o.attempts.last().is_some_and(|a| a.problems.is_empty());
            o.attempts.len() <= MAX_ATTEMPTS as usize
                && match &o.value {
                    Some(v) => validate(&self.schema, v).is_empty() && last_clean,
                    None => {
                        o.attempts.len() == MAX_ATTEMPTS as usize
                            && o.attempts.iter().all(|a| !a.problems.is_empty())
                    }
                }
        });
        let shape = !self.require_recorded_shape
            || (self.outcomes.len() == EXPECTED.len()
                && self.outcomes.iter().zip(EXPECTED).all(|(o, (id, used))| {
                    o.id == id
                        && (o.value.is_some() == used.is_some())
                        && used.is_none_or(|n| o.attempts.len() == n)
                }));
        !self.outcomes.is_empty() && self.floor_refused && sound && shape
    }
}

/// Group the fixture's replies by case, in attempt order.
fn replay_from(text: &str, cases: &[(String, String)]) -> Result<Replay, String> {
    let mut by_case: BTreeMap<String, Vec<(u64, String)>> = BTreeMap::new();
    for r in rows(text)? {
        let attempt = r
            .get("attempt")
            .and_then(Value::as_u64)
            .ok_or("fixture row has no numeric attempt")?;
        by_case
            .entry(field(&r, "case")?.to_string())
            .or_default()
            .push((attempt, field(&r, "reply")?.to_string()));
    }
    let mut replay = Replay::default();
    for (id, prompt) in cases {
        let mut list = by_case
            .remove(id)
            .ok_or_else(|| format!("fixture has no replies for {id:?}"))?;
        list.sort();
        replay = replay.with(prompt, list.into_iter().map(|(_, r)| r).collect());
    }
    Ok(replay)
}

/// The set's cases as (id, prompt).
pub fn cases() -> Result<Vec<(String, String)>, String> {
    cases_from(SET)
}

/// Parse a set of structured-output cases.
pub fn cases_from(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let n = i + 1;
        let v: Value =
            serde_json::from_str(line).map_err(|e| format!("structured.jsonl line {n}: {e}"))?;
        out.push((
            field(&v, "id")?.to_string(),
            prompt_for(field(&v, "message")?),
        ));
    }
    Ok(out)
}

/// Offline replays the fixture; `live` asks the configured runtime.
pub fn run(cfg: &Config, live: bool, out: &mut impl Write) -> Result<Summary, RunError> {
    let schema: Value = serde_json::from_str(SCHEMA).expect("evals/ticket.schema.json is JSON");
    let cases = cases().expect("evals/structured.jsonl is a valid set");
    let responder: Box<dyn Responder> = if live {
        Box::new(ClientResponder {
            client: client_for(cfg.runtime, Some(&cfg.base_url), true),
            model: cfg.model.clone(),
        })
    } else {
        Box::new(replay_from(FIXTURE, &cases).expect("the structured fixture is valid"))
    };
    let mut outcomes = Vec::new();
    for (id, prompt) in &cases {
        let (value, attempts) = ask(responder.as_ref(), &schema, prompt, MAX_ATTEMPTS)?;
        let _ = writeln!(
            out,
            "{id:<16} {} after {} attempt(s)",
            if value.is_some() {
                "accepted"
            } else {
                "GAVE UP "
            },
            attempts.len()
        );
        for a in attempts.iter().filter(|a| !a.problems.is_empty()) {
            let _ = writeln!(
                out,
                "  attempt {} rejected: {}",
                a.number,
                a.problems.join("; ")
            );
        }
        if let Some(v) = &value {
            let _ = writeln!(out, "  value: {v}");
        }
        outcomes.push(Outcome {
            id: id.clone(),
            value,
            attempts,
        });
    }
    Ok(Summary {
        floor_refused: !validate(&schema, &json!({})).is_empty(),
        schema,
        outcomes,
        require_recorded_shape: !live,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rll_client::Runtime;

    fn cfg() -> Config {
        Config {
            runtime: Runtime::Apr,
            base_url: Runtime::Apr.default_base_url().into(),
            model: "m".into(),
        }
    }

    fn schema() -> Value {
        serde_json::from_str(SCHEMA).unwrap()
    }

    fn offline() -> Summary {
        run(&cfg(), false, &mut Vec::new()).unwrap()
    }

    fn good() -> Value {
        json!({"title":"t","priority":"low","tags":["x"],"estimate_hours":3})
    }

    #[test]
    fn offline_run_satisfies_the_contract_and_prints_the_story() {
        let mut buf = Vec::new();
        let s = run(&cfg(), false, &mut buf).unwrap();
        assert!(s.contract_holds());
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("clean"));
        assert!(text.contains("accepted after 1 attempt(s)"));
        assert!(text.contains("attempt 1 rejected: reply is not JSON"));
        assert!(text.contains("is not one of"));
        assert!(text.contains("GAVE UP  after 3 attempt(s)"));
        assert!(text.contains("value: {"));
    }

    #[test]
    fn the_schema_accepts_a_good_ticket_and_refuses_each_kind_of_bad_one() {
        let s = schema();
        assert!(validate(&s, &good()).is_empty());
        let bad = |v: Value| validate(&s, &v);
        assert!(bad(json!({}))
            .iter()
            .any(|e| e.contains("missing required")));
        assert!(bad(json!([])).iter().any(|e| e.contains("expected object")));
        let mut v = good();
        v["priority"] = json!("critical");
        assert!(bad(v).iter().any(|e| e.contains("is not one of")));
        let mut v = good();
        v["estimate_hours"] = json!(0);
        assert!(bad(v).iter().any(|e| e.contains("below the minimum")));
        let mut v = good();
        v["estimate_hours"] = json!(41);
        assert!(bad(v).iter().any(|e| e.contains("above the maximum")));
        let mut v = good();
        v["estimate_hours"] = json!(2.5);
        assert!(bad(v).iter().any(|e| e.contains("expected integer")));
        let mut v = good();
        v["title"] = json!("");
        assert!(bad(v).iter().any(|e| e.contains("shorter than")));
        let mut v = good();
        v["tags"] = json!(["ok", 7]);
        assert!(bad(v).iter().any(|e| e.contains("$.tags[1]")));
        let mut v = good();
        v["extra"] = json!(true);
        assert!(bad(v).iter().any(|e| e.contains("unexpected property")));
    }

    #[test]
    fn every_type_name_is_checked_and_an_unknown_one_fails() {
        for (t, ok, no) in [
            ("object", json!({}), json!(1)),
            ("array", json!([]), json!(1)),
            ("string", json!("s"), json!(1)),
            ("integer", json!(1), json!(1.5)),
            ("number", json!(1.5), json!("s")),
            ("boolean", json!(true), json!(1)),
            ("null", json!(null), json!(1)),
        ] {
            let s = json!({ "type": t });
            assert!(validate(&s, &ok).is_empty(), "{t}");
            assert!(!validate(&s, &no).is_empty(), "{t}");
        }
        assert!(!validate(&json!({"type":"mystery"}), &json!(1)).is_empty());
    }

    #[test]
    fn an_unknown_keyword_or_a_non_object_schema_fails_closed() {
        let e = validate(&json!({"pattern":"^a"}), &json!("b"));
        assert!(e.iter().any(|e| e.contains("unsupported schema keyword")));
        let e = validate(&json!(true), &json!(1));
        assert!(e.iter().any(|e| e.contains("not an object")));
    }

    #[test]
    fn additional_properties_true_and_missing_properties_are_open() {
        let open = json!({"type":"object","additionalProperties":true});
        assert!(validate(&open, &json!({"a":1})).is_empty());
        assert!(validate(&json!({"type":"object"}), &json!({"a":1})).is_empty());
    }

    #[test]
    fn extract_json_reads_plain_fenced_and_refuses_prose() {
        assert_eq!(extract_json(" {\"a\":1} ").unwrap(), json!({"a":1}));
        assert_eq!(
            extract_json("```json\n{\"a\":1}\n```").unwrap(),
            json!({"a":1})
        );
        assert!(extract_json("```").is_err());
        assert!(extract_json("I can't help with that.")
            .unwrap_err()
            .contains("not JSON"));
        assert!(extract_json("Here you go: {\"a\":1}").is_err());
    }

    struct Fixed(Vec<&'static str>);

    impl Responder for Fixed {
        fn answer(&self, prompt: &str, seed: u32, _t: f32) -> Result<String, rll_client::Error> {
            if seed > 0 {
                assert!(prompt.contains("Your previous reply was rejected"));
                assert!(prompt.contains("- "));
            }
            Ok(self.0[(seed as usize).min(self.0.len() - 1)].to_string())
        }
    }

    #[test]
    fn a_retry_carries_the_reasons_and_stops_at_the_first_valid_reply() {
        let ok = r#"{"title":"t","priority":"low","tags":[],"estimate_hours":1}"#;
        let r = Fixed(vec!["no", "still no", ok, "never reached"]);
        let (v, attempts) = ask(&r, &schema(), "base", 5).unwrap();
        assert!(v.is_some());
        assert_eq!(attempts.len(), 3);
        assert!(attempts[2].problems.is_empty());
    }

    #[test]
    fn retries_are_bounded_and_give_no_value() {
        let r = Fixed(vec!["no"]);
        let (v, attempts) = ask(&r, &schema(), "base", 2).unwrap();
        assert!(v.is_none());
        assert_eq!(attempts.len(), 2);
        let (v, attempts) = ask(&r, &schema(), "base", 0).unwrap();
        assert!(v.is_none() && attempts.is_empty());
    }

    #[test]
    fn a_model_error_stops_the_ask() {
        let r = Replay::default();
        assert!(matches!(
            ask(&r, &schema(), "base", 3),
            Err(RunError::Model(_))
        ));
    }

    #[test]
    fn contract_fails_when_an_accepted_value_is_invalid() {
        let mut s = offline();
        s.outcomes[0].value = Some(json!({"title":"t"}));
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_when_the_accepting_attempt_was_not_clean() {
        let mut s = offline();
        s.outcomes[0].attempts[0].problems = vec!["x".into()];
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_when_a_gave_up_case_did_not_use_every_attempt() {
        let mut s = offline();
        let last = s.outcomes.last_mut().unwrap();
        last.attempts.pop();
        assert!(!s.contract_holds());
        let mut s = offline();
        s.outcomes.last_mut().unwrap().attempts[1].problems.clear();
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_when_attempts_exceed_the_bound() {
        let mut s = offline();
        let extra = s.outcomes[1].attempts[0].clone();
        s.outcomes[1]
            .attempts
            .extend([extra.clone(), extra.clone(), extra]);
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_when_the_validator_accepts_everything() {
        let mut s = offline();
        s.floor_refused = false;
        assert!(!s.contract_holds());
        let mut s = offline();
        s.outcomes.clear();
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_offline_when_the_recorded_shape_drifts_but_not_live() {
        let mut s = offline();
        s.outcomes[1].attempts.remove(0);
        s.outcomes[1].attempts[0].number = 1;
        assert!(!s.contract_holds());
        s.require_recorded_shape = false;
        assert!(s.contract_holds());
        let mut s = offline();
        s.outcomes[0].id = "renamed".into();
        assert!(!s.contract_holds());
    }

    #[test]
    fn the_recorded_shape_names_every_case_in_the_set() {
        let ids: Vec<String> = cases().unwrap().into_iter().map(|(id, _)| id).collect();
        let want: Vec<&str> = EXPECTED.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, want);
    }

    #[test]
    fn a_bad_fixture_is_refused() {
        let p = r#""provenance":"illustrative: x""#;
        let cs = vec![("a".to_string(), "P".to_string())];
        let row = |extra: &str| format!("{{{p},{extra}}}");
        let ok = row(r#""case":"a","attempt":0,"reply":"r""#);
        assert!(replay_from(&ok, &cs).is_ok());
        assert!(replay_from(&row(r#""case":"a","reply":"r""#), &cs)
            .unwrap_err()
            .contains("attempt"));
        assert!(
            replay_from(&row(r#""case":"b","attempt":0,"reply":"r""#), &cs)
                .unwrap_err()
                .contains("no replies")
        );
        assert!(replay_from(&row(r#""attempt":0,"reply":"r""#), &cs).is_err());
        assert!(replay_from(&row(r#""case":"a","attempt":0"#), &cs).is_err());
    }

    #[test]
    fn replies_are_ordered_by_attempt_not_by_line() {
        let p = r#""provenance":"illustrative: x""#;
        let text = format!(
            "{{{p},\"case\":\"a\",\"attempt\":1,\"reply\":\"second\"}}\n{{{p},\"case\":\"a\",\"attempt\":0,\"reply\":\"first\"}}"
        );
        let r = replay_from(&text, &[("a".into(), "P".into())]).unwrap();
        assert_eq!(r.answer("P", 0, 0.0).unwrap(), "first");
        assert_eq!(r.answer("P", 1, 0.0).unwrap(), "second");
    }

    #[test]
    fn live_run_without_a_server_is_an_error() {
        let mut c = cfg();
        c.base_url = "http://127.0.0.1:1".into();
        assert!(matches!(
            run(&c, true, &mut Vec::new()),
            Err(RunError::Model(_))
        ));
    }

    #[test]
    fn a_bad_set_is_refused_and_blank_lines_are_skipped() {
        let ok = cases_from("\n{\"id\":\"a\",\"message\":\"m\"}\n\n").unwrap();
        assert_eq!(ok.len(), 1);
        assert!(cases_from("not json").unwrap_err().contains("line 1"));
    }
}
