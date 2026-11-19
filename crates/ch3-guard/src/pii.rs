//! PII and secret detection and redaction (video 3.3).
//!
//! Detection is plain byte scanning: no regex engine, no model, no network.
//! Every span is ASCII, so slicing at a span edge is always a valid UTF-8 boundary.

/// What a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Email,
    Ssn,
    Card,
    Phone,
    Ipv4,
    ApiKey,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::Email,
        Kind::Ssn,
        Kind::Card,
        Kind::Phone,
        Kind::Ipv4,
        Kind::ApiKey,
    ];

    /// The lowercase name used in eval files and log lines.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Email => "email",
            Kind::Ssn => "ssn",
            Kind::Card => "card",
            Kind::Phone => "phone",
            Kind::Ipv4 => "ipv4",
            Kind::ApiKey => "api_key",
        }
    }

    pub fn from_label(s: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.label() == s)
    }

    /// The placeholder that replaces a span of this kind.
    pub fn placeholder(self) -> &'static str {
        match self {
            Kind::Email => "[EMAIL]",
            Kind::Ssn => "[SSN]",
            Kind::Card => "[CARD]",
            Kind::Phone => "[PHONE]",
            Kind::Ipv4 => "[IP]",
            Kind::ApiKey => "[API_KEY]",
        }
    }

    /// A secret (a credential) rather than personal data.
    pub fn is_secret(self) -> bool {
        self == Kind::ApiKey
    }
}

/// A span of the input, as byte offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
}

fn word(b: &[u8], i: usize) -> bool {
    b[i].is_ascii_alphanumeric()
}

fn boundary_before(b: &[u8], i: usize) -> bool {
    i == 0 || !word(b, i - 1)
}

fn boundary_after(b: &[u8], j: usize) -> bool {
    j >= b.len() || !word(b, j)
}

/// Does `pat` match at `i`? `d` is a digit, anything else is itself.
fn pat_at(b: &[u8], i: usize, pat: &str) -> bool {
    let p = pat.as_bytes();
    i + p.len() <= b.len()
        && p.iter().enumerate().all(|(k, &c)| {
            if c == b'd' {
                b[i + k].is_ascii_digit()
            } else {
                b[i + k] == c
            }
        })
}

fn scan_patterns(
    b: &[u8],
    pats: &[&str],
    kind: Kind,
    accept: fn(&[u8]) -> bool,
    out: &mut Vec<Finding>,
) {
    for i in 0..b.len() {
        if !boundary_before(b, i) {
            continue;
        }
        for p in pats {
            let end = i + p.len();
            if pat_at(b, i, p) && boundary_after(b, end) && accept(&b[i..end]) {
                out.push(Finding {
                    kind,
                    start: i,
                    end,
                });
            }
        }
    }
}

fn ssn_ok(s: &[u8]) -> bool {
    let area = &s[0..3];
    area != b"000" && area != b"666" && s[0] != b'9' && &s[4..6] != b"00" && &s[7..11] != b"0000"
}

fn always(_: &[u8]) -> bool {
    true
}

fn local_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || b"._%+-".contains(&c)
}

fn domain_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || b".-".contains(&c)
}

fn valid_domain(d: &[u8]) -> bool {
    let tld = d.rsplit(|&c| c == b'.').next().unwrap_or(&[]);
    d.contains(&b'.') && d[0] != b'.' && tld.len() >= 2 && tld.iter().all(u8::is_ascii_alphabetic)
}

fn email_at(b: &[u8], k: usize) -> Option<Finding> {
    let start = (0..k).rev().take_while(|&i| local_byte(b[i])).last()?;
    let run = b[k + 1..].iter().take_while(|&&c| domain_byte(c)).count();
    let keep = b[k + 1..k + 1 + run]
        .iter()
        .rposition(|c| !b".-".contains(c))
        .map_or(0, |p| p + 1);
    valid_domain(&b[k + 1..k + 1 + keep]).then_some(Finding {
        kind: Kind::Email,
        start,
        end: k + 1 + keep,
    })
}

