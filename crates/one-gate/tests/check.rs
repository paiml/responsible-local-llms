//! Every evasion in the two sign-off rounds of paiml/responsible-local-llms#54 is a failing
//! fixture here, beside the positive tests on the synthetic good tree and on this repository.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static N: AtomicUsize = AtomicUsize::new(0);

const GOOD: &str = "name: CI
on:
  pull_request:
    branches: [main]
  push:
    branches: [main]
  workflow_dispatch:
jobs:
  gate:
    name: gate
    runs-on: [self-hosted, clean-room, X64, ffmpeg]
    steps:
      - run: true
";

fn tree(files: &[(&str, &str)]) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "one-gate-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&root);
    for (rel, body) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }
    root
}

fn wf(name: &str) -> String {
    format!(".github/workflows/{name}")
}

/// Violations for a tree of ci.yml plus the given extra workflow, required list `gate`.
fn with_extra(extra: &str) -> Vec<String> {
    with_extra_req(extra, "gate\n")
}

fn with_extra_req(extra: &str, required: &str) -> Vec<String> {
    let t = tree(&[
        (&wf("ci.yml"), GOOD),
        (&wf("extra.yml"), extra),
        (".github/required-status-checks.txt", required),
    ]);
    one_gate::check(&t)
}

/// Violations for a tree of one ci.yml, required list `gate`.
fn only(ci: &str) -> Vec<String> {
    let t = tree(&[
        (&wf("ci.yml"), ci),
        (".github/required-status-checks.txt", "gate\n"),
    ]);
    one_gate::check(&t)
}

fn has(v: &[String], needle: &str) {
    assert!(
        v.iter().any(|l| l.contains(needle)),
        "no '{needle}' in {v:?}"
    );
}

fn main_job(on: &str) -> String {
    format!("name: x\non: {on}\njobs:\n  sneaky:\n    runs-on: x\n    steps:\n      - run: true\n")
}

const NOT_REQ: &str = "is not in .github/required-status-checks.txt";

#[test]
fn good_tree_passes() {
    assert_eq!(only(GOOD), Vec::<String>::new());
}

#[test]
fn this_repository_passes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert_eq!(one_gate::check(&root), Vec::<String>::new());
}

#[test]
fn single_quoted_on_key() {
    has(
        &with_extra("'on':\n  push:\njobs:\n  sneaky:\n    runs-on: x\n"),
        NOT_REQ,
    );
}

#[test]
fn double_quoted_on_scalar() {
    has(
        &with_extra(&main_job("push").replace("on: push", "\"on\": push")),
        NOT_REQ,
    );
}

#[test]
fn yaml_11_boolean_true_key_is_on() {
    has(
        &with_extra(&main_job("{push: }").replace("on:", "true:")),
        NOT_REQ,
    );
}

#[test]
fn flow_mapping_triggers() {
    has(
        &with_extra(&main_job("{push: {branches: [main]}}")),
        NOT_REQ,
    );
}

#[test]
fn alias_triggers() {
    let y = "t: &t\n  push:\n    branches: [main]\non: *t\njobs:\n  sneaky:\n    runs-on: x\n";
    has(&with_extra(y), NOT_REQ);
}

#[test]
fn four_space_indentation() {
    let y = "on:\n    push:\n        branches: [main]\njobs:\n    sneaky:\n        runs-on: x\n";
    has(&with_extra(y), NOT_REQ);
}

#[test]
fn quoted_job_key() {
    let y = "on: push\njobs:\n  \"sneaky\":\n    runs-on: x\n";
    has(&with_extra(y), NOT_REQ);
}

#[test]
fn inline_job_mapping() {
    let y = "on: push\njobs:\n  sneaky: {runs-on: x, steps: [{run: true}]}\n";
    has(&with_extra(y), NOT_REQ);
}

#[test]
fn crlf_line_endings() {
    let y = main_job("push").replace('\n', "\r\n");
    has(&with_extra(&y), NOT_REQ);
}

#[test]
fn job_level_if_on_head_ref() {
    let y = "on:\n  push:\n  pull_request:\njobs:\n  second:\n    if: github.head_ref == ''\n    runs-on: x\n";
    has(&with_extra_req(y, "gate\nsecond\n"), "job-level 'if'");
}

