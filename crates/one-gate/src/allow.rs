//! The committed allow-list, `.github/one-gate-allow.txt`. Anything the check does not
//! positively find here is RED. THIS FILE IS PART OF THE GATE: an edit to it is a GATE CHANGE and
//! needs a sign-off round from a session that is not the author, bound to the final commit.
//! One entry per line; `#` starts a comment line.
//!
//! * `trigger <event> <main|pr|neutral> [file ...]`: classes an event. With files, the entry
//!   applies to those workflow files only. `push`, `workflow_run` and `merge_group` are always
//!   `main`; `pull_request` is always `pr`; `pull_request_target` is never allowed.
//! * `remote <owner/repo/path@ref> <inner check name> ...`: a remote reusable workflow, by exact
//!   ref, with the check names it renders after `caller / `.
//! * `value <file> <scope> <key> <rendered>`: an `env`, `defaults` or `run-name` accepted when its
//!   whole rendered value (`canon::render`) is byte-equal. Scope: `workflow`, `job:<id>`,
//!   `step:<id>:<index>`.
//! * `filter <file> <event> <key> <yes|no> <entry>`: one `branches`, `branches-ignore`, `tags`,
//!   `paths`, `paths-ignore` or `types` entry, byte-equal, with its class stated by hand: yes means
//!   it lets the workflow run on main (push) or on a PR to main (pull_request). No pattern is ever matched.
//! * `if <text>`: a job-level or step-level `if` accepted byte for byte on a main-side or
//!   required job.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    Main,
    Pr,
    Neutral,
}

pub struct Allow {
    triggers: Vec<(String, Class, Vec<String>)>,
    pub remotes: BTreeMap<String, Vec<String>>,
    pub ifs: BTreeSet<String>,
    values: BTreeSet<(String, String, String, String)>,
    filters: Vec<(String, String, String, bool, String)>,
}

fn parse_class(s: &str) -> Option<Class> {
    match s {
        "main" => Some(Class::Main),
        "pr" => Some(Class::Pr),
        "neutral" => Some(Class::Neutral),
        _ => None,
    }
}

fn fixed_class(event: &str) -> Option<Class> {
    match event {
        "push" | "workflow_run" | "merge_group" => Some(Class::Main),
        "pull_request" => Some(Class::Pr),
        "workflow_call" => Some(Class::Neutral),
        _ => None,
    }
}

fn trigger_line(words: &[&str]) -> Result<(String, Class, Vec<String>), String> {
    let [_, event, class, files @ ..] = words else {
        return Err("trigger needs an event and a class".to_owned());
    };
    let class = parse_class(class).ok_or_else(|| format!("unknown class '{class}'"))?;
    if *event == "pull_request_target" {
        return Err("pull_request_target is never allowed".to_owned());
    }
    match fixed_class(event) {
        Some(fixed) if fixed != class => {
            return Err(format!("'{event}' is always {fixed:?}, not {class:?}"));
        }
        None if class == Class::Pr => {
            return Err(format!("class pr is only for pull_request, not '{event}'"));
        }
        _ => {}
    }
    let files = files.iter().map(|f| (*f).to_owned()).collect();
    Ok(((*event).to_owned(), class, files))
}

fn remote_line(words: &[&str]) -> Result<(String, Vec<String>), String> {
    match words {
        [_, uses, names @ ..] if uses.contains('@') && !names.is_empty() => Ok((
            (*uses).to_owned(),
            names.iter().map(|s| (*s).to_owned()).collect(),
        )),
        _ => Err("remote needs <owner/repo/path@ref> and inner names".to_owned()),
    }
}

fn value_line(line: &str) -> Result<(String, String, String, String), String> {
    let w: Vec<&str> = line.splitn(5, ' ').collect();
    match w[..] {
        ["value", f, s, k, v] if !v.is_empty() && crate::canon::VALUE_KEYS.contains(&k) => {
            Ok((f.to_owned(), s.to_owned(), k.to_owned(), v.to_owned()))
        }
        _ => Err("value needs <file> <scope> <env|defaults|run-name> <rendered>".to_owned()),
    }
}

