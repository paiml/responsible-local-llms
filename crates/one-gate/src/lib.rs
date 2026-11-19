//! ONE GATE (operator ruling 2026-11-19): one check set per repo. Every job that runs on push
//! to main must also be a required check on pull requests.
//!
//! The decision is a fail-closed ALLOW-LIST, never a list of bad shapes. Each workflow under
//! `.github/workflows` is PARSED as YAML (aliases and merge keys resolved). Anything the check
//! does not positively understand is a violation:
//!
//! 1. Triggers. Every event of every workflow must be classed in `.github/one-gate-allow.txt`
//!    (main, pr or neutral, by file where listed). An unknown event, `pull_request_target` or an
//!    unparsable `on` is RED.
//! 2. Names. Checks are compared by RENDERED CHECK NAME (`name` or id, matrix expanded,
//!    `caller / inner` for a reusable workflow), never by job id. A name or matrix that cannot be
//!    rendered statically is RED, and a name produced twice is RED. Every main-side check name
//!    must be in `.github/required-status-checks.txt` or be a direct `needs` of a required
//!    aggregate job; every required name must be produced by a pull_request job.
//! 3. needs. A required job's `needs` must each be required, or the job must be the aggregate
//!    shape: `if: always()` and exactly one step, `if: needs.X.result != 'success' || ...` over
//!    every need, running `exit 1`. Local callees are parsed; a remote one is RED unless pinned
//!    by exact ref in the allow-list.
//! 4. `continue-on-error` (job, step or callee job) on a main-side or required job is RED; so is
//!    an `if` (job, step or callee job) unless byte-equal to an `if` line of the allow-list.

mod allow;
mod canon;
mod workflow;

use allow::Allow;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use workflow::{Cx, Job, Workflow};

const REQUIRED: &str = ".github/required-status-checks.txt";
const ALLOW: &str = ".github/one-gate-allow.txt";
const WORKFLOWS: &str = ".github/workflows";

fn read_lines(path: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect())
}

/// Every violation of the ONE GATE rule in the repository at `root`; empty means PASS.
pub fn check(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let required: BTreeSet<String> = read_lines(&root.join(REQUIRED))
        .unwrap_or_else(|e| {
            out.push(e);
            Vec::new()
        })
        .into_iter()
        .collect();
    if required.is_empty() {
        out.push(format!(
            "{REQUIRED} lists no checks; an empty list is not a gate"
        ));
    }
    let allow = match std::fs::read_to_string(root.join(ALLOW))
        .map_err(|e| vec![format!("{ALLOW}: {e}")])
        .and_then(|t| Allow::parse(&t))
    {
        Ok(a) => a,
        Err(es) => {
            out.extend(es);
            return out;
        }
    };
    let cx = Cx::new(root, &allow);
    let mut wfs = Vec::new();
    let dir = root.join(WORKFLOWS);
    let (names, bad) = canon::list_dir(&dir);
    out.extend(bad);
    if names.is_empty() {
        out.push(format!("{}: no workflow files", dir.display()));
    }
    for name in names {
        match std::fs::read_to_string(dir.join(&name))
            .map_err(|e| e.to_string())
            .and_then(|t| workflow::parse(&cx, &name, &t))
        {
            Ok(w) => wfs.push(w),
            Err(e) => out.push(format!(
                "{name}: {e}; a workflow that cannot be read fails closed"
            )),
        }
    }
    judge(&wfs, &required, &allow, &mut out);
    out
}

/// The committed aggregate shape: `if: always()` and one step that fails when any need did not
/// succeed (skipped and cancelled count, because GitHub reports a skipped job as Success).
fn aggregate_ok(j: &Job) -> bool {
    let always = |c: &str| matches!(c.trim(), "always()" | "${{ always() }}");
    if !j
        .cond
        .as_ref()
        .and_then(Option::as_deref)
        .is_some_and(always)
        || j.needs.is_empty()
    {
        return false;
    }
    let [step] = &j.steps[..] else {
        return false;
    };
    if !j.plain || !step.plain {
        return false;
    }
    let Some(Some(cond)) = &step.cond else {
        return false;
    };
    let cond = cond.trim();
    let cond = cond
        .strip_prefix("${{")
        .and_then(|c| c.strip_suffix("}}"))
        .unwrap_or(cond);
    let terms: BTreeSet<String> = cond.split("||").map(|t| t.trim().to_owned()).collect();
    let want: BTreeSet<String> = j
        .needs
        .iter()
        .map(|n| format!("needs.{n}.result != 'success'"))
        .collect();
    step.run.as_deref().map(str::trim) == Some("exit 1")
        && terms == want
        && cond.split("||").count() == j.needs.len()
}

type At = (usize, usize);
/// Check names compared as GitHub might: trimmed, case-folded, inner whitespace collapsed. How GitHub
/// really matches is not measured, so any two names that fold together collide.
fn fold(n: &str) -> String {
    n.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

type Producers<'a> = BTreeMap<String, (At, &'a str)>;

