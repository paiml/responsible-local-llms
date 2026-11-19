//! Lesson 5.1: read the licence before you run the model, and check it in code.
//!
//! The cards under `fixtures/cards/` are ILLUSTRATIVE (written for the course, not copied
//! from a vendor). The policy table cites its sources in [`crate::sources`].

use crate::sources;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Permissive: use freely if you keep the listed obligations.
    Permissive,
    /// Custom terms: a person reads them before the model runs.
    Custom,
}

/// What this checker knows about one licence id.
pub struct Policy {
    pub id: &'static str,
    pub class: Class,
    pub obligations: &'static [&'static str],
    pub source: &'static str,
}

pub const POLICIES: [Policy; 3] = [
    Policy {
        id: "apache-2.0",
        class: Class::Permissive,
        obligations: &[
            "include the NOTICE file if the Work has one",
            "mark files you changed",
        ],
        source: "apache-2.0",
    },
    Policy {
        id: "mit",
        class: Class::Permissive,
        obligations: &["keep the copyright and permission notice (NOT VERIFIED)"],
        source: "mit",
    },
    Policy {
        id: "llama3.1",
        class: Class::Custom,
        obligations: &[
            "display \"Built with Llama\"",
            "start a derived model's name with \"Llama\"",
            "request a licence from the vendor above the user threshold",
        ],
        source: "llama-3.1",
    },
];

/// The monthly-active-user threshold in the Llama 3.1 licence, Section 2.
pub const LLAMA_MAU_LIMIT: u64 = 700_000_000;

/// The fields of a model card this checker reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub license: Option<String>,
    pub base_model: Option<String>,
}

/// Read `license` and `base_model` from the YAML block at the top of a card.
pub fn parse_card(text: &str) -> Card {
    let mut card = Card {
        license: None,
        base_model: None,
    };
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("---") {
        return card;
    }
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
            match k.trim() {
                "license" if !v.is_empty() => card.license = Some(v.to_lowercase()),
                "base_model" if !v.is_empty() => card.base_model = Some(v.to_string()),
                _ => {}
            }
        }
    }
    card
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Review,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub verdict: Verdict,
    pub reasons: Vec<String>,
    pub obligations: Vec<String>,
}

/// The policy row for a licence id.
pub fn policy_for(id: &str) -> Option<&'static Policy> {
    POLICIES.iter().find(|p| p.id == id)
}

/// Decide for one card. `monthly_active_users` is the size of your own product.
pub fn check(card: &Card, monthly_active_users: u64) -> Finding {
    let mut f = Finding {
        verdict: Verdict::Review,
        reasons: vec![],
        obligations: vec![],
    };
    let Some(id) = card.license.as_deref() else {
        f.verdict = Verdict::Deny;
        f.reasons
            .push("no licence declared: no right to use is shown".into());
        return f;
    };
    let Some(p) = policy_for(id) else {
        f.reasons.push(format!(
            "licence `{id}` is not in the policy table: read it"
        ));
        return f;
    };
    f.obligations = p.obligations.iter().map(|s| s.to_string()).collect();
    match p.class {
        Class::Permissive => {
            f.verdict = Verdict::Allow;
            f.reasons
                .push(format!("`{id}` is permissive (source: {})", p.source));
        }
        Class::Custom if id == "llama3.1" && monthly_active_users > LLAMA_MAU_LIMIT => {
            f.verdict = Verdict::Deny;
            f.reasons.push(format!(
                "{monthly_active_users} monthly active users is above {LLAMA_MAU_LIMIT}: a licence from the vendor is required first"
            ));
        }
        Class::Custom => {
            f.reasons.push(format!(
                "`{id}` has custom terms: read them and the use policy (source: {})",
                p.source
            ));
        }
    }
    f
}

/// A finding is sound when it never grants more than the table allows.
pub fn finding_is_sound(card: &Card, f: &Finding) -> bool {
    match f.verdict {
        Verdict::Allow => card
            .license
            .as_deref()
            .and_then(policy_for)
            .is_some_and(|p| p.class == Class::Permissive),
        Verdict::Review => card.license.is_some() && !f.reasons.is_empty(),
        Verdict::Deny => !f.reasons.is_empty(),
    }
}

/// The recorded cards: (file name, text).
pub const CARDS: [(&str, &str); 6] = [
    ("apache.md", include_str!("../fixtures/cards/apache.md")),
    ("mit.md", include_str!("../fixtures/cards/mit.md")),
    ("llama.md", include_str!("../fixtures/cards/llama.md")),
    ("other.md", include_str!("../fixtures/cards/other.md")),
    ("missing.md", include_str!("../fixtures/cards/missing.md")),
    (
        "nofrontmatter.md",
        include_str!("../fixtures/cards/nofrontmatter.md"),
    ),
];

pub struct Summary {
    pub rows: Vec<(String, Card, Finding)>,
}

/// Check every recorded card for a product with `monthly_active_users`.
pub fn run(monthly_active_users: u64, out: &mut dyn std::io::Write) -> std::io::Result<Summary> {
    let mut rows = vec![];
    for (name, text) in CARDS {
        let card = parse_card(text);
        let f = check(&card, monthly_active_users);
        writeln!(out, "{name}: {:?}", f.verdict)?;
        for r in &f.reasons {
            writeln!(out, "  why: {r}")?;
        }
        for o in &f.obligations {
            writeln!(out, "  must: {o}")?;
        }
        rows.push((name.to_string(), card, f));
    }
    writeln!(out, "sources:")?;
    write!(
        out,
        "{}",
        sources::render(&["apache-2.0", "mit", "llama-3.1", "hf-model-cards"])
    )?;
    Ok(Summary { rows })
}

