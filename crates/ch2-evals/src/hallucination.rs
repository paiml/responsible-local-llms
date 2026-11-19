//! Lesson 2.2: catch hallucination. An answer must cite the source passage it relies on and
//! quote it verbatim, or the test fails. A question the passages cannot answer must be refused.

use crate::replay::{field, rows, Replay};
use ch1_sovereign::client_for;
use rll_client::Config;
use rll_evals::{ClientResponder, Responder, RunError};
use serde::Deserialize;
use std::io::Write;

/// The eval set the lesson reads (`evals/hallucination.jsonl`).
pub const SET: &str = include_str!("../../../evals/hallucination.jsonl");
/// Illustrative answers: good ones the offline run replays, and one trap per failure verdict.
pub const FIXTURE: &str = include_str!("../fixtures/hallucination.jsonl");
/// What a model must say when the passages do not answer the question.
pub const REFUSAL: &str = "NOT IN SOURCE";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Passage {
    pub id: String,
    pub text: String,
}

/// A question, the passages it may use, and (when answerable) the passage that answers it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    pub question: String,
    pub passages: Vec<Passage>,
    /// The id of the passage that answers the question; absent when nothing does.
    pub gold: Option<String>,
    /// The fact the quoted text must carry; present exactly when `gold` is.
    pub fact: Option<String>,
}

/// Parse and validate a hallucination set.
pub fn parse(text: &str) -> Result<Vec<Case>, String> {
    let mut cases: Vec<Case> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let c = parse_line(i + 1, line, &cases)?;
        cases.push(c);
    }
    if cases.is_empty() {
        return Err("no cases".into());
    }
    Ok(cases)
}

fn parse_line(n: usize, line: &str, seen: &[Case]) -> Result<Case, String> {
    let c: Case = serde_json::from_str(line).map_err(|e| format!("line {n}: {e}"))?;
    if seen.iter().any(|o| o.id == c.id) {
        return Err(format!("line {n}: duplicate id {:?}", c.id));
    }
    if c.passages.is_empty() {
        return Err(format!("line {n}: a case needs passages"));
    }
    check_gold(n, &c)?;
    Ok(c)
}

fn check_gold(n: usize, c: &Case) -> Result<(), String> {
    match (&c.gold, &c.fact) {
        (None, None) => Ok(()),
        (Some(g), Some(f)) => match c.passages.iter().find(|p| &p.id == g) {
            None => Err(format!("line {n}: gold {g:?} is not a passage")),
            Some(p) if !norm(&p.text).contains(&norm(f)) => {
                Err(format!("line {n}: fact {f:?} is not in passage {g:?}"))
            }
            Some(_) => Ok(()),
        },
        _ => Err(format!("line {n}: gold and fact go together")),
    }
}

/// The prompt a model gets: passages, then the question, with the citation rules.
pub fn prompt_for(case: &Case) -> String {
    let mut p = format!(
        "Answer using only the passages. Cite the passage id in square brackets and quote the supporting text in double quotes. If the passages do not say, reply exactly: {REFUSAL}\n\nPassages:\n"
    );
    for ps in &case.passages {
        p.push_str(&format!("[{}] {}\n", ps.id, ps.text));
    }
    p.push_str(&format!("\nQuestion: {}", case.question));
    p
}

/// How one answer fared. `Grounded` and `Refused` pass; everything else is a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Grounded,
    Refused,
    /// Answered a question the passages do not answer.
    Hallucinated,
    /// Refused a question the passages do answer.
    WrongRefusal,
    /// No `[id]` citation, or no quoted text.
    Uncited,
    /// Cited a passage other than the one that answers it.
    WrongSource,
    /// A quote that is not in the cited passage.
    FabricatedQuote,
    /// Quotes the passage, but the quote does not carry the fact.
    MissingFact,
}

impl Verdict {
    pub const FAILURES: [Verdict; 6] = [
        Verdict::Hallucinated,
        Verdict::WrongRefusal,
        Verdict::Uncited,
        Verdict::WrongSource,
        Verdict::FabricatedQuote,
        Verdict::MissingFact,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Verdict::Grounded => "Grounded",
            Verdict::Refused => "Refused",
            Verdict::Hallucinated => "Hallucinated",
            Verdict::WrongRefusal => "WrongRefusal",
            Verdict::Uncited => "Uncited",
            Verdict::WrongSource => "WrongSource",
            Verdict::FabricatedQuote => "FabricatedQuote",
            Verdict::MissingFact => "MissingFact",
        }
    }

    pub fn from_name(name: &str) -> Option<Verdict> {
        Verdict::FAILURES
            .into_iter()
            .chain([Verdict::Grounded, Verdict::Refused])
            .find(|v| v.name() == name)
    }

    pub fn passes(self) -> bool {
        matches!(self, Verdict::Grounded | Verdict::Refused)
    }
}