#[test]
fn block_scalar_if() {
    let y = "on:\n  push:\n  pull_request:\njobs:\n  second:\n    if: >-\n      github.event_name\n      == 'push'\n    runs-on: x\n";
    has(&with_extra_req(y, "gate\nsecond\n"), "job-level 'if'");
}

#[test]
fn non_string_if() {
    let y = "on:\n  push:\n  pull_request:\njobs:\n  second:\n    if: true\n    runs-on: x\n";
    has(&with_extra_req(y, "gate\nsecond\n"), "job-level 'if'");
}

#[test]
fn if_allowed_only_when_byte_equal() {
    let y = "on:\n  push:\n  pull_request:\njobs:\n  second:\n    if: always()\n    runs-on: x\n";
    let ok = |allow: &str| {
        let t = tree(&[
            (&wf("ci.yml"), GOOD),
            (&wf("extra.yml"), y),
            (".github/required-status-checks.txt", "gate\nsecond\n"),
            (".github/one-gate-allowed-if.txt", allow),
        ]);
        one_gate::check(&t)
    };
    assert_eq!(ok("# why\n\nalways()\n"), Vec::<String>::new());
    has(&ok("always() \n"), "job-level 'if'");
    has(&ok("# only comments\n"), "job-level 'if'");
}

#[test]
fn allow_list_that_cannot_be_read() {
    let t = tree(&[
        (&wf("ci.yml"), GOOD),
        (".github/required-status-checks.txt", "gate\n"),
        (".github/one-gate-allowed-if.txt/keep", ""),
    ]);
    has(&one_gate::check(&t), "one-gate-allowed-if.txt");
}

#[test]
fn pull_request_branches_ignore_main_is_no_pr_check() {
    let y = "on:\n  push:\n    branches: [main]\n  pull_request:\n    branches-ignore: [main]\njobs:\n  gate:\n    runs-on: x\n";
    let v = with_extra(y);
    has(&v, "no pull_request trigger that covers main");
    has(&v, "also in ci.yml");
}

#[test]
fn main_push_only_workflow_with_required_job() {
    let y = "on:\n  push:\n    branches: [main]\njobs:\n  second:\n    runs-on: x\n";
    let v = with_extra_req(y, "gate\nsecond\n");
    has(&v, "no pull_request trigger that covers main");
    has(&v, "no job of that id runs on pull_request");
}

#[test]
fn workflow_run_and_merge_group_without_pull_request() {
    has(&with_extra(&main_job("workflow_run")), NOT_REQ);
    has(&with_extra(&main_job("[merge_group]")), NOT_REQ);
    has(&with_extra(&main_job("push")), "no pull_request trigger");
}

#[test]
fn unreadable_workflows_fail_closed() {
    for y in [
        "on: [",
        "name: x\njobs: {}\n",
        "- a\n- b\n",
        "on:\njobs: {}\n",
        "on: {}\njobs: {}\n",
        "on: [1]\njobs: {}\n",
        "on: {1: x}\njobs: {}\n",
        "on: push\n",
        "on: push\njobs: [a]\n",
        "on: push\njobs:\n  1:\n    runs-on: x\n",
        "on: push\njobs:\n  a: 5\n",
        "on: push\njobs:\n  a:\n    name: [x]\n",
        "on: push\njobs:\n  a:\n    needs: 5\n",
        "on: push\non: pull_request\njobs: {}\n",
        "on: push\ntrue: pull_request\njobs: {}\n",
        "on:\n  push:\n  push:\njobs: {}\n",
        "on: push\njobs:\n  a:\n    <<: 5\n",
        "on:\n  push: 5\njobs: {}\n",
    ] {
        has(&with_extra(y), "fails closed");
    }
}

fn push_cfg(cfg: &str) -> Vec<String> {
    with_extra(&format!(
        "on:\n  push:\n{cfg}\njobs:\n  sneaky:\n    runs-on: x\n"
    ))
}

#[test]
fn push_branch_filters() {
    for hit in [
        "    branches: [main]",
        "    branches: main",
        "    branches: ['**']",
        "    branches: [m*]",
        "    branches: [mai?n]",
        "    branches: [ma+in]",
        "    branches: ['[mn]ain']",
        "    branches: ['**', '!release']",
        "    branches: [release, main]",
        "    branches-ignore: [release]",
        "    paths: [src]",
    ] {
        has(&push_cfg(hit), NOT_REQ);
    }
    for miss in [
        "    branches: [release/*]",
        "    branches: ['**', '!main']",
        "    branches: ['!main']",
        "    branches-ignore: [main]",
        "    branches-ignore: ['**']",
    ] {
        assert_eq!(push_cfg(miss), Vec::<String>::new(), "{miss}");
    }
    for bad in [
        "    branches: [main]\n    branches-ignore: [x]",
        "    branches: []",
        "    branches: 5",
        "    branches: ['[']",
    ] {
        has(&push_cfg(bad), "fails closed");
    }
}