fn email(b: &[u8], out: &mut Vec<Finding>) {
    let ats = (0..b.len()).filter(|&k| b[k] == b'@');
    out.extend(ats.filter_map(|k| email_at(b, k)));
}

fn api_key(b: &[u8], out: &mut Vec<Finding>) {
    // (prefix, minimum tail length, allowed tail byte)
    type Allowed = fn(u8) -> bool;
    let shapes: [(&[u8], usize, Allowed); 3] = [
        (b"sk-", 20, |c| {
            c.is_ascii_alphanumeric() || c == b'-' || c == b'_'
        }),
        (b"AKIA", 16, |c| {
            c.is_ascii_uppercase() || c.is_ascii_digit()
        }),
        (b"ghp_", 36, |c| c.is_ascii_alphanumeric()),
    ];
    for i in 0..b.len() {
        if !boundary_before(b, i) {
            continue;
        }
        for (prefix, min_tail, allowed) in &shapes {
            if !b[i..].starts_with(prefix) {
                continue;
            }
            let start = i + prefix.len();
            let tail = b[start..].iter().take_while(|&&c| allowed(c)).count();
            if tail >= *min_tail {
                out.push(Finding {
                    kind: Kind::ApiKey,
                    start: i,
                    end: start + tail,
                });
            }
        }
    }
}

fn luhn(digits: &[u8]) -> bool {
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| {
            let v = u32::from(d - b'0');
            if i % 2 == 1 {
                let x = v * 2;
                if x > 9 {
                    x - 9
                } else {
                    x
                }
            } else {
                v
            }
        })
        .sum();
    sum.is_multiple_of(10)
}

/// The digits from `i` (single space or dash separators allowed), up to 19, and where each ends.
fn card_walk(b: &[u8], i: usize) -> (Vec<u8>, Vec<usize>) {
    let mut digits = Vec::new();
    let mut ends = Vec::new();
    let mut p = i;
    while p < b.len() && digits.len() < 19 {
        let sep = b" -".contains(&b[p]) && b.get(p + 1).is_some_and(u8::is_ascii_digit);
        if b[p].is_ascii_digit() {
            digits.push(b[p]);
            ends.push(p + 1);
        } else if !sep {
            break;
        }
        p += 1;
    }
    (digits, ends)
}

/// The end of the longest Luhn-valid card number of 13 to 19 digits starting at `i`.
fn card_end(b: &[u8], i: usize) -> Option<usize> {
    let (digits, ends) = card_walk(b, i);
    (13..=digits.len())
        .rev()
        .find(|&n| luhn(&digits[..n]) && boundary_after(b, ends[n - 1]))
        .map(|n| ends[n - 1])
}

fn card(b: &[u8], out: &mut Vec<Finding>) {
    let mut i = 0;
    while i < b.len() {
        let start = b[i].is_ascii_digit() && boundary_before(b, i);
        match start.then(|| card_end(b, i)).flatten() {
            Some(end) => {
                out.push(Finding {
                    kind: Kind::Card,
                    start: i,
                    end,
                });
                i = end;
            }
            None => i += 1,
        }
    }
}

/// One octet (1 to 3 digits, at most 255) from `s`; returns where it ends.
fn octet_end(b: &[u8], s: usize) -> Option<usize> {
    let n = b[s..]
        .iter()
        .take(3)
        .take_while(|c| c.is_ascii_digit())
        .count();
    let v: u32 = std::str::from_utf8(&b[s..s + n]).ok()?.parse().ok()?;
    (v <= 255).then_some(s + n)
}

fn ipv4_end(b: &[u8], i: usize) -> Option<usize> {
    let mut p = octet_end(b, i)?;
    for _ in 0..3 {
        (b.get(p) == Some(&b'.')).then_some(())?;
        p = octet_end(b, p + 1)?;
    }
    let more_octet = p + 1 < b.len() && b[p] == b'.' && b[p + 1].is_ascii_digit();
    (boundary_after(b, p) && !more_octet).then_some(p)
}