/// Lower-case, single-spaced.
fn norm(s: &str) -> String {
    s.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every `[id]` in the answer.
fn citations(answer: &str) -> Vec<String> {
    answer
        .split('[')
        .skip(1)
        .filter_map(|s| s.split_once(']').map(|(id, _)| id.trim().to_string()))
        .collect()
}

/// Every closed `"quoted span"` in the answer.
fn quotes(answer: &str) -> Vec<String> {
    let parts: Vec<&str> = answer.split('"').collect();
    parts
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 1 && i + 1 < parts.len())
        .map(|(_, q)| q.to_string())
        .collect()
}

/// Judge one answer against its case.
pub fn judge(case: &Case, answer: &str) -> Verdict {
    let refused = answer
        .trim()
        .trim_end_matches('.')
        .trim()
        .eq_ignore_ascii_case(REFUSAL);
    let (gold, fact) = match (&case.gold, &case.fact) {
        (Some(g), Some(f)) => (g, f),
        _ => {
            return if refused {
                Verdict::Refused
            } else {
                Verdict::Hallucinated
            }
        }
    };
    if refused {
        return Verdict::WrongRefusal;
    }
    let cited = citations(answer);
    let quoted = quotes(answer);
    if cited.is_empty() || quoted.is_empty() {
        return Verdict::Uncited;
    }
    if !cited.iter().all(|c| c == gold) {
        return Verdict::WrongSource;
    }
    let passage = norm(
        &case
            .passages
            .iter()
            .find(|p| &p.id == gold)
            .expect("parse() checked that gold is a passage")
            .text,
    );
    if !quoted.iter().all(|q| passage.contains(&norm(q))) {
        return Verdict::FabricatedQuote;
    }
    if !quoted.iter().any(|q| norm(q).contains(&norm(fact))) {
        return Verdict::MissingFact;
    }
    Verdict::Grounded
}

/// One case answered by the model under test.
pub struct Row {
    pub case_id: String,
    pub answer: String,
    pub verdict: Verdict,
}

/// A known-bad answer and the verdict the checker must give it.
pub struct Trap {
    pub case_id: String,
    pub expect: Verdict,
    pub got: Verdict,
}

pub struct Summary {
    pub rows: Vec<Row>,
    pub traps: Vec<Trap>,
    /// Offline, the replayed answers are all good. A live model is measured, not required to pass.
    pub require_all_pass: bool,
}

impl Summary {
    /// The checker refuses every kind of bad answer (one trap per failure verdict, each caught
    /// with the right verdict), and offline every replayed answer passes.
    pub fn contract_holds(&self) -> bool {
        let caught = self.traps.iter().all(|t| t.got == t.expect);
        let covers_all = Verdict::FAILURES
            .iter()
            .all(|v| self.traps.iter().any(|t| t.expect == *v));
        !self.rows.is_empty()
            && caught
            && covers_all
            && (!self.require_all_pass || self.rows.iter().all(|r| r.verdict.passes()))
    }
}

/// Offline replay of the fixture's good answers, plus the fixture's trap rows.
type Traps = Vec<(String, Verdict, String)>;

fn load_fixture(text: &str, cases: &[Case]) -> Result<(Replay, Traps), String> {
    let mut replay = Replay::default();
    let mut traps = Vec::new();
    let all = rows(text)?;
    for case in cases {
        let good = all
            .iter()
            .find(|r| field(r, "role") == Ok("good") && field(r, "case") == Ok(case.id.as_str()))
            .ok_or_else(|| format!("fixture has no good answer for {:?}", case.id))?;
        replay = replay.with(&prompt_for(case), vec![field(good, "answer")?.to_string()]);
    }
    for r in all.iter().filter(|r| field(r, "role") == Ok("trap")) {
        let expect = Verdict::from_name(field(r, "expect")?)
            .ok_or_else(|| "fixture trap names an unknown verdict".to_string())?;
        traps.push((
            field(r, "case")?.to_string(),
            expect,
            field(r, "answer")?.to_string(),
        ));
    }
    Ok((replay, traps))
}