fn index_producers<'a>(wfs: &'a [Workflow], out: &mut Vec<String>) -> Producers<'a> {
    let mut producers: Producers = BTreeMap::new();
    for (wi, w) in wfs.iter().enumerate().filter(|(_, w)| w.standalone) {
        for (ji, j) in w.jobs.iter().enumerate() {
            for n in &j.names {
                if producers.insert(fold(n), ((wi, ji), n)).is_some() {
                    out.push(format!(
                        "{}: check name '{n}' is produced more than once; two checks would share one context",
                        w.file
                    ));
                }
            }
        }
    }
    producers
}

fn all_required(j: &Job, required: &BTreeSet<String>) -> bool {
    j.names.iter().all(|n| required.contains(n))
}

/// The jobs that produce a required name on pull requests.
fn required_jobs(
    wfs: &[Workflow],
    producers: &Producers,
    required: &BTreeSet<String>,
    out: &mut Vec<String>,
) -> BTreeSet<At> {
    let mut req_jobs = BTreeSet::new();
    for r in required {
        match producers.get(&fold(r)) {
            Some(&((wi, ji), n)) if wfs[wi].pr_side && n == r => {
                req_jobs.insert((wi, ji));
            }
            _ => out.push(format!(
                "{REQUIRED} lists '{r}' but no job renders that check name on pull_request to main"
            )),
        }
    }
    req_jobs
}

/// Judge each required job's `needs`. Returns (aggregates, jobs guarded by an aggregate).
fn judge_needs(
    wfs: &[Workflow],
    req_jobs: &BTreeSet<At>,
    required: &BTreeSet<String>,
    out: &mut Vec<String>,
) -> (BTreeSet<At>, BTreeSet<At>) {
    let (mut aggregates, mut guarded) = (BTreeSet::new(), BTreeSet::new());
    for &(wi, ji) in req_jobs {
        let (w, j) = (&wfs[wi], &wfs[wi].jobs[ji]);
        let at = format!("{}: required job '{}'", w.file, j.id);
        let aggregate = aggregate_ok(j) && all_required(j, required);
        if aggregate {
            aggregates.insert((wi, ji));
        }
        for n in &j.needs {
            match w.jobs.iter().position(|x| &x.id == n) {
                None => out.push(format!("{at} needs '{n}', which is not a job of the workflow")),
                Some(ni) if aggregate => {
                    guarded.insert((wi, ni));
                }
                Some(ni) if !all_required(&w.jobs[ni], required) => out.push(format!(
                    "{at} needs '{n}', which is not itself required, and is not the aggregate shape (if: always() plus a failure test of every needs result)"
                )),
                Some(_) => {}
            }
        }
    }
    (aggregates, guarded)
}

struct Facts<'a> {
    required: &'a BTreeSet<String>,
    allow: &'a Allow,
    aggregates: BTreeSet<At>,
    guarded: BTreeSet<At>,
}

fn judge_job(w: &Workflow, j: &Job, at_idx: At, f: &Facts, out: &mut Vec<String>) {
    let at = format!("{}: job '{}'", w.file, j.id);
    if w.main_side {
        if !w.pr_side {
            out.push(format!(
                "{at} runs on push to main but its workflow has no pull_request trigger that covers main"
            ));
        }
        if !f.guarded.contains(&at_idx) {
            for n in j.names.iter().filter(|n| !f.required.contains(*n)) {
                out.push(format!(
                    "{at} runs on push to main with check name '{n}', which is not in {REQUIRED} and not needed by a required aggregate"
                ));
            }
        }
    }
    if j.third_state {
        out.push(format!("{at} has continue-on-error (a third state)"));
    }
    for c in j.conds(!f.aggregates.contains(&at_idx)) {
        if !c.is_some_and(|c| f.allow.ifs.contains(&c)) {
            out.push(format!(
                "{at} has an 'if' that is not byte-equal to an if entry of {ALLOW}"
            ));
        }
    }
}

fn judge(wfs: &[Workflow], required: &BTreeSet<String>, allow: &Allow, out: &mut Vec<String>) {
    let producers = index_producers(wfs, out);
    let req_jobs = required_jobs(wfs, &producers, required, out);
    let (aggregates, guarded) = judge_needs(wfs, &req_jobs, required, out);
    let mut judged = req_jobs;
    for (wi, w) in wfs.iter().enumerate().filter(|(_, w)| w.main_side) {
        judged.extend((0..w.jobs.len()).map(|ji| (wi, ji)));
    }
    judged.extend(guarded.iter().copied());
    let facts = Facts {
        required,
        allow,
        aggregates,
        guarded,
    };
    for (wi, ji) in judged {
        judge_job(&wfs[wi], &wfs[wi].jobs[ji], (wi, ji), &facts, out);
    }
}