fn ipv4(b: &[u8], out: &mut Vec<Finding>) {
    for i in 0..b.len() {
        let inside_dotted = i >= 2 && b[i - 1] == b'.' && b[i - 2].is_ascii_digit();
        if b[i].is_ascii_digit() && boundary_before(b, i) && !inside_dotted {
            if let Some(end) = ipv4_end(b, i) {
                out.push(Finding {
                    kind: Kind::Ipv4,
                    start: i,
                    end,
                });
            }
        }
    }
}

/// Every PII or secret span in `text`, in order, with no overlaps.
pub fn detect(text: &str) -> Vec<Finding> {
    let b = text.as_bytes();
    let mut all = Vec::new();
    email(b, &mut all);
    api_key(b, &mut all);
    scan_patterns(b, &["ddd-dd-dddd"], Kind::Ssn, ssn_ok, &mut all);
    card(b, &mut all);
    scan_patterns(
        b,
        &[
            "ddd-ddd-dddd",
            "ddd.ddd.dddd",
            "(ddd) ddd-dddd",
            "+1 ddd-ddd-dddd",
        ],
        Kind::Phone,
        always,
        &mut all,
    );
    ipv4(b, &mut all);
    all.sort_by(|a, c| a.start.cmp(&c.start).then(c.end.cmp(&a.end)));
    let mut kept: Vec<Finding> = Vec::new();
    for f in all {
        if kept.last().is_none_or(|l| f.start >= l.end) {
            kept.push(f);
        }
    }
    kept
}

/// `text` with every finding replaced by its placeholder, plus the findings (original offsets).
pub fn redact(text: &str) -> (String, Vec<Finding>) {
    let findings = detect(text);
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for f in &findings {
        out.push_str(&text[at..f.start]);
        out.push_str(f.kind.placeholder());
        at = f.end;
    }
    out.push_str(&text[at..]);
    (out, findings)
}

/// The kinds in a finding list, as a comma-joined label string ("-" when empty).
pub(crate) fn kinds_label(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return "-".to_string();
    }
    findings
        .iter()
        .map(|f| f.kind.label())
        .collect::<Vec<_>>()
        .join(",")
}