impl Summary {
    /// Every finding is sound, a card with no licence is denied, and all three verdicts
    /// appear so the contract is not vacuous.
    pub fn contract_holds(&self) -> bool {
        let sound = self.rows.iter().all(|(_, c, f)| finding_is_sound(c, f));
        let unlicensed_denied = self
            .rows
            .iter()
            .filter(|(_, c, _)| c.license.is_none())
            .all(|(_, _, f)| f.verdict == Verdict::Deny);
        let has = |v| self.rows.iter().any(|(_, _, f)| f.verdict == v);
        sound
            && unlicensed_denied
            && has(Verdict::Allow)
            && has(Verdict::Review)
            && has(Verdict::Deny)
    }
}

#[cfg(test)]
mod tests {
    struct FailAt(usize, usize);
    impl std::io::Write for FailAt {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.1 += 1;
            if self.1 > self.0 {
                Err(std::io::Error::other("full"))
            } else {
                Ok(b.len())
            }
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_write_failure_at_any_point_is_an_error() {
        std::io::Write::flush(&mut FailAt(0, 0)).unwrap();
        let mut ok = FailAt(usize::MAX, 0);
        run(1_000, &mut ok).unwrap();
        for n in 0..ok.1 {
            assert!(run(1_000, &mut FailAt(n, 0)).is_err(), "write {n}");
        }
    }

    use super::*;

    fn card(l: Option<&str>) -> Card {
        Card {
            license: l.map(String::from),
            base_model: None,
        }
    }

    #[test]
    fn parses_license_and_base_model_and_strips_quotes() {
        let c = parse_card("---\nlicense: \"Apache-2.0\"\nbase_model: a/b\nx: y\nnocolon\nlicense_name: z\n---\nlicense: mit\n");
        assert_eq!(c.license.as_deref(), Some("apache-2.0"));
        assert_eq!(c.base_model.as_deref(), Some("a/b"));
    }

    #[test]
    fn empty_values_and_missing_blocks_leave_fields_unset() {
        assert_eq!(parse_card("---\nlicense:\n---\n").license, None);
        assert_eq!(parse_card("# no block\nlicense: mit\n").license, None);
        assert_eq!(parse_card("").license, None);
    }

    #[test]
    fn no_licence_is_denied() {
        let f = check(&card(None), 0);
        assert_eq!(f.verdict, Verdict::Deny);
    }

    #[test]
    fn permissive_is_allowed_with_obligations() {
        let f = check(&card(Some("apache-2.0")), 0);
        assert_eq!(f.verdict, Verdict::Allow);
        assert_eq!(f.obligations.len(), 2);
    }

    #[test]
    fn unknown_licence_is_reviewed_never_allowed() {
        let f = check(&card(Some("other")), 0);
        assert_eq!(f.verdict, Verdict::Review);
        assert!(f.obligations.is_empty());
    }

    #[test]
    fn llama_threshold_is_exact() {
        let at = check(&card(Some("llama3.1")), LLAMA_MAU_LIMIT);
        let over = check(&card(Some("llama3.1")), LLAMA_MAU_LIMIT + 1);
        assert_eq!(at.verdict, Verdict::Review);
        assert_eq!(over.verdict, Verdict::Deny);
        assert_eq!(at.obligations.len(), 3);
    }

    #[test]
    fn every_policy_cites_a_known_source() {
        for p in &POLICIES {
            assert!(sources::find(p.source).is_some(), "{}", p.id);
        }
    }

    #[test]
    fn soundness_refuses_an_allow_for_an_unlicensed_or_custom_card() {
        let allow = |r: &[&str]| Finding {
            verdict: Verdict::Allow,
            reasons: r.iter().map(|s| s.to_string()).collect(),
            obligations: vec![],
        };
        assert!(!finding_is_sound(&card(None), &allow(&["x"])));
        assert!(!finding_is_sound(&card(Some("llama3.1")), &allow(&["x"])));
        assert!(!finding_is_sound(&card(Some("nope")), &allow(&["x"])));
        assert!(finding_is_sound(&card(Some("mit")), &allow(&["x"])));
    }

    #[test]
    fn soundness_wants_a_reason_for_review_and_deny() {
        let f = |v| Finding {
            verdict: v,
            reasons: vec![],
            obligations: vec![],
        };
        assert!(!finding_is_sound(&card(Some("mit")), &f(Verdict::Review)));
        assert!(!finding_is_sound(&card(None), &f(Verdict::Deny)));
        let mut r = f(Verdict::Review);
        r.reasons.push("x".into());
        assert!(!finding_is_sound(&card(None), &r));
        assert!(finding_is_sound(&card(Some("mit")), &r));
    }

    fn summary() -> Summary {
        run(1_000, &mut Vec::new()).unwrap()
    }

    #[test]
    fn the_recorded_cards_satisfy_the_contract() {
        let mut out = Vec::new();
        let s = run(1_000, &mut out).unwrap();
        assert!(s.contract_holds());
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("missing.md: Deny"));
        assert!(text.contains("apache.md: Allow"));
        assert!(text.contains("[NOT VERIFIED] <https://spdx.org/licenses/MIT.html>"));
    }

    #[test]
    fn contract_fails_when_an_unlicensed_card_is_allowed() {
        let mut s = summary();
        for (_, c, f) in s.rows.iter_mut() {
            if c.license.is_none() {
                f.verdict = Verdict::Allow;
            }
        }
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_when_a_verdict_never_appears() {
        let mut s = summary();
        s.rows.retain(|(_, _, f)| f.verdict != Verdict::Review);
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_fails_when_a_custom_licence_is_allowed() {
        let mut s = summary();
        for (_, c, f) in s.rows.iter_mut() {
            if c.license.as_deref() == Some("llama3.1") {
                f.verdict = Verdict::Allow;
            }
        }
        assert!(!s.contract_holds());
    }
}
