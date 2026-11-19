//! Every evasion in the sign-off rounds of paiml/responsible-local-llms#54 is a failing fixture
//! here, beside the positive tests on the synthetic good tree and on this repository. The check
//! is an allow-list: each test plants something the check must NOT positively understand.

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

const ALLOW: &str = "trigger push main
trigger workflow_run main
trigger merge_group main
trigger pull_request pr
trigger workflow_dispatch neutral
trigger workflow_call neutral
trigger schedule neutral live.yml
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

/// Violations for the given workflow files, required list and allow-list.
fn run(files: &[(&str, &str)], required: &str, allow: &str) -> Vec<String> {
    let mut all: Vec<(String, &str)> = files.iter().map(|(n, b)| (wf(n), *b)).collect();
    all.push((".github/required-status-checks.txt".to_owned(), required));
    all.push((".github/one-gate-allow.txt".to_owned(), allow));
    let refs: Vec<(&str, &str)> = all.iter().map(|(n, b)| (n.as_str(), *b)).collect();
    one_gate::check(&tree(&refs))
}

/// ci.yml (GOOD) plus the given extra workflow, required list `gate`.
fn with_extra(extra: &str) -> Vec<String> {
    with_extra_req(extra, "gate\n")
}

fn with_extra_req(extra: &str, required: &str) -> Vec<String> {
    run(&[("ci.yml", GOOD), ("extra.yml", extra)], required, ALLOW)
}

/// One ci.yml, required list `gate`.
fn only(ci: &str) -> Vec<String> {
    run(&[("ci.yml", ci)], "gate\n", ALLOW)
}

fn has(v: &[String], needle: &str) {
    assert!(
        v.iter().any(|l| l.contains(needle)),
        "no '{needle}' in {v:?}"
    );
}

fn none(v: Vec<String>) {
    assert_eq!(v, Vec::<String>::new());
}

fn main_job(on: &str) -> String {
    format!("name: x\non: {on}\njobs:\n  sneaky:\n    runs-on: x\n    steps:\n      - run: true\n")
}

const NOT_REQ: &str = "is not in .github/required-status-checks.txt";
const CLOSED: &str = "fails closed";
const IF_MSG: &str = "'if' that is not byte-equal";

// ---- positive ------------------------------------------------------------------------------

#[test]
fn good_tree_passes() {
    none(only(GOOD));
}

#[test]
fn this_repository_passes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    none(one_gate::check(&root));
}

#[test]
fn uppercase_extension_is_read() {
    none(run(&[("ci.YML", GOOD)], "gate\n", ALLOW));
    has(
        &run(
            &[("ci.yml", GOOD), ("x.YAML", &main_job("push"))],
            "gate\n",
            ALLOW,
        ),
        NOT_REQ,
    );
}

// ---- round 1 and 2 evasions ----------------------------------------------------------------

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
    has(&with_extra_req(y, "gate\nsecond\n"), IF_MSG);
}

#[test]
fn block_scalar_if() {
    let y = "on:\n  push:\n  pull_request:\njobs:\n  second:\n    if: >-\n      github.event_name\n      == 'push'\n    runs-on: x\n";
    has(&with_extra_req(y, "gate\nsecond\n"), IF_MSG);
}

#[test]
fn non_string_if() {
    let y = "on:\n  push:\n  pull_request:\njobs:\n  second:\n    if: true\n    runs-on: x\n";
    has(&with_extra_req(y, "gate\nsecond\n"), IF_MSG);
}

#[test]
fn if_allowed_only_when_byte_equal() {
    let y = "on:\n  push:\n  pull_request:\njobs:\n  second:\n    if: always()\n    runs-on: x\n";
    let ok = |allow: &str| {
        run(
            &[("ci.yml", GOOD), ("extra.yml", y)],
            "gate\nsecond\n",
            &format!("{ALLOW}{allow}"),
        )
    };
    none(ok("if always()\n"));
    has(&ok("if always() \n"), IF_MSG);
    has(&ok("# if always()\n"), IF_MSG);
}

