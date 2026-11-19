//! Lesson 2.5: Elo in Rust. Rank models head to head on the same eval cases, then decide
//! whether a candidate is good enough to replace the model you run today.

use crate::replay::{field, rows, Replay};
use rll_evals::{parse_jsonl, Case, Responder, RunError};
use std::collections::BTreeMap;
use std::io::Write;

/// The head-to-head cases (`evals/elo.jsonl`).
pub const SET: &str = include_str!("../../../evals/elo.jsonl");
/// Illustrative answers, one per model and case. The model names are placeholders.
pub const FIXTURE: &str = include_str!("../fixtures/elo.jsonl");
/// The fixture's models, strongest first by construction.
pub const MODELS: [&str; 3] = ["model-a", "model-b", "model-c"];
/// Everyone starts here.
pub const START: f64 = 1000.0;
/// How far one result moves a rating.
pub const K: f64 = 32.0;
/// A candidate is good enough when it is expected to score at least this against the reference.
pub const MIN_EXPECTED: f64 = 0.35;

/// The score `ra` is expected to take from `rb` (0 to 1). The two sides always sum to 1.
pub fn expected_score(ra: f64, rb: f64) -> f64 {
    1.0 / (1.0 + 10f64.powf((rb - ra) / 400.0))
}

/// One head-to-head result: `score_a` is 1 for an `a` win, 0.5 for a draw, 0 for a loss.
#[derive(Debug, Clone, PartialEq)]
pub struct Match {
    pub case_id: String,
    pub a: String,
    pub b: String,
    pub score_a: f64,
}

/// Apply every match in order. Ratings come back strongest first (ties by name).
pub fn rate(models: &[String], matches: &[Match]) -> Result<Vec<(String, f64)>, String> {
    if models.len() < 2 {
        return Err("a ranking needs at least two models".into());
    }
    if matches.is_empty() {
        return Err("a ranking needs at least one match".into());
    }
    let mut r: BTreeMap<&str, f64> = models.iter().map(|m| (m.as_str(), START)).collect();
    for m in matches {
        if m.a == m.b {
            return Err(format!("{} cannot play itself", m.a));
        }
        let (Some(&ra), Some(&rb)) = (r.get(m.a.as_str()), r.get(m.b.as_str())) else {
            return Err(format!("match names an unknown model: {} vs {}", m.a, m.b));
        };
        let ea = expected_score(ra, rb);
        r.insert(m.a.as_str(), ra + K * (m.score_a - ea));
        r.insert(m.b.as_str(), rb + K * ((1.0 - m.score_a) - (1.0 - ea)));
    }
    let mut out: Vec<(String, f64)> = r.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    out.sort_by(|x, y| y.1.total_cmp(&x.1).then_with(|| x.0.cmp(&y.0)));
    Ok(out)
}

/// Good enough: the candidate is expected to score at least `min_expected` against the reference.
pub fn good_enough(candidate: f64, reference: f64, min_expected: f64) -> bool {
    expected_score(candidate, reference) >= min_expected
}

/// Fraction of a case's properties an answer has (1 when it passes the case).
fn score(case: &Case, answer: &str) -> f64 {
    let total = case.properties.len() as f64;
    (total - case.check(answer).len() as f64) / total
}

/// Every pair of models plays every case: the better answer wins, equal answers draw.
pub fn matches_from(
    cases: &[Case],
    models: &[(String, &dyn Responder)],
) -> Result<Vec<Match>, RunError> {
    let mut out = Vec::new();
    for (i, (a, ra)) in models.iter().enumerate() {
        for (b, rb) in &models[i + 1..] {
            for case in cases {
                let sa = score(case, &ra.answer(&case.prompt, 0, 0.0)?);
                let sb = score(case, &rb.answer(&case.prompt, 0, 0.0)?);
                out.push(Match {
                    case_id: case.id.clone(),
                    a: a.clone(),
                    b: b.clone(),
                    score_a: if sa > sb {
                        1.0
                    } else if sa < sb {
                        0.0
                    } else {
                        0.5
                    },
                });
            }
        }
    }
    Ok(out)
}

pub struct Summary {
    pub ratings: Vec<(String, f64)>,
    pub matches: Vec<Match>,
    /// Is the candidate good enough against the reference.
    pub good_enough: bool,
}

impl Summary {
    /// Ratings conserve the total (Elo only moves points between players), come strongest
    /// first, the expected scores of two players sum to 1, and the fixture's order holds.
    pub fn contract_holds(&self) -> bool {
        let total: f64 = self.ratings.iter().map(|(_, r)| r).sum();
        let conserved = (total - START * self.ratings.len() as f64).abs() < 1e-6;
        let sorted = self.ratings.windows(2).all(|w| w[0].1 >= w[1].1);
        let symmetric = match self.ratings.as_slice() {
            [(_, x), (_, y), ..] => {
                (expected_score(*x, *y) + expected_score(*y, *x) - 1.0).abs() < 1e-12
            }
            _ => false,
        };
        let order: Vec<&str> = self.ratings.iter().map(|(m, _)| m.as_str()).collect();
        !self.matches.is_empty() && conserved && sorted && symmetric && order == MODELS
    }
}