fn filter_line(line: &str) -> Result<(String, String, String, bool, String), String> {
    let w: Vec<&str> = line.splitn(6, ' ').collect();
    match w[..] {
        ["filter", f, ev, k, c @ ("yes" | "no"), e]
            if !e.is_empty() && crate::canon::EVENT_KEYS.contains(&k) =>
        {
            Ok((
                f.to_owned(),
                ev.to_owned(),
                k.to_owned(),
                c == "yes",
                e.to_owned(),
            ))
        }
        _ => Err("filter needs <file> <event> <key> <yes|no> <entry>".to_owned()),
    }
}

impl Allow {
    /// Is this rendered `env`, `defaults` or `run-name` byte-equal to an entry for the scope?
    pub fn value_ok(&self, file: &str, scope: &str, key: &str, rendered: &str) -> bool {
        self.values.contains(&(
            file.to_owned(),
            scope.to_owned(),
            key.to_owned(),
            rendered.to_owned(),
        ))
    }

    /// The hand-stated class of a filter entry, byte-equal, or None.
    pub fn filter_class(&self, file: &str, event: &str, key: &str, entry: &str) -> Option<bool> {
        let hits: Vec<bool> = self
            .filters
            .iter()
            .filter(|(f, ev, k, _, e)| f == file && ev == event && k == key && e == entry)
            .map(|x| x.3)
            .collect();
        match hits.first() {
            Some(c) if hits.iter().all(|x| x == c) => Some(*c),
            _ => None,
        }
    }

    fn entry(&mut self, line: &str) -> Result<(), String> {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.first().copied() {
            Some("trigger") => trigger_line(&words).map(|t| self.triggers.push(t)),
            Some("value") => value_line(line).map(|v| {
                self.values.insert(v);
            }),
            Some("filter") => filter_line(line).map(|f| self.filters.push(f)),
            Some("remote") => remote_line(&words).map(|(k, v)| {
                self.remotes.insert(k, v);
            }),
            Some("if") if line.starts_with("if ") && line.len() > 3 => {
                self.ifs.insert(line[3..].to_owned());
                Ok(())
            }
            _ => Err("not a trigger, remote, value, filter or if entry".to_owned()),
        }
    }

    /// Parse the file; every bad line is reported, so an unreadable list fails closed.
    pub fn parse(text: &str) -> Result<Allow, Vec<String>> {
        let mut a = Allow {
            triggers: Vec::new(),
            remotes: BTreeMap::new(),
            ifs: BTreeSet::new(),
            values: BTreeSet::new(),
            filters: Vec::new(),
        };
        let mut errs = Vec::new();
        for (n, line) in text.lines().enumerate() {
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            if let Err(e) = a.entry(line) {
                errs.push(format!(
                    ".github/one-gate-allow.txt line {}: {e}; an allow-list that cannot be read fails closed",
                    n + 1
                ));
            }
        }
        if errs.is_empty() {
            Ok(a)
        } else {
            Err(errs)
        }
    }

    /// The class of `event` in workflow `file`, or why it is not understood.
    pub fn class_of(&self, event: &str, file: &str) -> Result<Class, String> {
        let classes: Vec<Class> = self
            .triggers
            .iter()
            .filter(|(e, _, f)| e == event && (f.is_empty() || f.iter().any(|x| x == file)))
            .map(|(_, c, _)| *c)
            .collect();
        match classes.first() {
            Some(c) if classes.iter().all(|x| x == c) => Ok(*c),
            Some(_) => Err(format!(
                "trigger '{event}' has conflicting classes in .github/one-gate-allow.txt"
            )),
            None => Err(format!(
                "trigger '{event}' is not in .github/one-gate-allow.txt for this file; an unknown trigger fails closed"
            )),
        }
    }
}
