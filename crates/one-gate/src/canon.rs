//! The NARROW CANONICAL FORM. This check does not model GitHub: it accepts one small shape and
//! makes everything else RED. Nothing here matches a glob, runs a shell or interprets a tag.
//!
//! * Directory: every entry under `.github/workflows` is a regular file (not a symlink) named
//!   `[a-z0-9][a-z0-9._-]*.yml` or `.yaml`; anything else is RED.
//! * Document: ONE YAML document; no tag anywhere (keys included) and no merge key.
//! * Keys: workflow top level, and job and step level of a workflow that can gate, are
//!   allow-listed. `env`, `defaults` and `run-name` pass only when their whole rendered value is
//!   byte-equal to a `value` entry of `.github/one-gate-allow.txt` for that file and scope.
//! * Filters: `branches`, `branches-ignore`, `tags`, `paths`, `paths-ignore` and `types` are never
//!   matched. Each entry must be byte-equal to a `filter` entry that states its class by hand;
//!   only the literal `main` under `branches` is understood without one.

use crate::allow::Allow;
use serde_yaml_ng::{Mapping, Value};
use std::path::Path;

pub const TOP_KEYS: &[&str] = &["name", "on", "permissions", "concurrency", "jobs"];
pub const VALUE_KEYS: &[&str] = &["env", "defaults", "run-name"];
pub const JOB_KEYS: &[&str] = &[
    "name",
    "runs-on",
    "needs",
    "if",
    "steps",
    "timeout-minutes",
    "strategy",
    "permissions",
    "continue-on-error",
    "uses",
    "with",
];
pub const STEP_KEYS: &[&str] = &[
    "name",
    "id",
    "if",
    "run",
    "uses",
    "with",
    "timeout-minutes",
    "continue-on-error",
];
pub const EVENT_KEYS: &[&str] = &[
    "branches",
    "branches-ignore",
    "tags",
    "tags-ignore",
    "paths",
    "paths-ignore",
    "types",
];

/// A workflow file name in the canonical form.
pub fn valid_name(name: &str) -> bool {
    let stem = name
        .strip_suffix(".yaml")
        .or_else(|| name.strip_suffix(".yml"));
    stem.is_some_and(|s| {
        s.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "._-".contains(c))
    })
}

/// Every entry of the workflows directory: the canonical names to read, and a violation for each
/// entry that is not a canonical workflow file.
pub fn list_dir(dir: &Path) -> (Vec<String>, Vec<String>) {
    let (mut names, mut bad) = (Vec::new(), Vec::new());
    let rd = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) => return (names, vec![format!("{}: {e}", dir.display())]),
    };
    for ent in rd {
        let path = ent.map(|e| e.path()).unwrap_or_default();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let regular = path.symlink_metadata().is_ok_and(|m| m.is_file());
        if regular && valid_name(name) {
            names.push(name.to_owned());
        } else {
            let shown = path.file_name().unwrap_or_default();
            bad.push(format!(
                ".github/workflows/{shown:?}: not a regular file named [a-z0-9][a-z0-9._-]* with .yml or .yaml; the check reads nothing else, so it refuses it"
            ));
        }
    }
    names.sort();
    (names, bad)
}

/// RED on a tag, a merge key or a non-string-able key anywhere in the document.
pub fn plain_yaml(v: &Value) -> Result<(), String> {
    match v {
        Value::Tagged(t) => Err(format!("custom tag '{}' is not allowed", t.tag)),
        Value::Sequence(q) => q.iter().try_for_each(plain_yaml),
        Value::Mapping(m) => {
            for (k, x) in m {
                if k.as_str() == Some("<<") {
                    return Err("merge key '<<' is not allowed".to_owned());
                }
                plain_yaml(k)?;
                plain_yaml(x)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// One-line, deterministic rendering of a value: what a `value` allow entry is compared to.
pub fn render(v: &Value) -> String {
    match v {
        Value::Null | Value::Tagged(_) => "null".to_owned(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("{s:?}"),
        Value::Sequence(q) => format!("[{}]", q.iter().map(render).collect::<Vec<_>>().join(",")),
        Value::Mapping(m) => format!(
            "{{{}}}",
            m.iter()
                .map(|(k, x)| format!("{}:{}", render(k), render(x)))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn keys_in(
    m: &Mapping,
    allowed: &[&str],
    at: &str,
    allow: &Allow,
    scope: &str,
    file: &str,
) -> Vec<String> {
    let mut out = Vec::new();
    for (k, v) in m {
        match k.as_str() {
            Some(s) if allowed.contains(&s) => {}
            Some(s) if VALUE_KEYS.contains(&s) => {
                if !allow.value_ok(file, scope, s, &render(v)) {
                    out.push(format!(
                        "{at}: '{s}' is not byte-equal to a value entry for {scope} in .github/one-gate-allow.txt"
                    ));
                }
            }
            _ => out.push(format!(
                "{at}: key {} is not in the committed key allow-list",
                render(k)
            )),
        }
    }
    out
}

/// Top-level keys of any workflow.
pub fn top_keys(file: &str, top: &Mapping, allow: &Allow) -> Vec<String> {
    keys_in(top, TOP_KEYS, "workflow top level", allow, "workflow", file)
}

/// Job-level keys, then step-level keys, of a job in a workflow that can gate.
pub fn job_keys(file: &str, id: &str, job: &Mapping, allow: &Allow) -> Vec<String> {
    let at = format!("job '{id}'");
    let mut out = keys_in(job, JOB_KEYS, &at, allow, &format!("job:{id}"), file);
    if let Some(steps) = job.get("steps").and_then(Value::as_sequence) {
        for (i, s) in steps
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_mapping().map(|m| (i, m)))
        {
            let at = format!("job '{id}' step {i}");
            out.extend(keys_in(
                s,
                STEP_KEYS,
                &at,
                allow,
                &format!("step:{id}:{i}"),
                file,
            ));
        }
    }
    out
}

/// Per filter key, each entry with its hand-stated class (true = yes).
pub type Filters = Vec<(String, Vec<(String, bool)>)>;

/// A trigger config's filter keys, each entry byte-matched against the allow-list.
/// Returns, per key, each entry with its hand-stated class (true = yes), or why not.
pub fn filters(file: &str, event: &str, cfg: &Value, allow: &Allow) -> Result<Filters, String> {
    let m = match cfg {
        Value::Null => return Ok(Vec::new()),
        Value::Mapping(m) => m,
        _ => return Err(format!("'{event}' config is not a mapping")),
    };
    let mut out = Vec::new();
    for (k, v) in m {
        let key = k
            .as_str()
            .filter(|k| EVENT_KEYS.contains(k))
            .ok_or_else(|| format!("'{event}' has a key outside the filter allow-list"))?;
        let entries = match v {
            Value::String(s) => vec![s.clone()],
            Value::Sequence(q) if !q.is_empty() => q
                .iter()
                .map(|x| x.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| format!("'{event}.{key}' has an entry that is not a string"))?,
            _ => {
                return Err(format!(
                    "'{event}.{key}' is not a non-empty string or list of strings"
                ))
            }
        };
        let mut classes = Vec::new();
        for e in entries {
            let class = if key == "branches" && e == "main" {
                Some(true)
            } else {
                allow.filter_class(file, event, key, &e)
            };
            classes.push((e.clone(), class.ok_or_else(|| {
                format!("'{event}.{key}' entry '{e}' is not byte-equal to a filter entry in .github/one-gate-allow.txt; patterns are never matched")
            })?));
        }
        out.push((key.to_owned(), classes));
    }
    Ok(out)
}