fn replay_for(model: &str, all: &[serde_json::Value], cases: &[Case]) -> Result<Replay, String> {
    let mut replay = Replay::default();
    for case in cases {
        let row = all
            .iter()
            .find(|r| field(r, "model") == Ok(model) && field(r, "case") == Ok(case.id.as_str()))
            .ok_or_else(|| format!("fixture has no answer from {model} for {:?}", case.id))?;
        replay = replay.with(&case.prompt, vec![field(row, "answer")?.to_string()]);
    }
    Ok(replay)
}

/// Offline only: the fixture's answers play every pair. The same `matches_from` ranks live
/// models when it is given real responders.
pub fn run(out: &mut impl Write) -> Result<Summary, RunError> {
    let cases = parse_jsonl(SET).expect("evals/elo.jsonl is a valid eval set");
    let all = rows(FIXTURE).expect("the elo fixture is valid");
    let replays: Vec<(String, Replay)> = MODELS
        .iter()
        .map(|m| {
            (
                m.to_string(),
                replay_for(m, &all, &cases).expect("the elo fixture covers every case"),
            )
        })
        .collect();
    let players: Vec<(String, &dyn Responder)> = replays
        .iter()
        .map(|(m, r)| (m.clone(), r as &dyn Responder))
        .collect();
    let matches = matches_from(&cases, &players)?;
    let names: Vec<String> = MODELS.iter().map(|m| m.to_string()).collect();
    let ratings = rate(&names, &matches).expect("the fixture gives a valid ranking");
    let _ = writeln!(out, "{} matches, K = {K}, start = {START}", matches.len());
    for (i, (m, r)) in ratings.iter().enumerate() {
        let _ = writeln!(out, "  {}. {m:<8} {r:7.1}", i + 1);
    }
    let rating = |name: &str| ratings.iter().find(|(m, _)| m == name).map(|(_, r)| *r);
    // The incumbent is the strongest model; the candidate is the middle one.
    let (reference, candidate) = (
        rating(MODELS[0]).expect("ranked"),
        rating(MODELS[1]).expect("ranked"),
    );
    let ok = good_enough(candidate, reference, MIN_EXPECTED);
    let _ = writeln!(
        out,
        "{} vs {}: expected score {:.2}, needs {MIN_EXPECTED}: {}",
        MODELS[1],
        MODELS[0],
        expected_score(candidate, reference),
        if ok { "good enough" } else { "not good enough" }
    );
    Ok(Summary {
        ratings,
        matches,
        good_enough: ok,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(n: &[&str]) -> Vec<String> {
        n.iter().map(|s| s.to_string()).collect()
    }

    fn m(a: &str, b: &str, score_a: f64) -> Match {
        Match {
            case_id: "c".into(),
            a: a.into(),
            b: b.into(),
            score_a,
        }
    }

    fn offline() -> Summary {
        run(&mut Vec::new()).unwrap()
    }

    #[test]
    fn offline_run_satisfies_the_contract_and_prints_the_ranking() {
        let mut buf = Vec::new();
        let s = run(&mut buf).unwrap();
        assert!(s.contract_holds());
        assert_eq!(s.matches.len(), 18);
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("18 matches, K = 32"));
        assert!(text.contains("1. model-a"));
        assert!(text.contains("3. model-c"));
        assert!(text.contains("good enough") || text.contains("not good enough"));
    }

    #[test]
    fn the_fixture_has_wins_and_draws_so_the_demo_shows_both() {
        let s = offline();
        assert!(s.matches.iter().any(|x| x.score_a == 1.0));
        assert!(s.matches.iter().any(|x| x.score_a == 0.5));
        assert!(s.matches.iter().any(|x| x.score_a == 0.0));
    }

    #[test]
    fn expected_scores_of_two_players_sum_to_one_and_equal_ratings_split_evenly() {
        assert!((expected_score(1000.0, 1000.0) - 0.5).abs() < 1e-12);
        let e = expected_score(1200.0, 1000.0);
        assert!(e > 0.75 && e < 0.77, "{e}");
        assert!((e + expected_score(1000.0, 1200.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_win_moves_k_over_two_points_from_loser_to_winner() {
        let r = rate(&names(&["x", "y"]), &[m("x", "y", 1.0)]).unwrap();
        assert_eq!(r[0].0, "x");
        assert!((r[0].1 - 1016.0).abs() < 1e-9);
        assert!((r[1].1 - 984.0).abs() < 1e-9);
    }

    #[test]
    fn a_draw_between_equals_changes_nothing_and_ties_sort_by_name() {
        let r = rate(&names(&["y", "x"]), &[m("x", "y", 0.5)]).unwrap();
        assert_eq!(r[0], ("x".to_string(), 1000.0));
        assert_eq!(r[1], ("y".to_string(), 1000.0));
    }

    #[test]
    fn a_loss_by_the_favourite_costs_more_than_a_win_gains() {
        let warm = [m("x", "y", 1.0), m("x", "y", 1.0), m("x", "y", 1.0)];
        let up = rate(&names(&["x", "y"]), &warm).unwrap()[0].1;
        let mut upset = warm.to_vec();
        upset.push(m("x", "y", 0.0));
        let after = rate(&names(&["x", "y"]), &upset).unwrap()[0].1;
        assert!(after < up);
        assert!(up - START < 3.0 * K / 2.0);
    }

    #[test]
    fn a_ranking_with_no_models_no_matches_or_a_bad_match_is_refused() {
        assert!(rate(&names(&["x"]), &[m("x", "y", 1.0)])
            .unwrap_err()
            .contains("two models"));
        assert!(rate(&names(&["x", "y"]), &[])
            .unwrap_err()
            .contains("one match"));
        assert!(rate(&names(&["x", "y"]), &[m("x", "x", 1.0)])
            .unwrap_err()
            .contains("itself"));
        assert!(rate(&names(&["x", "y"]), &[m("x", "z", 1.0)])
            .unwrap_err()
            .contains("unknown model"));
        assert!(rate(&names(&["x", "y"]), &[m("z", "y", 1.0)])
            .unwrap_err()
            .contains("unknown model"));
    }

    #[test]
    fn good_enough_is_a_threshold_on_the_expected_score() {
        assert!(good_enough(1000.0, 1000.0, 0.5));
        assert!(!good_enough(900.0, 1000.0, 0.5));
        assert!(good_enough(900.0, 1000.0, MIN_EXPECTED));
        assert!(!good_enough(800.0, 1000.0, MIN_EXPECTED));
    }

    #[test]
    fn contract_fails_when_points_are_created_or_lost() {
        let mut s = offline();
        s.ratings[0].1 += 5.0;
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_when_the_order_is_wrong_or_unsorted() {
        let mut s = offline();
        s.ratings.swap(0, 2);
        assert!(!s.contract_holds());
        let mut s = offline();
        s.ratings.swap(0, 1);
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_with_no_matches_or_too_few_players() {
        let mut s = offline();
        s.matches.clear();
        assert!(!s.contract_holds());
        let mut s = offline();
        s.ratings.truncate(1);
        assert!(!s.contract_holds());
    }

    #[test]
    fn scoring_counts_the_properties_an_answer_has() {
        let cases = parse_jsonl(SET).unwrap();
        let planet = cases.iter().find(|c| c.id == "largest-planet").unwrap();
        assert_eq!(score(planet, "Jupiter."), 1.0);
        assert_eq!(score(planet, "Saturn."), 0.5);
        assert_eq!(
            score(
                planet,
                "Saturn is a very big planet indeed, one of the large ones out there in the sky above us all."
            ),
            0.0
        );
    }

    #[test]
    fn a_failing_responder_stops_the_matches() {
        let cases = parse_jsonl(SET).unwrap();
        let empty = Replay::default();
        let players: Vec<(String, &dyn Responder)> =
            vec![("a".into(), &empty), ("b".into(), &empty)];
        assert!(matches!(
            matches_from(&cases, &players),
            Err(RunError::Model(_))
        ));
    }

    #[test]
    fn a_second_model_that_errors_also_stops_the_matches() {
        let cases = parse_jsonl(SET).unwrap();
        let good = Replay::default().with(&cases[0].prompt, vec!["Paris.".into()]);
        let empty = Replay::default();
        let players: Vec<(String, &dyn Responder)> =
            vec![("a".into(), &good), ("b".into(), &empty)];
        assert!(matches!(
            matches_from(&cases[..1], &players),
            Err(RunError::Model(_))
        ));
    }

    #[test]
    fn the_fixture_must_cover_every_case_for_every_model() {
        let cases = parse_jsonl(SET).unwrap();
        let all = rows(FIXTURE).unwrap();
        assert!(replay_for("model-a", &all, &cases).is_ok());
        assert!(replay_for("model-z", &all, &cases)
            .unwrap_err()
            .contains("no answer from model-z"));
        let p = r#""provenance":"illustrative: x""#;
        let no_answer = rows(&format!(r#"{{{p},"model":"m","case":"capital-france"}}"#)).unwrap();
        assert!(replay_for("m", &no_answer, &cases[..1]).is_err());
    }
}
