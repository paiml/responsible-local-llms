//! Parse one workflow file (YAML, aliases and merge keys resolved) into what the rule needs:
//! its trigger classes and every job with its RENDERED check names.

use crate::allow::{Allow, Class};
use regex::Regex;
use serde_yaml_ng::{Mapping, Value};
use std::collections::BTreeMap;
use std::path::Path;

const MAX_DEPTH: usize = 4;
const MAX_MATRIX: usize = 256;

/// Keys an aggregate job and its single step may carry. Anything else is not understood.
const AGG_JOB_KEYS: &[&str] = &["name", "runs-on", "needs", "if", "steps", "timeout-minutes"];
const AGG_STEP_KEYS: &[&str] = &["name", "if", "run"];

fn only_keys(m: &Mapping, allowed: &[&str]) -> bool {
    m.keys()
        .all(|k| k.as_str().is_some_and(|k| allowed.contains(&k)))
}

pub struct Step {
    /// Every key of the step is one the aggregate shape names (allow-list, not deny-list).
    pub plain: bool,
    pub cond: Option<Option<String>>,
    pub run: Option<String>,
}

pub struct Job {
    pub id: String,
    /// Every job key, and the workflow, is one the aggregate shape names: no `shell`, `defaults`,
    /// `env`, `container`, `strategy`, `uses` or anything else that could change what the step does.
    pub plain: bool,
    /// The check names this job reports: matrix expanded, `caller / inner` for a callee.
    pub names: Vec<String>,
    pub needs: Vec<String>,
    pub cond: Option<Option<String>>,
    pub steps: Vec<Step>,
    /// continue-on-error on the job, a step, or any job of a callee.
    pub third_state: bool,
    /// Every job-level and step-level `if` inside callee jobs.
    pub inner_conds: Vec<Option<String>>,
}

impl Job {
    /// Own job-level and step-level `if`s, then the callee's.
    pub fn conds(&self, with_own: bool) -> Vec<Option<String>> {
        let mut out = Vec::new();
        if with_own {
            out.extend(self.cond.clone());
            out.extend(self.steps.iter().filter_map(|s| s.cond.clone()));
        }
        out.extend(self.inner_conds.iter().cloned());
        out
    }
}

pub struct Workflow {
    pub file: String,
    pub main_side: bool,
    pub pr_side: bool,
    /// Has a trigger other than workflow_call, so its jobs report checks of their own.
    pub standalone: bool,
    pub jobs: Vec<Job>,
}

pub struct Cx<'a> {
    pub root: &'a Path,
    pub allow: &'a Allow,
    depth: usize,
    stack: Vec<String>,
}

