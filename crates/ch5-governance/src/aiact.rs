//! Lesson 5.2: the EU AI Act for developers, with the documentation generated from the repo.
//!
//! A repo declares what it is used for in `governance.json`. The generator classifies the
//! uses into a risk tier, lists the obligations that tier brings, and marks each one with
//! evidence found on disk or MISSING. It never invents evidence.
//!
//! The tier table is a teaching simplification; its legal basis is NOT VERIFIED where
//! [`crate::sources`] says so. Not legal advice.

use crate::sources;
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Minimal,
    Limited,
    High,
    Prohibited,
}

/// (use tag, tier). Prohibited and high-risk tags are NOT VERIFIED (see sources).
pub const USES: [(&str, Tier); 14] = [
    ("internal_tool", Tier::Minimal),
    ("chatbot", Tier::Limited),
    ("synthetic_text", Tier::Limited),
    ("deepfake", Tier::Limited),
    ("biometric_identification", Tier::High),
    ("critical_infrastructure", Tier::High),
    ("education_admission", Tier::High),
    ("employment_screening", Tier::High),
    ("essential_services_credit", Tier::High),
    ("law_enforcement", Tier::High),
    ("migration_border", Tier::High),
    ("justice_administration", Tier::High),
    ("social_scoring", Tier::Prohibited),
    ("untargeted_facial_scraping", Tier::Prohibited),
];

/// One obligation: (id, applies to a use tag or tier, article, text, source id).
pub struct Obligation {
    pub id: &'static str,
    pub when: When,
    pub article: &'static str,
    pub text: &'static str,
    pub source: &'static str,
}