fn pr_cfg(cfg: &str) -> Vec<String> {
    only(&GOOD.replace(
        "  pull_request:\n    branches: [main]\n",
        &format!("  pull_request:\n{cfg}\n"),
    ))
}

#[test]
fn pull_request_filters() {
    for ok in [
        "    branches: [main]",
        "    types: [opened, synchronize, reopened, labeled]",
        "    branches-ignore: [release]",
    ] {
        assert_eq!(pr_cfg(ok), Vec::<String>::new(), "{ok}");
    }
    for narrow in [
        "    branches: [release]",
        "    branches-ignore: [main]",
        "    paths: [src/**]",
        "    paths-ignore: [docs/**]",
        "    types: [opened]",
    ] {
        has(&pr_cfg(narrow), "no pull_request trigger that covers main");
    }
    assert_eq!(
        only(&GOOD.replace(
            "  pull_request:\n    branches: [main]\n",
            "  pull_request:\n"
        )),
        Vec::<String>::new()
    );
}

#[test]
fn required_job_check_name_must_equal_its_id() {
    has(
        &only(&GOOD.replace("name: gate", "name: lint")),
        "check name differs",
    );
    let m = GOOD.replace(
        "    steps:",
        "    strategy:\n      matrix:\n        a: [1, 2]\n    steps:",
    );
    has(&only(&m), "check name differs");
}

#[test]
fn needs_closure_makes_a_main_job_required() {
    let ci = format!(
        "{GOOD}  build:\n    runs-on: x\n  lint:\n    needs: [build, gate]\n    runs-on: x\n  prep:\n    runs-on: x\n"
    )
    .replace("    name: gate\n", "    name: gate\n    needs: lint\n");
    let v = only(&ci);
    has(&v, "job 'prep' runs on push to main but is not in");
    assert_eq!(v.len(), 1, "{v:?}");
}

#[test]
fn continue_on_error_is_a_third_state() {
    has(
        &only(&GOOD.replace("    steps:", "    continue-on-error: true\n    steps:")),
        "continue-on-error",
    );
    has(
        &only(&GOOD.replace(
            "- run: true",
            "- run: true\n        continue-on-error: ${{ true }}",
        )),
        "continue-on-error",
    );
    let off = GOOD.replace("    steps:", "    continue-on-error: false\n    steps:");
    assert_eq!(only(&off), Vec::<String>::new());
}

#[test]
fn merge_keys_are_resolved() {
    let ci = "on:\n  push:\n  pull_request:\nbase: &b\n  continue-on-error: true\njobs:\n  gate:\n    <<: *b\n    runs-on: x\n";
    has(&only(ci), "continue-on-error");
}

#[test]
fn missing_or_empty_inputs() {
    let t = tree(&[(&wf("ci.yml"), GOOD)]);
    let v = one_gate::check(&t);
    has(&v, "required-status-checks.txt");
    has(&v, "lists no checks");
    let t = tree(&[
        (&wf("ci.yml"), GOOD),
        (".github/required-status-checks.txt", "# none\n\n"),
    ]);
    has(&one_gate::check(&t), "lists no checks");
    let t = tree(&[(".github/required-status-checks.txt", "gate\n")]);
    has(&one_gate::check(&t), "workflows");
    let t = tree(&[
        (".github/workflows/README.md", "x"),
        (".github/required-status-checks.txt", "gate\n"),
    ]);
    has(&one_gate::check(&t), "no workflow files");
    let t = tree(&[
        (".github/workflows/d.yml/keep", ""),
        (".github/required-status-checks.txt", "gate\n"),
    ]);
    has(&one_gate::check(&t), "d.yml");
    let t = tree(&[
        (&wf("ci.yaml"), GOOD),
        (".github/required-status-checks.txt", "gate\nghost\n"),
    ]);
    has(&one_gate::check(&t), "'ghost'");
}
