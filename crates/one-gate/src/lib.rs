//! ONE GATE (operator ruling 2026-11-19): one check set per repo. Every job that runs on push
//! to main must also be a required check on pull requests.
//!
//! Each workflow under `.github/workflows` is PARSED as YAML (aliases and merge keys resolved)
//! and the rule is applied to the parsed tree, never to lines of text. The check fails closed:
//! an unreadable, unparseable or trigger-less workflow is a violation.
//!
//! A workflow runs on push to main when its `on` has `push` with no branch filter or a filter
//! that matches `main`, or has `workflow_run` or `merge_group`. For every job of such a workflow:
//! (a) the workflow must also run on `pull_request` to main, unfiltered by paths, with the
//! default activity types; a job id exists in one workflow only, so a same-named job elsewhere
//! cannot stand in for it; (b) the job is in `.github/required-status-checks.txt` or in the `needs`
//! closure of a listed job; (c) it has no job-level `if` unless the text is byte-equal to a line
//! of `.github/one-gate-allowed-if.txt`; and it has no `continue-on-error` (a third state).

use regex::Regex;
use serde_yaml_ng::{Mapping, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const REQUIRED: &str = ".github/required-status-checks.txt";
const ALLOWED_IF: &str = ".github/one-gate-allowed-if.txt";
const WORKFLOWS: &str = ".github/workflows";

struct Job {
    id: String,
    plain_name: bool,
    needs: Vec<String>,
    cond: Option<Option<String>>,
    third_state: bool,
}

struct Workflow {
    file: String,
    main_side: bool,
    pr_side: bool,
    jobs: Vec<Job>,
}

fn str_list(v: &Value) -> Option<Vec<String>> {
    match v {
        Value::String(s) => Some(vec![s.clone()]),
        Value::Sequence(q) => q.iter().map(|x| x.as_str().map(str::to_owned)).collect(),
        _ => None,
    }
}

/// GitHub branch glob to an anchored regex: `**` any, `*` no slash, `?` and `+` repeat the
/// previous atom, `[..]` a class.
fn glob_matches(pat: &str, text: &str) -> Result<bool, String> {
    let mut re = String::from("^");
    let mut it = pat.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '*' if it.peek() == Some(&'*') => {
                it.next();
                re.push_str(".*");
            }
            '*' => re.push_str("[^/]*"),
            '?' | '+' | '[' | ']' => re.push(c),
            _ => re.push_str(&regex::escape(&c.to_string())),
        }
    }
    re.push('$');
    Regex::new(&re)
        .map(|r| r.is_match(text))
        .map_err(|e| format!("branch pattern '{pat}' is not understood: {e}"))
}

fn patterns(m: &Mapping, key: &str) -> Result<Option<Vec<String>>, String> {
    match m.get(key) {
        None => Ok(None),
        Some(v) => match str_list(v) {
            Some(p) if !p.is_empty() => Ok(Some(p)),
            _ => Err(format!(
                "'{key}' is not a non-empty string or list of strings"
            )),
        },
    }
}

/// Patterns are applied in order: a plain one selects `main`, a `!` one deselects it.
fn selected(pats: &[String]) -> Result<bool, String> {
    let mut hit = false;
    for pat in pats {
        match pat.strip_prefix('!') {
            Some(neg) => hit = hit && !glob_matches(neg, "main")?,
            None => hit = hit || glob_matches(pat, "main")?,
        }
    }
    Ok(hit)
}

