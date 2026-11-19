//! Parse one workflow file (YAML, aliases and merge keys resolved) into what the rule needs:
//! its trigger classes and every job with its RENDERED check names.

use crate::allow::{Allow, Class};
use crate::canon;
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

fn refuse(v: Vec<String>) -> Result<(), String> {
    if v.is_empty() {
        Ok(())
    } else {
        Err(v.join("; "))
    }
}

fn str_list(v: &Value) -> Option<Vec<String>> {
    match v {
        Value::String(s) => Some(vec![s.clone()]),
        Value::Sequence(q) => q.iter().map(|x| x.as_str().map(str::to_owned)).collect(),
        _ => None,
    }
}

use canon::Filters;

fn class_of<'a>(f: &'a Filters, key: &str) -> Option<&'a Vec<(String, bool)>> {
    f.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// Does this `push` config run on pushes to main? Entries are never matched: each was stated
/// by hand in the allow-list (the literal `main` under `branches` excepted).
fn push_main(file: &str, cfg: &Value, allow: &Allow) -> Result<bool, String> {
    let f = canon::filters(file, "push", cfg, allow)?;
    let (branches, ignore) = (class_of(&f, "branches"), class_of(&f, "branches-ignore"));
    if class_of(&f, "types").is_some() {
        return Err("'push' has 'types'".to_owned());
    }
    Ok(match (branches, ignore) {
        (Some(_), Some(_)) => {
            return Err("both 'branches' and 'branches-ignore' are set".to_owned())
        }
        (Some(b), None) => b.iter().any(|(_, c)| *c),
        (None, Some(i)) => i.iter().all(|(_, c)| *c),
        (None, None) => match (class_of(&f, "tags"), class_of(&f, "tags-ignore")) {
            (Some(t), _) => t.iter().any(|(_, c)| *c),
            (None, Some(t)) => t.iter().all(|(_, c)| *c),
            (None, None) => true,
        },
    })
}

/// A pull_request trigger counts only when it provably runs on every PR to main: each filter
/// entry byte-equal to an allow-list entry whose hand-stated class is yes, and `types` covering
/// the default three.
fn pr_counts(file: &str, cfg: &Value, allow: &Allow) -> Result<bool, String> {
    let f = canon::filters(file, "pull_request", cfg, allow)?;
    if class_of(&f, "tags").is_some() || class_of(&f, "tags-ignore").is_some() {
        return Err("'pull_request' has 'tags'".to_owned());
    }
    if class_of(&f, "branches").is_some() && class_of(&f, "branches-ignore").is_some() {
        return Err("both 'branches' and 'branches-ignore' are set".to_owned());
    }
    let mut counts = true;
    for (key, entries) in &f {
        counts &= match key.as_str() {
            "branches" => entries.iter().any(|(_, c)| *c),
            "types" => ["opened", "synchronize", "reopened"]
                .iter()
                .all(|d| entries.iter().any(|(e, _)| e == d)),
            _ => entries.iter().all(|(_, c)| *c),
        };
    }
    Ok(counts)
}

fn find_on(top: &Mapping) -> Result<&Value, String> {
    top.get("on").ok_or_else(|| "has no 'on' key".to_owned())
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
    let file = rel.trim_start_matches(".github/workflows/");
    if !canon::valid_name(file) {
        return Err(format!(
            "reusable workflow '{uses}' is not a canonical workflow file name"
        ));
    }
    let text = std::fs::read_to_string(cx.root.join(rel)).map_err(|e| format!("{uses}: {e}"))?;
    let top = parse_doc(&text)?;
    refuse(canon::top_keys(file, &top, cx.allow))?;
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
    let jobs = parse_jobs(&top, &inner_cx, file, true)?;
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

fn parse_job(id: &str, v: &Value, cx: &Cx, ctx: (&str, bool, bool)) -> Result<Job, String> {
    let (file, strict, top_extra) = ctx;
    let m = v
        .as_mapping()
        .ok_or_else(|| format!("job '{id}' is not a mapping"))?;
    if strict {
        refuse(canon::job_keys(file, id, m, cx.allow))?;
    }
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
        plain: only_keys(m, AGG_JOB_KEYS) && !top_extra,
        names,
        needs,
        cond: m.get("if").map(|c| c.as_str().map(str::to_owned)),
        steps,
        third_state,
        inner_conds,
    })
}

fn parse_jobs(top: &Mapping, cx: &Cx, file: &str, strict: bool) -> Result<Vec<Job>, String> {
    let jobs = top
        .get("jobs")
        .and_then(Value::as_mapping)
        .filter(|j| !j.is_empty())
        .ok_or("has no non-empty 'jobs' mapping")?;
    let mut out = Vec::new();
    for (k, v) in jobs {
        let id = k.as_str().ok_or("a job id is not a string")?;
        let extra = ["env", "defaults"].iter().any(|k| top.contains_key(*k));
        out.push(parse_job(id, v, cx, (file, strict, extra))?);
    }
    Ok(out)
}

fn parse_doc(text: &str) -> Result<Mapping, String> {
    let doc: Value = serde_yaml_ng::from_str(text).map_err(|e| e.to_string())?;
    canon::plain_yaml(&doc)?;
    doc.as_mapping()
        .cloned()
        .ok_or_else(|| "top level is not a mapping".to_owned())
}

pub fn parse(cx: &Cx, file: &str, text: &str) -> Result<Workflow, String> {
    let top = parse_doc(text)?;
    refuse(canon::top_keys(file, &top, cx.allow))?;
    let trig = triggers(find_on(&top)?)?;
    let (mut main_side, mut pr_side, mut standalone) = (false, false, false);
    for (event, cfg) in &trig {
        if event == "pull_request_target" {
            return Err("pull_request_target is never allowed".to_owned());
        }
        match cx.allow.class_of(event, file)? {
            Class::Main if event == "push" => main_side |= push_main(file, cfg, cx.allow)?,
            Class::Main => main_side = true,
            Class::Pr => pr_side = pr_counts(file, cfg, cx.allow)?,
            Class::Neutral => {}
        }
        standalone |= event != "workflow_call";
    }
    Ok(Workflow {
        file: file.to_owned(),
        main_side,
        pr_side,
        standalone,
        jobs: parse_jobs(&top, cx, file, main_side || pr_side)?,
    })
}