#[test]
fn step_level_if_on_a_main_job_needs_an_allow_entry() {
    let ci = GOOD.replace(
        "- run: true",
        "- run: true\n        if: github.event_name == 'push'",
    );
    has(&only(&ci), IF_MSG);
    none(run(
        &[("ci.yml", &ci)],
        "gate\n",
        &format!("{ALLOW}if github.event_name == 'push'\n"),
    ));
}

#[test]
fn pull_request_branches_ignore_main_is_no_pr_check() {
    let y = "on:\n  push:\n    branches: [main]\n  pull_request:\n    branches-ignore: [main]\njobs:\n  gate:\n    runs-on: x\n";
    let v = with_extra(y);
    has(&v, "no pull_request trigger that covers main");
    has(&v, "'gate' is produced more than once");
}

#[test]
fn main_push_only_workflow_with_required_job() {
    let y = "on:\n  push:\n    branches: [main]\njobs:\n  second:\n    runs-on: x\n";
    let v = with_extra_req(y, "gate\nsecond\n");
    has(&v, "no pull_request trigger that covers main");
    has(&v, "no job renders that check name on pull_request");
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
        "on: push\njobs: {}\n",
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
        "on: 5\njobs:\n  a:\n    runs-on: x\n",
        "on: push\njobs:\n  a:\n    steps: x\n",
        "on: push\njobs:\n  a:\n    steps: [x]\n",
        "on: push\njobs:\n  a:\n    uses: 5\n",
    ] {
        has(&with_extra(y), CLOSED);
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
        none(push_cfg(miss));
    }
    for bad in [
        "    branches: [main]\n    branches-ignore: [x]",
        "    branches: []",
        "    branches: 5",
        "    branches: ['[']",
    ] {
        has(&push_cfg(bad), CLOSED);
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
        none(pr_cfg(ok));
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
    none(only(&GOOD.replace(
        "  pull_request:\n    branches: [main]\n",
        "  pull_request:\n",
    )));
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
    none(only(&GOOD.replace(
        "    steps:",
        "    continue-on-error: false\n    steps:",
    )));
}

#[test]
fn continue_on_error_on_a_pr_only_required_job() {
    let y = "on: pull_request\njobs:\n  second:\n    continue-on-error: true\n    runs-on: x\n";
    has(
        &with_extra_req(y, "gate\nsecond\n"),
        "second' has continue-on-error",
    );
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
    has(&v, "one-gate-allow.txt");
    none(run(&[("ci.yml", GOOD)], "gate\n", ALLOW));
    has(
        &run(&[("ci.yml", GOOD)], "# none\n\n", ALLOW),
        "lists no checks",
    );
    let t = tree(&[
        (".github/required-status-checks.txt", "gate\n"),
        (".github/one-gate-allow.txt", ALLOW),
    ]);
    has(&one_gate::check(&t), "workflows");
    let t = tree(&[
        (".github/workflows/README.md", "x"),
        (".github/required-status-checks.txt", "gate\n"),
        (".github/one-gate-allow.txt", ALLOW),
    ]);
    has(&one_gate::check(&t), "no workflow files");
    let t = tree(&[
        (".github/workflows/d.yml/keep", ""),
        (".github/required-status-checks.txt", "gate\n"),
        (".github/one-gate-allow.txt", ALLOW),
    ]);
    has(&one_gate::check(&t), "d.yml");
    has(
        &run(&[("ci.yaml", GOOD)], "gate\nghost\n", ALLOW),
        "'ghost'",
    );
}

// ---- round 3 refusals ------------------------------------------------------------------------

const HELPER: &str = "on:\n  push:\n  pull_request:\njobs:\n  helper:\n    runs-on: x\n    steps:\n      - run: false\n";

/// 3a: a gate that needs an unrequired helper fails open (GitHub reports a skipped job as
/// Success, so a red helper would let the PR merge).
#[test]
fn needs_helper_that_is_not_required_is_red() {
    let ci = GOOD.replace("    name: gate\n", "    name: gate\n    needs: helper\n");
    let v = run(&[("ci.yml", &ci), ("h.yml", HELPER)], "gate\n", ALLOW);
    has(&v, "needs 'helper', which is not a job of the workflow");
    has(&v, "check name 'helper', which is not in");
}