/// Offline replays the fixture; `live` asks the configured runtime.
pub fn run(cfg: &Config, live: bool, out: &mut impl Write) -> Result<Summary, RunError> {
    let cases = parse(SET).expect("evals/hallucination.jsonl is a valid set");
    let (replay, trap_rows) =
        load_fixture(FIXTURE, &cases).expect("the hallucination fixture is valid");
    let responder: Box<dyn Responder> = if live {
        Box::new(ClientResponder {
            client: client_for(cfg.runtime, Some(&cfg.base_url), true),
            model: cfg.model.clone(),
        })
    } else {
        Box::new(replay)
    };
    let mut rows = Vec::new();
    for case in &cases {
        let answer = responder.answer(&prompt_for(case), 0, 0.0)?;
        let verdict = judge(case, &answer);
        let _ = writeln!(out, "{:<15} {:<15} {}", case.id, verdict.name(), answer);
        rows.push(Row {
            case_id: case.id.clone(),
            answer,
            verdict,
        });
    }
    let _ = writeln!(out, "traps: bad answers the checker must catch");
    let mut traps = Vec::new();
    for (case_id, expect, answer) in trap_rows {
        let case = cases
            .iter()
            .find(|c| c.id == case_id)
            .expect("fixture traps name a case in the set");
        let got = judge(case, &answer);
        let _ = writeln!(
            out,
            "  {:<15} expect {:<15} got {:<15} {}",
            case_id,
            expect.name(),
            got.name(),
            answer
        );
        traps.push(Trap {
            case_id,
            expect,
            got,
        });
    }
    Ok(Summary {
        rows,
        traps,
        require_all_pass: !live,
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

    fn cases() -> Vec<Case> {
        parse(SET).unwrap()
    }

    fn case(id: &str) -> Case {
        cases().into_iter().find(|c| c.id == id).unwrap()
    }

    fn offline() -> Summary {
        run(&cfg(), false, &mut Vec::new()).unwrap()
    }

    #[test]
    fn offline_run_satisfies_the_contract_and_prints_the_story() {
        let mut buf = Vec::new();
        let s = run(&cfg(), false, &mut buf).unwrap();
        assert!(s.contract_holds());
        assert_eq!(s.rows.len(), 4);
        assert_eq!(s.traps.len(), 6);
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("refund-window"));
        assert!(text.contains("Grounded"));
        assert!(text.contains("Refused"));
        assert!(text.contains("traps: bad answers"));
        assert!(text.contains("expect Hallucinated"));
    }

    #[test]
    fn each_verdict_has_a_name_that_round_trips() {
        for v in Verdict::FAILURES {
            assert_eq!(Verdict::from_name(v.name()), Some(v));
            assert!(!v.passes());
        }
        assert_eq!(Verdict::from_name("Grounded"), Some(Verdict::Grounded));
        assert_eq!(Verdict::from_name("Refused"), Some(Verdict::Refused));
        assert!(Verdict::Grounded.passes() && Verdict::Refused.passes());
        assert_eq!(Verdict::from_name("Nope"), None);
    }

    #[test]
    fn the_checker_gives_each_failure_its_own_verdict() {
        let c = case("refund-window");
        let q = "\"The Orchard Co-op refund window is 30 days from delivery.\"";
        assert_eq!(judge(&c, &format!("[p1] {q}")), Verdict::Grounded);
        assert_eq!(judge(&c, "NOT IN SOURCE."), Verdict::WrongRefusal);
        assert_eq!(judge(&c, "It is 30 days."), Verdict::Uncited);
        assert_eq!(
            judge(&c, &format!("{q} without a citation")),
            Verdict::Uncited
        );
        assert_eq!(judge(&c, "[p1] no quote at all"), Verdict::Uncited);
        assert_eq!(judge(&c, "[p1] \"unterminated"), Verdict::Uncited);
        assert_eq!(judge(&c, &format!("[p2] {q}")), Verdict::WrongSource);
        assert_eq!(judge(&c, &format!("[p1] [p2] {q}")), Verdict::WrongSource);
        assert_eq!(
            judge(&c, "[p1] \"Refunds take 30 days, always.\""),
            Verdict::FabricatedQuote
        );
        assert_eq!(
            judge(&c, &format!("[p1] {q} \"and something invented\"")),
            Verdict::FabricatedQuote
        );
        assert_eq!(
            judge(&c, "[p1] \"The Orchard Co-op\""),
            Verdict::MissingFact
        );
    }

    #[test]
    fn an_unanswerable_question_needs_the_refusal() {
        let c = case("support-hours");
        assert_eq!(judge(&c, "not in source"), Verdict::Refused);
        assert_eq!(judge(&c, "  NOT IN SOURCE.  "), Verdict::Refused);
        assert_eq!(judge(&c, "Nine to five."), Verdict::Hallucinated);
        assert_eq!(
            judge(
                &c,
                "[p1] \"The Orchard Co-op refund window is 30 days from delivery.\""
            ),
            Verdict::Hallucinated
        );
    }

    #[test]
    fn quotes_match_across_case_and_whitespace() {
        let c = case("free-shipping");
        assert_eq!(
            judge(&c, "[p2] \"shipping  is FREE on orders\nover 50 euros.\""),
            Verdict::Grounded
        );
    }

    #[test]
    fn the_prompt_carries_every_passage_the_question_and_the_refusal_rule() {
        let p = prompt_for(&case("refund-window"));
        assert!(p.contains("[p1] The Orchard Co-op refund window"));
        assert!(p.contains("[p2] Shipping is free"));
        assert!(p.contains("Question: How long is the refund window?"));
        assert!(p.contains(REFUSAL));
    }

    #[test]
    fn a_set_with_a_hole_is_refused() {
        let ok = r#"{"id":"a","question":"q","passages":[{"id":"p1","text":"It is 30 days."}],"gold":"p1","fact":"30 days"}"#;
        assert!(parse(&format!("\n{ok}\n\n")).is_ok());
        assert!(parse("").unwrap_err().contains("no cases"));
        assert!(parse("{nope").unwrap_err().contains("line 1"));
        assert!(parse(&format!("{ok}\n{ok}"))
            .unwrap_err()
            .contains("duplicate"));
        let e = parse(r#"{"id":"a","question":"q","passages":[]}"#).unwrap_err();
        assert!(e.contains("needs passages"), "{e}");
        let e = parse(&ok.replace("\"gold\":\"p1\"", "\"gold\":\"p9\"")).unwrap_err();
        assert!(e.contains("not a passage"), "{e}");
        let e = parse(&ok.replace("30 days\"}", "31 days\"}")).unwrap_err();
        assert!(e.contains("not in passage"), "{e}");
        let e = parse(&ok.replace(",\"fact\":\"30 days\"", "")).unwrap_err();
        assert!(e.contains("go together"), "{e}");
    }

    #[test]
    fn contract_fails_when_a_trap_is_not_caught() {
        let mut s = offline();
        s.traps[0].got = Verdict::Grounded;
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_when_a_failure_kind_has_no_trap() {
        let mut s = offline();
        s.traps.pop();
        assert!(!s.contract_holds());
        s.traps.clear();
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_offline_when_an_answer_does_not_pass_but_not_live() {
        let mut s = offline();
        s.rows[0].verdict = Verdict::Uncited;
        assert!(!s.contract_holds());
        s.require_all_pass = false;
        assert!(s.contract_holds());
        s.rows.clear();
        assert!(!s.contract_holds());
    }

    #[test]
    fn the_fixture_is_complete_for_the_set() {
        assert!(load_fixture(FIXTURE, &cases()).is_ok());
        let mut extra = cases();
        let mut c = extra[0].clone();
        c.id = "no-recording".into();
        extra.push(c);
        assert!(load_fixture(FIXTURE, &extra)
            .unwrap_err()
            .contains("no good answer"));
    }

    #[test]
    fn a_bad_fixture_is_refused() {
        let p = r#""provenance":"illustrative: x""#;
        let good = format!(r#"{{{p},"role":"good","case":"a","answer":"x"}}"#);
        let one =
            parse(r#"{"id":"a","question":"q","passages":[{"id":"p1","text":"t"}]}"#).unwrap();
        assert!(load_fixture(&good, &one).is_ok());
        let trap = format!(r#"{{{p},"role":"trap","expect":"Nope","case":"a","answer":"x"}}"#);
        let bad = format!("{good}\n{trap}");
        assert!(load_fixture(&bad, &one)
            .unwrap_err()
            .contains("unknown verdict"));
        let trap = format!(r#"{{{p},"role":"trap","expect":"Uncited","case":"a"}}"#);
        let bad = format!("{good}\n{trap}");
        assert!(load_fixture(&bad, &one).unwrap_err().contains("answer"));
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
}