fn ignored(pats: &[String]) -> Result<bool, String> {
    for pat in pats {
        if glob_matches(pat, "main")? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Does this event config (the value under `push` or `pull_request`) cover branch `main`?
fn covers_main(cfg: &Value) -> Result<bool, String> {
    let m = match cfg {
        Value::Null => return Ok(true),
        Value::Mapping(m) => m,
        _ => return Err("event config is not a mapping".to_owned()),
    };
    match (patterns(m, "branches")?, patterns(m, "branches-ignore")?) {
        (Some(_), Some(_)) => Err("both 'branches' and 'branches-ignore' are set".to_owned()),
        (Some(p), None) => selected(&p),
        (None, Some(p)) => ignored(&p).map(|i| !i),
        (None, None) => Ok(true),
    }
}

/// A pull_request trigger counts only if it covers main, has no path filter (a required check
/// that a path filter skips never reports) and its types include the default three.
fn pr_counts(cfg: &Value) -> Result<bool, String> {
    if !covers_main(cfg)? {
        return Ok(false);
    }
    let Some(m) = cfg.as_mapping() else {
        return Ok(true);
    };
    if m.contains_key("paths") || m.contains_key("paths-ignore") {
        return Ok(false);
    }
    Ok(match patterns(m, "types")? {
        None => true,
        Some(t) => ["opened", "synchronize", "reopened"]
            .iter()
            .all(|d| t.iter().any(|x| x == d)),
    })
}

fn find_on(top: &Mapping) -> Result<&Value, String> {
    let mut found = top
        .iter()
        .filter(|(k, _)| k.as_str() == Some("on") || **k == Value::Bool(true))
        .map(|(_, v)| v);
    match (found.next(), found.next()) {
        (Some(v), None) => Ok(v),
        (None, _) => Err("has no 'on' key".to_owned()),
        _ => Err("has more than one 'on' key".to_owned()),
    }
}

fn triggers(on: &Value) -> Result<BTreeMap<String, Value>, String> {
    let mut out = BTreeMap::new();
    match on {
        Value::String(s) => {
            out.insert(s.clone(), Value::Null);
        }
        Value::Sequence(q) => {
            for x in q {
                let s = x.as_str().ok_or("an 'on' list entry is not a string")?;
                out.insert(s.to_owned(), Value::Null);
            }
        }
        Value::Mapping(m) => {
            for (k, v) in m {
                let s = k.as_str().ok_or("an 'on' key is not a string")?;
                out.insert(s.to_owned(), v.clone());
            }
        }
        _ => return Err("'on' is not a string, list or mapping".to_owned()),
    }
    if out.is_empty() {
        return Err("'on' lists no triggers".to_owned());
    }
    Ok(out)
}

fn continue_on_error(m: &Mapping) -> bool {
    m.get("continue-on-error")
        .is_some_and(|v| *v != Value::Bool(false))
}

fn parse_job(id: &str, v: &Value) -> Result<Job, String> {
    let m = v
        .as_mapping()
        .ok_or_else(|| format!("job '{id}' is not a mapping"))?;
    let named = match m.get("name") {
        None => true,
        Some(Value::String(s)) => s == id,
        Some(_) => return Err(format!("job '{id}' has a name that is not a string")),
    };
    let matrixed = m
        .get("strategy")
        .and_then(Value::as_mapping)
        .is_some_and(|s| s.contains_key("matrix"));
    let needs = match m.get("needs") {
        None => Vec::new(),
        Some(n) => str_list(n).ok_or_else(|| format!("job '{id}' has unreadable needs"))?,
    };
    let steps_bad = m
        .get("steps")
        .and_then(Value::as_sequence)
        .is_some_and(|s| {
            s.iter()
                .any(|st| st.as_mapping().is_some_and(continue_on_error))
        });
    Ok(Job {
        id: id.to_owned(),
        plain_name: named && !matrixed,
        needs,
        cond: m.get("if").map(|c| c.as_str().map(str::to_owned)),
        third_state: continue_on_error(m) || steps_bad,
    })
}

fn parse(file: &str, text: &str) -> Result<Workflow, String> {
    let mut doc: Value = serde_yaml_ng::from_str(text).map_err(|e| e.to_string())?;
    doc.apply_merge().map_err(|e| e.to_string())?;
    let top = doc.as_mapping().ok_or("top level is not a mapping")?;
    let trig = triggers(find_on(top)?)?;
    let main_side = match trig.get("push") {
        Some(cfg) => covers_main(cfg)?,
        None => false,
    } || trig.contains_key("workflow_run")
        || trig.contains_key("merge_group");
    let pr_side = match trig.get("pull_request") {
        Some(cfg) => pr_counts(cfg)?,
        None => false,
    };
    let jobs = top
        .get("jobs")
        .and_then(Value::as_mapping)
        .ok_or("has no 'jobs' mapping")?;
    let mut out = Vec::new();
    for (k, v) in jobs {
        let id = k.as_str().ok_or("a job id is not a string")?;
        out.push(parse_job(id, v)?);
    }
    Ok(Workflow {
        file: file.to_owned(),
        main_side,
        pr_side,
        jobs: out,
    })
}

fn read_lines(path: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(text
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .map(str::to_owned)
        .collect())
}

fn workflow_files(root: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let dir = root.join(WORKFLOWS);
    let rd = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut files: Vec<_> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| matches!(p.extension().and_then(|x| x.to_str()), Some("yml" | "yaml")))
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(format!("{}: no workflow files", dir.display()));
    }
    Ok(files)
}