#[test]
fn needs_helper_in_the_same_file_is_red() {
    let ci = format!("{GOOD}  helper:\n    runs-on: x\n    steps:\n      - run: false\n")
        .replace("    name: gate\n", "    name: gate\n    needs: helper\n");
    let v = only(&ci);
    has(&v, "needs 'helper', which is not itself required");
}

#[test]
fn needs_a_required_job_is_fine() {
    let ci = format!("{GOOD}  helper:\n    runs-on: x\n    steps:\n      - run: true\n")
        .replace("    name: gate\n", "    name: gate\n    needs: helper\n");
    none(run(&[("ci.yml", &ci)], "gate\nhelper\n", ALLOW));
}

#[test]
fn needs_a_job_that_does_not_exist() {
    let ci = GOOD.replace("    name: gate\n", "    name: gate\n    needs: ghost\n");
    has(&only(&ci), "needs 'ghost', which is not a job");
}

const AGG: &str = "on:
  push:
  pull_request:
jobs:
  helper:
    runs-on: x
    steps:
      - run: true
  lint:
    runs-on: x
    steps:
      - run: true
  gate:
    needs: [helper, lint]
    if: always()
    runs-on: x
    steps:
      - if: needs.helper.result != 'success' || needs.lint.result != 'success'
        run: exit 1
";

#[test]
fn the_committed_aggregate_shape_guards_its_needs() {
    none(run(&[("ci.yml", AGG)], "gate\n", ALLOW));
    none(run(
        &[(
            "ci.yml",
            &AGG.replace("if: always()", "if: ${{ always() }}"),
        )],
        "gate\n",
        ALLOW,
    ));
    none(run(
        &[(
            "ci.yml",
            &AGG.replace("if: needs.helper", "if: ${{ needs.helper")
                .replace("'success'\n        run", "'success' }}\n        run"),
        )],
        "gate\n",
        ALLOW,
    ));
}

#[test]
fn aggregate_shape_variants_are_red() {
    let bad = [
        AGG.replace("    if: always()\n", ""),
        AGG.replace("if: always()", "if: success()"),
        AGG.replace("if: always()", "if: github.ref == 'x'"),
        AGG.replace(" || needs.lint.result != 'success'", ""),
        AGG.replace("needs.lint.result != 'success'", "needs.helper.result != 'success'"),
        AGG.replace("needs.lint.result != 'success'", "needs.lint.result == 'failure'"),
        AGG.replace("run: exit 1", "run: exit 0"),
        AGG.replace("      - if: needs.helper.result != 'success' || needs.lint.result != 'success'\n        run: exit 1", "      - run: exit 1"),
        AGG.replace("      - if: needs.helper.result != 'success' || needs.lint.result != 'success'\n        run: exit 1", "      - if: true\n        run: exit 1"),
        format!("{AGG}      - run: true\n"),
        AGG.replace("      - if: needs.helper.result != 'success' || needs.lint.result != 'success'\n        run: exit 1\n", ""),
        AGG.replace("    needs: [helper, lint]\n", "").replace("      - if: needs.helper.result != 'success' || needs.lint.result != 'success'", "      - if: false"),
        AGG.replace("    runs-on: x\n    steps:\n      - if", "    runs-on: x\n    steps:\n      - continue-on-error: true\n        if"),
    ];
    for y in bad {
        let v = run(&[("ci.yml", &y)], "gate\n", ALLOW);
        assert!(!v.is_empty(), "aggregate variant passed:\n{y}");
    }
}

#[test]
fn aggregate_needs_still_carry_continue_on_error_and_if_rules() {
    let y = AGG.replace(
        "  lint:\n    runs-on: x\n",
        "  lint:\n    runs-on: x\n    continue-on-error: true\n",
    );
    has(
        &run(&[("ci.yml", &y)], "gate\n", ALLOW),
        "continue-on-error",
    );
    let y = AGG.replace(
        "  lint:\n    runs-on: x\n",
        "  lint:\n    if: always()\n    runs-on: x\n",
    );
    has(&run(&[("ci.yml", &y)], "gate\n", ALLOW), IF_MSG);
}

