//! Every legal or regulatory fact the demos state, with where it came from.
//!
//! `verified` is true only when the text was read at `url` on `accessed`. A second-hand
//! mirror is said so in `note`. Anything else is NOT VERIFIED and the demos print that.

/// One cited source.
pub struct Source {
    pub id: &'static str,
    pub url: &'static str,
    pub accessed: &'static str,
    pub verified: bool,
    pub note: &'static str,
}

/// The access date for every row below.
pub const ACCESSED: &str = "2026-11-19";

pub const SOURCES: [Source; 10] = [
    Source {
        id: "apache-2.0",
        url: "https://www.apache.org/licenses/LICENSE-2.0.txt",
        accessed: ACCESSED,
        verified: true,
        note: "Section 3 patent licence ends if you sue over the Work; Section 4 asks for the NOTICE file and a prominent note on files you changed.",
    },
    Source {
        id: "spdx-apache-2.0",
        url: "https://spdx.org/licenses/Apache-2.0.html",
        accessed: ACCESSED,
        verified: true,
        note: "SPDX identifier Apache-2.0 confirmed. OSI approval NOT VERIFIED on this page.",
    },
    Source {
        id: "mit",
        url: "https://spdx.org/licenses/MIT.html",
        accessed: ACCESSED,
        verified: false,
        note: "NOT VERIFIED: the MIT text was not read this session; it is treated as permissive from general knowledge.",
    },
    Source {
        id: "llama-3.1",
        url: "https://huggingface.co/meta-llama/Llama-3.1-8B/blob/main/LICENSE",
        accessed: ACCESSED,
        verified: true,
        note: "Section 2: a licensee whose products exceed 700 million monthly active users in the month before the release date must request a licence from Meta. Section 1.b.i: display \"Built with Llama\" and start the name of a derived model with \"Llama\". The Hub licence id `llama3.1` is NOT VERIFIED.",
    },
    Source {
        id: "hf-model-cards",
        url: "https://huggingface.co/docs/hub/model-cards",
        accessed: ACCESSED,
        verified: true,
        note: "A model card is README.md with a YAML block holding license, base_model, datasets; `license: other` plus license_name and license_link names a custom licence.",
    },
    Source {
        id: "eu-ai-act-art50",
        url: "https://artificialintelligenceact.eu/article/50/",
        accessed: ACCESSED,
        verified: true,
        note: "Unofficial mirror of Regulation (EU) 2024/1689, not the Official Journal. Art. 50(1): tell people they talk to an AI. 50(2): mark synthetic output machine-readably. 50(4): disclose deepfakes.",
    },
    Source {
        id: "eu-ai-act-art53",
        url: "https://artificialintelligenceact.eu/article/53/",
        accessed: ACCESSED,
        verified: true,
        note: "Unofficial mirror. Art. 53(1): providers of general-purpose AI models keep technical documentation, inform downstream providers, adopt a copyright policy and publish a training-content summary; 53(2) exempts (a) and (b) for open-source models without systemic risk. Whether a deployer of a local model is a provider is NOT VERIFIED.",
    },
    Source {
        id: "eu-ai-act-art99",
        url: "https://artificialintelligenceact.eu/article/99/",
        accessed: ACCESSED,
        verified: true,
        note: "Unofficial mirror. Art. 99(3): up to EUR 35,000,000 or 7 % of worldwide annual turnover for Art. 5 breaches. Art. 99(4): up to EUR 15,000,000 or 3 % for most operator obligations. Higher of the two for an undertaking.",
    },
    Source {
        id: "eu-ai-act-art113",
        url: "https://artificialintelligenceact.eu/article/113/",
        accessed: ACCESSED,
        verified: false,
        note: "NOT VERIFIED: the mirror shows tracked amendments to the application dates that disagree with each other. No date is stated by the demos; check the Official Journal.",
    },
    Source {
        id: "eu-ai-act-tiers",
        url: "https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:32024R1689",
        accessed: ACCESSED,
        verified: false,
        note: "NOT VERIFIED: the Art. 5 prohibited list, the Annex III high-risk areas and the Art. 9 to 15 requirement titles in the demo's tier table are from memory; the EUR-Lex page was fetched but its articles were not read.",
    },
];

/// Look a source up by id.
pub fn find(id: &str) -> Option<&'static Source> {
    SOURCES.iter().find(|s| s.id == id)
}

/// One markdown line per source, with the NOT VERIFIED flag where it applies.
pub fn render(ids: &[&str]) -> String {
    let mut out = String::new();
    for id in ids {
        if let Some(s) = find(id) {
            let flag = if s.verified { "read" } else { "NOT VERIFIED" };
            out.push_str(&format!(
                "- [{flag}] <{}> accessed {}. {}\n",
                s.url, s.accessed, s.note
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_source_has_an_https_url_and_the_access_date() {
        for s in &SOURCES {
            assert!(s.url.starts_with("https://"), "{}", s.id);
            assert_eq!(s.accessed, ACCESSED, "{}", s.id);
        }
    }

    #[test]
    fn ids_are_unique() {
        for (i, a) in SOURCES.iter().enumerate() {
            assert!(SOURCES[i + 1..].iter().all(|b| b.id != a.id), "{}", a.id);
        }
    }

    #[test]
    fn an_unverified_source_says_so_in_its_note() {
        for s in SOURCES.iter().filter(|s| !s.verified) {
            assert!(s.note.contains("NOT VERIFIED"), "{}", s.id);
        }
    }

    #[test]
    fn render_flags_unverified_and_skips_unknown_ids() {
        let text = render(&["mit", "apache-2.0", "no-such-source"]);
        assert!(text.contains("[NOT VERIFIED] <https://spdx.org/licenses/MIT.html>"));
        assert!(text.contains("[read] <https://www.apache.org/licenses/LICENSE-2.0.txt>"));
        assert_eq!(text.lines().count(), 2);
        assert!(find("no-such-source").is_none());
    }
}