impl<'a> Cx<'a> {
    pub fn new(root: &'a Path, allow: &'a Allow) -> Self {
        Cx {
            root,
            allow,
            depth: 0,
            stack: Vec::new(),
        }
    }
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
    // Allow-list of what a pattern may contain; GitHub's escape and ref-prefix handling is not
    // measured, so a backslash, a `refs/` prefix or any other character is refused, not read literally.
    if pat.starts_with("refs/")
        || !pat
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./*?+[]".contains(c))
    {
        return Err(format!("branch pattern '{pat}' is not understood"));
    }
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

fn scalar(v: &Value) -> Result<String, String> {
    match v {
        Value::String(s) if !s.contains("${{") => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        _ => Err("a matrix value is not a plain string, number or boolean".to_owned()),
    }
}

/// Every combination of the matrix, as (key, value) pairs in declared key order.
fn combos(strategy: &Value) -> Result<Vec<Vec<(String, String)>>, String> {
    let s = strategy
        .as_mapping()
        .ok_or("'strategy' is not a mapping, so its check names cannot be rendered")?;
    let Some(mx) = s.get("matrix") else {
        return Ok(vec![Vec::new()]);
    };
    let mx = mx
        .as_mapping()
        .filter(|m| !m.is_empty())
        .ok_or("'matrix' is not a non-empty mapping, so its check names cannot be rendered")?;
    let mut out: Vec<Vec<(String, String)>> = vec![Vec::new()];
    for (k, v) in mx {
        let key = k.as_str().ok_or("a matrix key is not a string")?;
        if key == "include" || key == "exclude" {
            return Err(format!("matrix '{key}' cannot be rendered statically"));
        }
        let vals = v
            .as_sequence()
            .filter(|q| !q.is_empty())
            .ok_or_else(|| format!("matrix '{key}' is not a non-empty list"))?;
        let vals = vals.iter().map(scalar).collect::<Result<Vec<_>, _>>()?;
        let mut next = Vec::new();
        for base in &out {
            for val in &vals {
                let mut c = base.clone();
                c.push((key.to_owned(), val.clone()));
                next.push(c);
            }
        }
        if next.len() > MAX_MATRIX {
            return Err("matrix is too large to render".to_owned());
        }
        out = next;
    }
    Ok(out)
}

/// The check names of a job: `name` (or the id), matrix expanded.
fn render_names(id: &str, m: &Mapping) -> Result<Vec<String>, String> {
    let custom = match m.get("name") {
        None => None,
        Some(Value::String(s)) => Some(s.as_str()),
        Some(_) => return Err(format!("job '{id}' has a name that is not a string")),
    };
    let cs = match m.get("strategy") {
        None => vec![Vec::new()],
        Some(s) => combos(s).map_err(|e| format!("job '{id}': {e}"))?,
    };
    let sub = Regex::new(r"\$\{\{\s*matrix\.([A-Za-z0-9_-]+)\s*\}\}").expect("static regex");
    let mut out = Vec::new();
    for c in cs {
        let name = match custom {
            Some(t) => sub
                .replace_all(t, |caps: &regex::Captures| {
                    c.iter()
                        .find(|(k, _)| k == &caps[1])
                        .map_or_else(|| caps[0].to_owned(), |(_, v)| v.clone())
                })
                .into_owned(),
            None if c.is_empty() => id.to_owned(),
            None => {
                let vals: Vec<&str> = c.iter().map(|(_, v)| v.as_str()).collect();
                format!("{id} ({})", vals.join(", "))
            }
        };
        if name.contains("${{") {
            return Err(format!(
                "job '{id}' has a name that cannot be rendered statically"
            ));
        }
        out.push(name);
    }
    Ok(out)
}

struct Inner {
    name: String,
    third: bool,
    conds: Vec<Option<String>>,
}

fn resolve_uses(uses: &str, cx: &Cx) -> Result<Vec<Inner>, String> {
    let Some(rel) = uses.strip_prefix("./") else {
        let names = cx.allow.remotes.get(uses).ok_or_else(|| {
            format!(
                "reusable workflow '{uses}' is remote and not pinned in .github/one-gate-allow.txt"
            )
        })?;
        return Ok(names
            .iter()
            .map(|n| Inner {
                name: n.clone(),
                third: false,
                conds: Vec::new(),
            })
            .collect());
    };
    if !rel.starts_with(".github/workflows/") || rel.contains("..") {
        return Err(format!(
            "reusable workflow '{uses}' is outside .github/workflows"
        ));
    }
    if cx.depth >= MAX_DEPTH || cx.stack.iter().any(|s| s == rel) {
        return Err(format!(
            "reusable workflow '{uses}' nests too deep or loops"
        ));
    }
    let text = std::fs::read_to_string(cx.root.join(rel)).map_err(|e| format!("{uses}: {e}"))?;
    let top = parse_doc(&text)?;
    if !triggers(find_on(&top)?)?.contains_key("workflow_call") {
        return Err(format!(
            "reusable workflow '{uses}' has no workflow_call trigger"
        ));
    }
    let mut inner_cx = Cx {
        root: cx.root,
        allow: cx.allow,
        depth: cx.depth + 1,
        stack: cx.stack.clone(),
    };
    inner_cx.stack.push(rel.to_owned());
    let jobs = parse_jobs(&top, &inner_cx)?;
    Ok(jobs
        .iter()
        .flat_map(|j| {
            j.names.iter().map(|n| Inner {
                name: n.clone(),
                third: j.third_state,
                conds: j.conds(true),
            })
        })
        .collect())
}

fn parse_steps(m: &Mapping, id: &str) -> Result<(Vec<Step>, bool), String> {
    let Some(v) = m.get("steps") else {
        return Ok((Vec::new(), false));
    };
    let q = v
        .as_sequence()
        .ok_or_else(|| format!("job '{id}' has steps that are not a list"))?;
    let mut steps = Vec::new();
    let mut third = false;
    for s in q {
        let sm = s
            .as_mapping()
            .ok_or_else(|| format!("job '{id}' has a step that is not a mapping"))?;
        third |= continue_on_error(sm);
        steps.push(Step {
            plain: only_keys(sm, AGG_STEP_KEYS),
            cond: sm.get("if").map(|c| c.as_str().map(str::to_owned)),
            run: sm.get("run").and_then(Value::as_str).map(str::to_owned),
        });
    }
    Ok((steps, third))
}

fn parse_job(id: &str, v: &Value, cx: &Cx, top_defaults: bool) -> Result<Job, String> {
    let m = v
        .as_mapping()
        .ok_or_else(|| format!("job '{id}' is not a mapping"))?;
    let mut names = render_names(id, m)?;
    let needs = match m.get("needs") {
        None => Vec::new(),
        Some(n) => str_list(n).ok_or_else(|| format!("job '{id}' has unreadable needs"))?,
    };
    let (steps, steps_third) = parse_steps(m, id)?;
    let mut third_state = continue_on_error(m) || steps_third;
    let mut inner_conds = Vec::new();
    if let Some(u) = m.get("uses") {
        let uses = u
            .as_str()
            .ok_or_else(|| format!("job '{id}' has a 'uses' that is not a string"))?;
        let inner = resolve_uses(uses, cx).map_err(|e| format!("job '{id}': {e}"))?;
        names = names
            .iter()
            .flat_map(|b| inner.iter().map(move |i| format!("{b} / {}", i.name)))
            .collect();
        third_state |= inner.iter().any(|i| i.third);
        inner_conds = inner.into_iter().flat_map(|i| i.conds).collect();
    }
    Ok(Job {
        id: id.to_owned(),
        plain: only_keys(m, AGG_JOB_KEYS) && !top_defaults,
        names,
        needs,
        cond: m.get("if").map(|c| c.as_str().map(str::to_owned)),
        steps,
        third_state,
        inner_conds,
    })
}

fn parse_jobs(top: &Mapping, cx: &Cx) -> Result<Vec<Job>, String> {
    let jobs = top
        .get("jobs")
        .and_then(Value::as_mapping)
        .filter(|j| !j.is_empty())
        .ok_or("has no non-empty 'jobs' mapping")?;
    let mut out = Vec::new();
    for (k, v) in jobs {
        let id = k.as_str().ok_or("a job id is not a string")?;
        out.push(parse_job(id, v, cx, top.contains_key("defaults"))?);
    }
    Ok(out)
}

fn parse_doc(text: &str) -> Result<Mapping, String> {
    let mut doc: Value = serde_yaml_ng::from_str(text).map_err(|e| e.to_string())?;
    doc.apply_merge().map_err(|e| e.to_string())?;
    doc.as_mapping()
        .cloned()
        .ok_or_else(|| "top level is not a mapping".to_owned())
}

pub fn parse(cx: &Cx, file: &str, text: &str) -> Result<Workflow, String> {
    let top = parse_doc(text)?;
    let trig = triggers(find_on(&top)?)?;
    let (mut main_side, mut pr_side, mut standalone) = (false, false, false);
    for (event, cfg) in &trig {
        if event == "pull_request_target" {
            return Err("pull_request_target is never allowed".to_owned());
        }
        match cx.allow.class_of(event, file)? {
            Class::Main if event == "push" => main_side |= covers_main(cfg)?,
            Class::Main => main_side = true,
            Class::Pr => pr_side = pr_counts(cfg)?,
            Class::Neutral => {}
        }
        standalone |= event != "workflow_call";
    }
    Ok(Workflow {
        file: file.to_owned(),
        main_side,
        pr_side,
        standalone,
        jobs: parse_jobs(&top, cx)?,
    })
}