/// 3b: the collision guard compares rendered check names, not job ids.
#[test]
fn pr_only_job_named_gate_collides() {
    let y = "on: pull_request\njobs:\n  shadow:\n    name: gate\n    runs-on: x\n    steps:\n      - run: true\n";
    has(
        &with_extra(y),
        "check name 'gate' is produced more than once",
    );
}

#[test]
fn matrix_job_producing_one_name_twice_collides() {
    let y = "on: pull_request\njobs:\n  shadow:\n    name: fixed\n    strategy:\n      matrix:\n        a: [1, 2]\n    runs-on: x\n";
    has(&with_extra(y), "'fixed' is produced more than once");
}

/// 3c: unknown events are RED, not silently neutral.
#[test]
fn unknown_events_fail_closed() {
    for ev in [
        "page_build",
        "status",
        "check_suite",
        "release",
        "repository_dispatch",
        "[issues, push]",
        "{label: }",
        "schedule",
    ] {
        has(
            &with_extra(&main_job(ev)),
            "not in .github/one-gate-allow.txt",
        );
    }
}

#[test]
fn schedule_is_neutral_only_in_the_files_that_list_it() {
    let sched = "on:\n  schedule:\n    - cron: '1 1 * * *'\n  workflow_dispatch:\njobs:\n  live:\n    runs-on: x\n    steps:\n      - run: true\n";
    none(run(
        &[("ci.yml", GOOD), ("live.yml", sched)],
        "gate\n",
        ALLOW,
    ));
    has(
        &run(&[("ci.yml", GOOD), ("other.yml", sched)], "gate\n", ALLOW),
        "not in .github/one-gate-allow.txt",
    );
}

#[test]
fn an_allow_list_may_class_other_events_as_main() {
    let allow = format!("{ALLOW}trigger page_build main\n");
    has(
        &run(
            &[("ci.yml", GOOD), ("x.yml", &main_job("page_build"))],
            "gate\n",
            &allow,
        ),
        NOT_REQ,
    );
}

#[test]
fn pull_request_target_is_never_allowed() {
    has(
        &with_extra(&main_job("pull_request_target")),
        "pull_request_target is never allowed",
    );
    let v = run(
        &[("ci.yml", GOOD)],
        "gate\n",
        &format!("{ALLOW}trigger pull_request_target neutral\n"),
    );
    has(&v, "never allowed");
}

#[test]
fn unreadable_allow_lists_fail_closed() {
    for bad in [
        "trigger\n",
        "trigger push\n",
        "trigger push sideways\n",
        "trigger push neutral\n",
        "trigger pull_request main\n",
        "trigger workflow_call main\n",
        "trigger release pr\n",
        "remote\n",
        "remote owner/repo/x.yml\n",
        "remote owner/repo/x.yml@v1\n",
        "remote owner/repo/x.yml name\n",
        "if\n",
        "if \n",
        "ifx y\n",
        "other thing\n",
    ] {
        let v = run(&[("ci.yml", GOOD)], "gate\n", bad);
        has(&v, "one-gate-allow.txt line 1");
    }
    let v = run(
        &[("ci.yml", GOOD)],
        "gate\n",
        "trigger push main\ntrigger bogus\n# c\n\n",
    );
    has(&v, "line 2");
}

#[test]
fn conflicting_trigger_classes_fail_closed() {
    let allow = format!("{ALLOW}trigger schedule main\n");
    let sched = "on: schedule\njobs:\n  live:\n    runs-on: x\n    steps:\n      - run: true\n";
    has(
        &run(&[("ci.yml", GOOD), ("live.yml", sched)], "gate\n", &allow),
        "conflicting classes",
    );
}

// ---- rendered check names --------------------------------------------------------------------