/// Every violation of the ONE GATE rule in the repository at `root`; empty means PASS.
pub fn check(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let required: BTreeSet<String> = read_lines(&root.join(REQUIRED))
        .unwrap_or_else(|e| {
            out.push(e);
            Vec::new()
        })
        .iter()
        .map(|l| l.trim().to_owned())
        .collect();
    if required.is_empty() {
        out.push(format!(
            "{REQUIRED} lists no checks; an empty list is not a gate"
        ));
    }
    let allowed: BTreeSet<String> = match root.join(ALLOWED_IF) {
        p if p.exists() => read_lines(&p)
            .unwrap_or_else(|e| {
                out.push(e);
                Vec::new()
            })
            .into_iter()
            .collect(),
        _ => BTreeSet::new(),
    };
    let mut wfs = Vec::new();
    match workflow_files(root) {
        Err(e) => out.push(e),
        Ok(files) => {
            for p in files {
                let name = p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                match std::fs::read_to_string(&p)
                    .map_err(|e| e.to_string())
                    .and_then(|t| parse(&name, &t))
                {
                    Ok(w) => wfs.push(w),
                    Err(e) => out.push(format!(
                        "{name}: {e}; a workflow that cannot be read fails closed"
                    )),
                }
            }
        }
    }
    judge(&wfs, &required, &allowed, &mut out);
    out
}

type Jobs<'a> = BTreeMap<&'a str, (&'a Workflow, &'a Job)>;

fn index_jobs<'a>(wfs: &'a [Workflow], out: &mut Vec<String>) -> Jobs<'a> {
    let mut jobs: Jobs = BTreeMap::new();
    for w in wfs {
        for j in &w.jobs {
            if let Some((prev, _)) = jobs.insert(&j.id, (w, j)) {
                out.push(format!(
                    "{}: job id '{}' is also in {}; check names would collide",
                    w.file, j.id, prev.file
                ));
            }
        }
    }
    jobs
}

/// The required jobs and everything they `needs`, transitively.
fn needs_closure<'a>(jobs: &Jobs<'a>, required: &'a BTreeSet<String>) -> BTreeSet<&'a str> {
    let mut closure: BTreeSet<&str> = BTreeSet::new();
    let mut todo: Vec<&str> = required.iter().map(String::as_str).collect();
    while let Some(id) = todo.pop() {
        if let Some((_, j)) = jobs.get(id) {
            if closure.insert(id) {
                todo.extend(j.needs.iter().map(String::as_str));
            }
        }
    }
    closure
}

fn judge_main_job(
    w: &Workflow,
    j: &Job,
    in_closure: bool,
    allowed: &BTreeSet<String>,
    out: &mut Vec<String>,
) {
    let at = format!("{}: job '{}' runs on push to main", w.file, j.id);
    if !w.pr_side {
        out.push(format!(
            "{at} but its workflow has no pull_request trigger that covers main"
        ));
    }
    if !in_closure {
        out.push(format!(
            "{at} but is not in {REQUIRED} or needed by a job in it"
        ));
    }
    match &j.cond {
        Some(Some(c)) if allowed.contains(c) => {}
        Some(_) => out.push(format!(
            "{at} with a job-level 'if' that is not in {ALLOWED_IF}"
        )),
        None => {}
    }
    if j.third_state {
        out.push(format!("{at} with continue-on-error (a third state)"));
    }
}

fn judge_required(jobs: &Jobs, required: &BTreeSet<String>, out: &mut Vec<String>) {
    for r in required {
        match jobs.get(r.as_str()) {
            Some((w, j)) if w.pr_side && j.plain_name => {}
            Some((w, _)) if w.pr_side => out.push(format!(
                "{REQUIRED} lists '{r}' but its check name differs from the job id (name or matrix)"
            )),
            _ => out.push(format!(
                "{REQUIRED} lists '{r}' but no job of that id runs on pull_request to main"
            )),
        }
    }
}

fn judge(
    wfs: &[Workflow],
    required: &BTreeSet<String>,
    allowed: &BTreeSet<String>,
    out: &mut Vec<String>,
) {
    let jobs = index_jobs(wfs, out);
    let closure = needs_closure(&jobs, required);
    for w in wfs.iter().filter(|w| w.main_side) {
        for j in &w.jobs {
            judge_main_job(w, j, closure.contains(j.id.as_str()), allowed, out);
        }
    }
    judge_required(&jobs, required, out);
}