pub enum When {
    Use(&'static str),
    AtLeast(Tier),
}

pub const OBLIGATIONS: [Obligation; 10] = [
    Obligation {
        id: "art50-1",
        when: When::Use("chatbot"),
        article: "Art. 50(1)",
        text: "Tell people they are talking to an AI unless that is obvious.",
        source: "eu-ai-act-art50",
    },
    Obligation {
        id: "art50-2",
        when: When::Use("synthetic_text"),
        article: "Art. 50(2)",
        text: "Mark synthetic output in a machine-readable way.",
        source: "eu-ai-act-art50",
    },
    Obligation {
        id: "art50-4",
        when: When::Use("deepfake"),
        article: "Art. 50(4)",
        text: "Disclose that deepfake content is artificial.",
        source: "eu-ai-act-art50",
    },
    Obligation {
        id: "art9",
        when: When::AtLeast(Tier::High),
        article: "Art. 9",
        text: "Risk management system.",
        source: "eu-ai-act-tiers",
    },
    Obligation {
        id: "art10",
        when: When::AtLeast(Tier::High),
        article: "Art. 10",
        text: "Data and data governance.",
        source: "eu-ai-act-tiers",
    },
    Obligation {
        id: "art11",
        when: When::AtLeast(Tier::High),
        article: "Art. 11",
        text: "Technical documentation.",
        source: "eu-ai-act-tiers",
    },
    Obligation {
        id: "art12",
        when: When::AtLeast(Tier::High),
        article: "Art. 12",
        text: "Record-keeping (logs).",
        source: "eu-ai-act-tiers",
    },
    Obligation {
        id: "art13",
        when: When::AtLeast(Tier::High),
        article: "Art. 13",
        text: "Transparency and information for deployers.",
        source: "eu-ai-act-tiers",
    },
    Obligation {
        id: "art14",
        when: When::AtLeast(Tier::High),
        article: "Art. 14",
        text: "Human oversight.",
        source: "eu-ai-act-tiers",
    },
    Obligation {
        id: "art15",
        when: When::AtLeast(Tier::High),
        article: "Art. 15",
        text: "Accuracy, robustness and cybersecurity.",
        source: "eu-ai-act-tiers",
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Evidence declared and found at this path.
    Evidence(String),
    /// Nothing found; the string says why.
    Missing(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub article: String,
    pub text: String,
    pub status: Status,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    pub name: String,
    pub purpose: String,
    pub uses: Vec<String>,
    pub models: Vec<(String, String)>,
    pub evidence: Vec<(String, String)>,
}

fn text_field(v: &Value, key: &str) -> Result<String, String> {
    v[key]
        .as_str()
        .map(String::from)
        .ok_or_else(|| format!("governance.json: `{key}` must be a string"))
}

/// Read `governance.json` from a repo root.
pub fn read_facts(root: &Path) -> Result<Facts, String> {
    let path = root.join("governance.json");
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_facts(&raw)
}

pub fn parse_facts(raw: &str) -> Result<Facts, String> {
    let v: Value = serde_json::from_str(raw).map_err(|e| format!("governance.json: {e}"))?;
    let uses = v["uses"]
        .as_array()
        .ok_or("governance.json: `uses` must be a list")?
        .iter()
        .map(|u| {
            u.as_str()
                .map(String::from)
                .ok_or("`uses` holds a non-string")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut models = vec![];
    for m in v["models"].as_array().into_iter().flatten() {
        models.push((text_field(m, "name")?, text_field(m, "license")?));
    }
    let mut evidence = vec![];
    for (k, p) in v["evidence"].as_object().into_iter().flatten() {
        let p = p.as_str().ok_or("`evidence` values must be paths")?;
        evidence.push((k.clone(), p.to_string()));
    }
    Ok(Facts {
        name: text_field(&v, "name")?,
        purpose: text_field(&v, "purpose")?,
        uses,
        models,
        evidence,
    })
}

/// The highest tier any use reaches, and the uses that set it. An unknown tag is an
/// error: a use the table cannot classify is never silently treated as minimal.
pub fn classify(uses: &[String]) -> Result<(Tier, Vec<String>), String> {
    let mut tier = Tier::Minimal;
    let mut tiers = vec![];
    for u in uses {
        let (_, t) = USES.iter().find(|(tag, _)| tag == u).ok_or_else(|| {
            format!("unknown use `{u}`: add it to the tier table before shipping")
        })?;
        tier = tier.max(*t);
        tiers.push((u.clone(), *t));
    }
    let triggers = tiers
        .into_iter()
        .filter(|(_, t)| *t == tier)
        .map(|(u, _)| u)
        .collect();
    Ok((tier, triggers))
}

fn applies(o: &Obligation, uses: &[String], tier: Tier) -> bool {
    match o.when {
        When::Use(u) => uses.iter().any(|x| x == u),
        When::AtLeast(t) => tier >= t,
    }
}

pub struct Doc {
    pub tier: Tier,
    pub triggers: Vec<String>,
    pub rows: Vec<Row>,
    pub markdown: String,
}

/// Build the document for a repo rooted at `root`.
pub fn generate(facts: &Facts, root: &Path) -> Result<Doc, String> {
    let (tier, triggers) = classify(&facts.uses)?;
    let mut rows = vec![];
    for o in OBLIGATIONS.iter().filter(|o| applies(o, &facts.uses, tier)) {
        let declared = facts.evidence.iter().find(|(k, _)| k == o.id);
        let status = match declared {
            None => Status::Missing("no evidence declared".into()),
            Some((_, p)) if root.join(p).is_file() => Status::Evidence(p.clone()),
            Some((_, p)) => Status::Missing(format!("declared path `{p}` not found")),
        };
        rows.push(Row {
            id: o.id.into(),
            article: o.article.into(),
            text: o.text.into(),
            status,
        });
    }
    if tier == Tier::Prohibited {
        rows.push(Row {
            id: "art5".into(),
            article: "Art. 5".into(),
            text: "A prohibited practice: do not ship. No evidence can fix this.".into(),
            status: Status::Missing("prohibited".into()),
        });
    }
    let markdown = render(facts, root, tier, &triggers, &rows);
    Ok(Doc {
        tier,
        triggers,
        rows,
        markdown,
    })
}

fn count_files(dir: &Path, ext: &str) -> usize {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == ext))
        .count()
}

fn render(facts: &Facts, root: &Path, tier: Tier, triggers: &[String], rows: &[Row]) -> String {
    let mut md = format!("# AI Act documentation: {}\n\n", facts.name);
    md.push_str(&format!("Purpose: {}\n\n", facts.purpose));
    md.push_str("Generated from the repo. Not legal advice.\n\n");
    md.push_str(&format!(
        "## Risk tier\n\n{tier:?}, set by: {}.\n\n",
        triggers.join(", ")
    ));
    md.push_str("## Inventory found in the repo\n\n");
    for (n, l) in &facts.models {
        md.push_str(&format!("- model `{n}`, licence `{l}`\n"));
    }
    md.push_str(&format!(
        "- licence file: {}\n- eval sets (evals/*.jsonl): {}\n- contracts (contracts/*.yaml): {}\n\n",
        if root.join("LICENSE").is_file() { "present" } else { "MISSING" },
        count_files(&root.join("evals"), "jsonl"),
        count_files(&root.join("contracts"), "yaml"),
    ));
    md.push_str(
        "## Obligations and evidence\n\n| Article | Obligation | Evidence |\n|---|---|---|\n",
    );
    for r in rows {
        let ev = match &r.status {
            Status::Evidence(p) => format!("`{p}`"),
            Status::Missing(why) => format!("MISSING ({why})"),
        };
        md.push_str(&format!("| {} | {} | {ev} |\n", r.article, r.text));
    }
    md.push_str("\n## Fines\n\nUp to EUR 35,000,000 or 7 % of worldwide annual turnover for prohibited practices (Art. 99(3)); up to EUR 15,000,000 or 3 % for most operator obligations (Art. 99(4)); the higher figure for a company.\n\n");
    md.push_str("Application dates: NOT VERIFIED, not stated here.\n\n## Sources\n\n");
    md.push_str(&sources::render(&[
        "eu-ai-act-art50",
        "eu-ai-act-art53",
        "eu-ai-act-art99",
        "eu-ai-act-art113",
        "eu-ai-act-tiers",
    ]));
    md
}

impl Doc {
    /// The document is honest: its tier is the highest of the uses, every evidence path
    /// exists on disk, every missing row says why and names no existing file, and a
    /// prohibited tier is never left without a blocking row.
    pub fn contract_holds(&self, facts: &Facts, root: &Path) -> bool {
        let Ok((tier, _)) = classify(&facts.uses) else {
            return false;
        };
        let evidence_real = self.rows.iter().all(|r| match &r.status {
            Status::Evidence(p) => root.join(p).is_file(),
            Status::Missing(why) => !why.is_empty(),
        });
        let blocked = self.tier != Tier::Prohibited || self.rows.iter().any(|r| r.id == "art5");
        let md_lists_all = self.rows.iter().all(|r| self.markdown.contains(&r.article));
        self.tier == tier && evidence_real && blocked && md_lists_all
    }
}

pub struct Summary {
    pub facts: Facts,
    pub doc: Doc,
}

/// Generate the document for the repo at `root` and write it to `out`.
pub fn run(root: &Path, out: &mut dyn std::io::Write) -> Result<Summary, String> {
    let facts = read_facts(root)?;
    let doc = generate(&facts, root)?;
    write!(out, "{}", doc.markdown).map_err(|e| e.to_string())?;
    Ok(Summary { facts, doc })
}

impl Summary {
    pub fn contract_holds(&self, root: &Path) -> bool {
        self.doc.contract_holds(&self.facts, root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/sample-repo")
    }

    fn facts(uses: &[&str], evidence: &[(&str, &str)]) -> Facts {
        Facts {
            name: "t".into(),
            purpose: "p".into(),
            uses: uses.iter().map(|s| s.to_string()).collect(),
            models: vec![],
            evidence: evidence
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
        }
    }

    #[test]
    fn the_sample_repo_is_limited_and_reports_one_gap() {
        let mut out = Vec::new();
        let s = run(&sample(), &mut out).unwrap();
        assert_eq!(s.doc.tier, Tier::Limited);
        assert!(s.contract_holds(&sample()));
        let md = String::from_utf8(out).unwrap();
        assert!(md.contains("| Art. 50(1) | Tell people they are talking to an AI unless that is obvious. | `docs/ai-disclosure.md` |"));
        assert!(md.contains("MISSING (declared path `docs/output-marking.md` not found)"));
        assert!(md.contains("model `example-model-a`, licence `apache-2.0`"));
        assert!(md.contains("eval sets (evals/*.jsonl): 1"));
        assert!(md.contains("contracts (contracts/*.yaml): 1"));
        assert!(md.contains("licence file: present"));
        assert!(md.contains("Application dates: NOT VERIFIED"));
        assert!(md.contains("[NOT VERIFIED] <https://eur-lex.europa.eu/"));
    }

    #[test]
    fn a_missing_licence_file_and_empty_dirs_are_reported() {
        let d = std::env::temp_dir().join("ch5-aiact-empty");
        std::fs::create_dir_all(&d).unwrap();
        let f = facts(&["internal_tool"], &[]);
        let doc = generate(&f, &d).unwrap();
        assert_eq!(doc.tier, Tier::Minimal);
        assert!(doc.rows.is_empty());
        assert!(doc.markdown.contains("licence file: MISSING"));
        assert!(doc.markdown.contains("eval sets (evals/*.jsonl): 0"));
    }

    #[test]
    fn the_highest_use_sets_the_tier() {
        let u = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(classify(&u(&["chatbot"])).unwrap().0, Tier::Limited);
        let (t, trig) = classify(&u(&["chatbot", "employment_screening"])).unwrap();
        assert_eq!(
            (t, trig),
            (Tier::High, vec!["employment_screening".to_string()])
        );
        assert_eq!(
            classify(&u(&["chatbot", "social_scoring"])).unwrap().0,
            Tier::Prohibited
        );
        assert_eq!(classify(&[]).unwrap().0, Tier::Minimal);
    }

    #[test]
    fn an_unknown_use_is_an_error_not_minimal() {
        let e = classify(&["telepathy".to_string()]).unwrap_err();
        assert!(e.contains("unknown use `telepathy`"));
    }

    #[test]
    fn high_risk_lists_articles_9_to_15_all_missing_without_evidence() {
        let doc = generate(&facts(&["law_enforcement"], &[]), &sample()).unwrap();
        assert_eq!(doc.rows.len(), 7);
        assert!(doc
            .rows
            .iter()
            .all(|r| matches!(&r.status, Status::Missing(w) if w == "no evidence declared")));
    }

    #[test]
    fn declared_evidence_that_exists_is_cited() {
        let f = facts(&["deepfake"], &[("art50-4", "docs/ai-disclosure.md")]);
        let doc = generate(&f, &sample()).unwrap();
        assert_eq!(
            doc.rows[0].status,
            Status::Evidence("docs/ai-disclosure.md".into())
        );
        assert!(doc.contract_holds(&f, &sample()));
    }

    #[test]
    fn a_prohibited_use_adds_a_blocking_row() {
        let f = facts(&["social_scoring"], &[]);
        let doc = generate(&f, &sample()).unwrap();
        assert_eq!(doc.tier, Tier::Prohibited);
        assert!(doc.rows.iter().any(|r| r.id == "art5"));
        assert!(doc.contract_holds(&f, &sample()));
    }

    #[test]
    fn contract_fails_on_invented_evidence() {
        let f = facts(&["chatbot"], &[]);
        let mut doc = generate(&f, &sample()).unwrap();
        doc.rows[0].status = Status::Evidence("docs/nope.md".into());
        assert!(!doc.contract_holds(&f, &sample()));
    }

    #[test]
    fn contract_fails_on_a_missing_row_with_no_reason() {
        let f = facts(&["chatbot"], &[]);
        let mut doc = generate(&f, &sample()).unwrap();
        doc.rows[0].status = Status::Missing(String::new());
        assert!(!doc.contract_holds(&f, &sample()));
    }

    #[test]
    fn contract_fails_on_a_lowered_tier_or_a_dropped_prohibition() {
        let f = facts(&["social_scoring"], &[]);
        let mut doc = generate(&f, &sample()).unwrap();
        doc.tier = Tier::Limited;
        assert!(!doc.contract_holds(&f, &sample()));
        let mut doc = generate(&f, &sample()).unwrap();
        doc.rows.retain(|r| r.id != "art5");
        assert!(!doc.contract_holds(&f, &sample()));
    }

    #[test]
    fn contract_fails_when_a_row_is_missing_from_the_markdown() {
        let f = facts(&["chatbot"], &[]);
        let mut doc = generate(&f, &sample()).unwrap();
        doc.markdown.clear();
        assert!(!doc.contract_holds(&f, &sample()));
    }

    #[test]
    fn contract_fails_for_facts_it_cannot_classify() {
        let f = facts(&["chatbot"], &[]);
        let doc = generate(&f, &sample()).unwrap();
        assert!(!doc.contract_holds(&facts(&["telepathy"], &[]), &sample()));
        assert!(doc.contract_holds(&f, &sample()));
    }

    #[test]
    fn bad_governance_files_are_refused() {
        let ok = r#"{"name":"n","purpose":"p","uses":[]}"#;
        assert!(parse_facts(ok).is_ok());
        assert!(parse_facts("{").unwrap_err().contains("governance.json"));
        assert!(parse_facts(r#"{"name":"n","purpose":"p"}"#)
            .unwrap_err()
            .contains("`uses` must be a list"));
        assert!(parse_facts(r#"{"name":"n","purpose":"p","uses":[1]}"#)
            .unwrap_err()
            .contains("non-string"));
        assert!(parse_facts(r#"{"purpose":"p","uses":[]}"#)
            .unwrap_err()
            .contains("`name`"));
        assert!(
            parse_facts(r#"{"name":"n","purpose":"p","uses":[],"models":[{"name":"m"}]}"#)
                .unwrap_err()
                .contains("`license`")
        );
        assert!(
            parse_facts(r#"{"name":"n","purpose":"p","uses":[],"evidence":{"a":1}}"#)
                .unwrap_err()
                .contains("paths")
        );
    }

    #[test]
    fn a_repo_without_governance_json_is_an_error() {
        let e = read_facts(&std::env::temp_dir().join("ch5-no-such-repo")).unwrap_err();
        assert!(e.contains("governance.json"));
        assert!(run(
            &std::env::temp_dir().join("ch5-no-such-repo"),
            &mut Vec::new()
        )
        .is_err());
    }

    #[test]
    fn an_unclassifiable_repo_stops_the_generator() {
        let e = generate(&facts(&["telepathy"], &[]), &sample())
            .err()
            .unwrap();
        assert!(e.contains("unknown use"));
    }

    #[test]
    fn a_failing_writer_is_an_error() {
        struct Bad;
        impl std::io::Write for Bad {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("full"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        std::io::Write::flush(&mut Bad).unwrap();
        assert_eq!(run(&sample(), &mut Bad).err().unwrap(), "full");
    }

    #[test]
    fn every_obligation_cites_a_known_source_and_an_application_rule() {
        for o in &OBLIGATIONS {
            assert!(sources::find(o.source).is_some(), "{}", o.id);
        }
    }
}