fn matrix_ci(strategy: &str, name: &str) -> String {
    GOOD.replace(
        "    name: gate\n",
        &format!("{name}    strategy:\n{strategy}\n"),
    )
}

#[test]
fn matrix_names_are_rendered() {
    let ci = matrix_ci("      matrix:\n        os: [a, b]\n        n: [1, 2]", "");
    let req = "gate (a, 1)\ngate (a, 2)\ngate (b, 1)\ngate (b, 2)\n";
    none(run(&[("ci.yml", &ci)], req, ALLOW));
    let v = run(&[("ci.yml", &ci)], "gate (a, 1)\n", ALLOW);
    has(&v, "check name 'gate (b, 2)', which is not in");
    has(&only(&ci), "'gate' but no job renders");
    let named = matrix_ci(
        "      matrix:\n        os: [a, b]\n        fast: [true]",
        "    name: build ${{ matrix.os }} ${{ matrix.fast }}\n",
    );
    none(run(
        &[("ci.yml", &named)],
        "build a true\nbuild b true\n",
        ALLOW,
    ));
    let plain = matrix_ci("      fail-fast: false", "");
    none(only(&plain));
    let no_matrix = matrix_ci("      fail-fast: false\n      matrix:\n        k: [x]", "");
    none(run(&[("ci.yml", &no_matrix)], "gate (x)\n", ALLOW));
}

#[test]
fn names_that_cannot_be_rendered_fail_closed() {
    for ci in [
        matrix_ci("      matrix:\n        include:\n          - a: 1", ""),
        matrix_ci("      matrix:\n        a: [1]\n        exclude:\n          - a: 1", ""),
        matrix_ci("      matrix: ${{ fromJSON(needs.x.outputs.m) }}", ""),
        matrix_ci("      matrix: {}", ""),
        matrix_ci("      matrix:\n        a: ${{ x }}", ""),
        matrix_ci("      matrix:\n        a: []", ""),
        matrix_ci("      matrix:\n        a: [[1]]", ""),
        matrix_ci("      matrix:\n        a: ['${{ x }}']", ""),
        matrix_ci("      matrix:\n        a: [~]", ""),
        matrix_ci("      matrix:\n        1: [a]", ""),
        matrix_ci("      matrix:\n        a: [1]", "    name: g ${{ github.run_id }}\n"),
        matrix_ci("      matrix:\n        a: [1]", "    name: g ${{ matrix.zzz }}\n"),
        GOOD.replace("name: gate", "name: ${{ matrix.a }}"),
        GOOD.replace("    steps:", "    strategy: x\n    steps:"),
        GOOD.replace("    steps:", "    strategy:\n      matrix:\n        a: [1]\n        b: [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17]\n        c: [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]\n    steps:"),
    ] {
        let v = run(&[("ci.yml", &ci)], "gate\n", ALLOW);
        assert!(v.iter().any(|l| l.contains(CLOSED)), "{ci}\n{v:?}");
    }
}

#[test]
fn required_name_produced_by_a_non_pr_job() {
    let y = "on: push\njobs:\n  second:\n    runs-on: x\n";
    has(
        &with_extra_req(y, "gate\nsecond\n"),
        "no job renders that check name on pull_request",
    );
}

// ---- reusable workflows ----------------------------------------------------------------------

const CALLEE: &str = "on: workflow_call\njobs:\n  inner:\n    runs-on: x\n    steps:\n      - run: true\n  more:\n    name: more ${{ matrix.v }}\n    strategy:\n      matrix:\n        v: [1, 2]\n    runs-on: x\n";

fn caller(uses: &str) -> String {
    format!(
        "on:\n  push:\n  pull_request:\njobs:\n  call:\n    uses: {uses}\n  gate:\n    runs-on: x\n    steps:\n      - run: true\n"
    )
}

const CALLED: &str = "./.github/workflows/callee.yml";
const CALLED_NAMES: &str = "gate\ncall / inner\ncall / more 1\ncall / more 2\n";