/// The log line for a prompt: counts and kinds, never the text.
pub fn log_line(text: &str) -> String {
    let findings = detect(text);
    format!(
        "prompt chars={} findings={} kinds={}",
        text.chars().count(),
        findings.len(),
        kinds_label(&findings)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(t: &str) -> Vec<Kind> {
        detect(t).iter().map(|f| f.kind).collect()
    }

    #[test]
    fn labels_round_trip_and_only_api_keys_are_secrets() {
        for k in Kind::ALL {
            assert_eq!(Kind::from_label(k.label()), Some(k));
            assert!(k.placeholder().starts_with('[') && k.placeholder().ends_with(']'));
            assert_eq!(k.is_secret(), k == Kind::ApiKey);
        }
        assert_eq!(Kind::from_label("passport"), None);
    }

    #[test]
    fn finds_an_email_and_rejects_near_misses() {
        assert_eq!(kinds("write to ada.l+x@example.co.uk now"), [Kind::Email]);
        let f = &detect("mail ada@example.com.")[0];
        assert_eq!((f.start, f.end), (5, 20));
        for no in [
            "a@b",
            "@example.com",
            "x@.com",
            "x@exa_mple.c0m",
            "x@example.c",
            "x @example.com",
        ] {
            assert!(kinds(no).is_empty(), "{no}");
        }
    }

    #[test]
    fn ssn_rules_refuse_impossible_numbers() {
        assert_eq!(kinds("ssn 123-45-6789."), [Kind::Ssn]);
        for no in [
            "000-12-3456",
            "666-12-3456",
            "912-12-3456",
            "123-00-4567",
            "123-45-0000",
            "123-45-67890",
            "x123-45-6789",
        ] {
            assert!(!kinds(no).contains(&Kind::Ssn), "{no}");
        }
    }

    #[test]
    fn card_needs_luhn_and_length() {
        assert_eq!(kinds("pay 4111 1111 1111 1111 ok"), [Kind::Card]);
        assert_eq!(kinds("pay 4111-1111-1111-1111"), [Kind::Card]);
        assert_eq!(kinds("pay 4111111111111111 12"), [Kind::Card]);
        assert_eq!(kinds("pay 4111111111111111"), [Kind::Card]);
        for no in [
            "order 4111111111111112",
            "id 123456789012",
            "ref 41111111111111111111111",
            "4111 1111 1111 1111x",
        ] {
            assert!(!kinds(no).contains(&Kind::Card), "{no}");
        }
    }

    #[test]
    fn luhn_checks_the_documented_test_numbers() {
        assert!(luhn(b"4111111111111111"));
        assert!(luhn(b"5500000000000004"));
        assert!(!luhn(b"4111111111111112"));
    }

    #[test]
    fn phone_shapes() {
        for yes in [
            "call 555-010-0199 now",
            "call 555.010.0199",
            "call (555) 010-0199",
            "call +1 555-010-0199",
        ] {
            assert_eq!(kinds(yes), [Kind::Phone], "{yes}");
        }
        for no in [
            "call 555-010-01990",
            "call 5550100199",
            "v555-010-0199x",
            "call (555)010-0199",
        ] {
            assert!(kinds(no).is_empty(), "{no}");
        }
    }

    #[test]
    fn plus_one_phone_is_one_span() {
        let f = detect("+1 555-010-0199");
        assert_eq!(f.len(), 1);
        assert_eq!((f[0].start, f[0].end), (0, 15));
    }

    #[test]
    fn ipv4_octets_are_bounded() {
        assert_eq!(kinds("host 192.168.0.1 up"), [Kind::Ipv4]);
        assert_eq!(kinds("host 10.0.0.255."), [Kind::Ipv4]);
        for no in [
            "host 256.1.1.1",
            "host 1.2.3",
            "host 1.2.3.4.5",
            "host 1.2.3.4567",
            "host 1.2.3.x",
            "x1.2.3.4",
        ] {
            assert!(!kinds(no).contains(&Kind::Ipv4), "{no}");
        }
    }

    #[test]
    fn api_keys_by_prefix_and_length() {
        assert_eq!(kinds("key sk-abcdefghijklmnopqrstuvwx"), [Kind::ApiKey]);
        assert_eq!(kinds("key AKIAIOSFODNN7EXAMPLE"), [Kind::ApiKey]);
        let gh = format!("key ghp_{}", "a1".repeat(18));
        assert_eq!(kinds(&gh), [Kind::ApiKey]);
        for no in [
            "key sk-short",
            "key AKIAshort",
            "key ghp_short",
            "key xsk-abcdefghijklmnopqrstuvwx",
        ] {
            assert!(kinds(no).is_empty(), "{no}");
        }
    }

    #[test]
    fn overlaps_keep_the_earlier_longer_span() {
        // The email local part contains a phone-shaped run; the email wins.
        assert_eq!(kinds("555-010-0199@example.com"), [Kind::Email]);
    }

    #[test]
    fn redact_replaces_spans_and_keeps_the_rest() {
        let (out, f) = redact("é mail ada@example.com or call 555-010-0199, thanks");
        assert_eq!(out, "é mail [EMAIL] or call [PHONE], thanks");
        assert_eq!(f.len(), 2);
        let (same, none) = redact("nothing to see");
        assert_eq!(same, "nothing to see");
        assert!(none.is_empty());
    }

    #[test]
    fn redaction_is_idempotent() {
        let (once, _) = redact("a@b.io 123-45-6789 sk-abcdefghijklmnopqrstuvwx");
        let (twice, f) = redact(&once);
        assert_eq!(once, twice);
        assert!(f.is_empty());
    }

    #[test]
    fn log_line_has_counts_and_kinds_but_not_the_text() {
        let l = log_line("mail ada@example.com");
        assert_eq!(l, "prompt chars=20 findings=1 kinds=email");
        assert!(!l.contains("ada"));
        assert_eq!(log_line("hi"), "prompt chars=2 findings=0 kinds=-");
        assert_eq!(kinds_label(&detect("a@b.io 123-45-6789")), "email,ssn");
    }
}