#[test]
fn local_callees_render_caller_slash_inner() {
    none(run(
        &[("ci.yml", &caller(CALLED)), ("callee.yml", CALLEE)],
        CALLED_NAMES,
        ALLOW,
    ));
    let v = run(
        &[("ci.yml", &caller(CALLED)), ("callee.yml", CALLEE)],
        "gate\ncall / inner\n",
        ALLOW,
    );
    has(&v, "check name 'call / more 1', which is not in");
    let v = run(
        &[("ci.yml", &caller(CALLED)), ("callee.yml", CALLEE)],
        "gate\ncall\ninner\n",
        ALLOW,
    );
    has(&v, "'call' but no job renders");
}

#[test]
fn callee_with_a_main_trigger_of_its_own_is_judged_standalone() {
    let both = CALLEE.replace("on: workflow_call", "on: [workflow_call, push]");
    let v = run(
        &[("ci.yml", &caller(CALLED)), ("callee.yml", &both)],
        CALLED_NAMES,
        ALLOW,
    );
    has(&v, "callee.yml: job 'inner' runs on push to main");
}

#[test]
fn callee_inner_jobs_carry_the_if_and_continue_on_error_rules() {
    let iff = CALLEE.replace(
        "    runs-on: x\n    steps:\n      - run: true\n  more",
        "    if: github.head_ref == ''\n    runs-on: x\n    steps:\n      - run: true\n  more",
    );
    has(
        &run(
            &[("ci.yml", &caller(CALLED)), ("callee.yml", &iff)],
            CALLED_NAMES,
            ALLOW,
        ),
        IF_MSG,
    );
    let step_if = CALLEE.replace("- run: true", "- run: true\n        if: false");
    has(
        &run(
            &[("ci.yml", &caller(CALLED)), ("callee.yml", &step_if)],
            CALLED_NAMES,
            ALLOW,
        ),
        IF_MSG,
    );
    let coe = CALLEE.replace("  inner:\n", "  inner:\n    continue-on-error: true\n");
    has(
        &run(
            &[("ci.yml", &caller(CALLED)), ("callee.yml", &coe)],
            CALLED_NAMES,
            ALLOW,
        ),
        "continue-on-error",
    );
}

#[test]
fn callee_chains_are_followed() {
    let mid = "on: workflow_call\njobs:\n  hop:\n    uses: ./.github/workflows/leaf.yml\n";
    let leaf = "on: workflow_call\njobs:\n  deep:\n    continue-on-error: true\n    runs-on: x\n";
    let v = run(
        &[
            ("ci.yml", &caller("./.github/workflows/mid.yml")),
            ("mid.yml", mid),
            ("leaf.yml", leaf),
        ],
        "gate\ncall / hop / deep\n",
        ALLOW,
    );
    has(&v, "continue-on-error");
}

#[test]
fn callee_problems_fail_closed() {
    let no_call = CALLEE.replace("workflow_call", "workflow_dispatch");
    let looped = "on: workflow_call\njobs:\n  again:\n    uses: ./.github/workflows/callee.yml\n";
    for (uses, callee) in [
        ("./.github/workflows/missing.yml", CALLEE),
        ("./other/callee.yml", CALLEE),
        ("./.github/workflows/../callee.yml", CALLEE),
        (CALLED, no_call.as_str()),
        (CALLED, "on: workflow_call\njobs: {}\n"),
        (CALLED, "not yaml: ["),
        (CALLED, looped),
        ("owner/repo/.github/workflows/x.yml@v1", CALLEE),
    ] {
        let v = run(
            &[("ci.yml", &caller(uses)), ("callee.yml", callee)],
            "gate\n",
            ALLOW,
        );
        has(&v, CLOSED);
    }
}

#[test]
fn remote_callee_only_when_pinned_by_exact_ref() {
    let uses = "paiml/.github/.github/workflows/pr-gate.yml@abc123";
    let allow = format!("{ALLOW}remote {uses} gate-inner other\n");
    none(run(
        &[("ci.yml", &caller(uses))],
        "gate\ncall / gate-inner\ncall / other\n",
        &allow,
    ));
    let v = run(
        &[(
            "ci.yml",
            &caller("paiml/.github/.github/workflows/pr-gate.yml@main"),
        )],
        "gate\n",
        &allow,
    );
    has(&v, "remote and not pinned");
}

// ---- round 4: what class of input was still accepted without being understood ---------------

/// The aggregate may carry only the keys of its committed shape. Anything else (a shell that
/// makes `exit 1` a no-op, defaults at any level, env, a container) is not understood: RED.
#[test]
fn aggregate_with_any_unlisted_key_is_red() {
    let step = "      - if: needs.helper.result != 'success' || needs.lint.result != 'success'\n        run: exit 1\n";
    let with_step_key = |k: &str| AGG.replace(step, &format!("{step}        {k}\n"));
    let with_job_key = |k: &str| {
        AGG.replace(
            "    if: always()\n",
            &format!("    if: always()\n    {k}\n"),
        )
    };
    let mut bad = vec![
        with_step_key("shell: 'true {0}'"),
        with_step_key("env: {BASH_ENV: x}"),
        with_step_key("working-directory: /"),
        with_step_key("timeout-minutes: 1"),
        with_job_key("defaults: {run: {shell: 'true {0}'}}"),
        with_job_key("env: {A: b}"),
        with_job_key("container: alpine"),
        with_job_key("strategy: {fail-fast: false}"),
        with_job_key("outputs: {a: b}"),
        AGG.replace(
            "jobs:\n",
            "defaults:\n  run:\n    shell: 'true {0}'\njobs:\n",
        ),
        AGG.replace(
            "jobs:\n",
            "defaults:\n  run:\n    working-directory: x\njobs:\n",
        ),
    ];
    bad.push(AGG.replace(step, &format!("      - uses: x/y@v1\n{step}")));
    for y in bad {
        let v = run(&[("ci.yml", &y)], "gate\n", ALLOW);
        has(&v, "not the aggregate shape");
    }
    // The listed keys stay legitimate.
    let ok = AGG
        .replace(
            "    if: always()\n",
            "    if: always()\n    name: gate\n    timeout-minutes: 5\n",
        )
        .replace(
            step,
            &format!(
                "      - name: fail\n{}",
                step.replacen("      - if", "        if", 1)
            ),
        );
    none(run(&[("ci.yml", &ok)], "gate\n", ALLOW));
}

/// A branch pattern is understood or refused: escapes, a `refs/` prefix and every character
/// outside the glob alphabet are RED on both sides, never read literally.
#[test]
fn unknown_branch_pattern_syntax_is_red() {
    for pat in [
        "'ma\\in'",
        "'refs/heads/main'",
        "'{main,dev}'",
        "'ma(in)'",
        "'ma in'",
        "'ma!in'",
        "'${{ github.base_ref }}'",
        "'mäin'",
    ] {
        has(&push_cfg(&format!("    branches: [{pat}]")), CLOSED);
        has(&push_cfg(&format!("    branches-ignore: [{pat}]")), CLOSED);
        has(
            &push_cfg(&format!(
                "    branches: ['**', '!{}']",
                pat.trim_matches('\'')
            )),
            CLOSED,
        );
        has(&pr_cfg(&format!("    branches: [{pat}]")), CLOSED);
    }
}

/// Check names collide when they fold together (trim, case, inner whitespace), and a required
/// name must match its producer exactly.
#[test]
fn folded_check_names_collide() {
    for n in ["'gate '", "' gate'", "Gate", "GATE", "'ga  te'"] {
        let y = format!(
            "on: pull_request\njobs:\n  shadow:\n    name: {n}\n    runs-on: x\n    steps:\n      - run: true\n"
        );
        let v = with_extra(&y);
        if n == "'ga  te'" {
            none(v);
        } else {
            has(&v, "is produced more than once");
        }
    }
    let ci = GOOD.replace("name: gate", "name: Gate");
    has(
        &run(&[("ci.yml", &ci)], "gate\n", ALLOW),
        "no job renders that check name",
    );
}
