//! The `one-gate` bin: exit status and output, with an explicit root and with the default root.

use std::path::PathBuf;
use std::process::Command;

fn tree(name: &str, workflow: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("bin-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".github/workflows")).unwrap();
    std::fs::write(root.join(".github/workflows/ci.yml"), workflow).unwrap();
    std::fs::write(root.join(".github/required-status-checks.txt"), "gate\n").unwrap();
    root
}

const GOOD: &str = "on:\n  push:\n  pull_request:\njobs:\n  gate:\n    runs-on: x\n";

#[test]
fn passes_on_an_explicit_root() {
    let root = tree("pass", GOOD);
    let o = Command::new(env!("CARGO_BIN_EXE_one-gate"))
        .arg(&root)
        .output()
        .unwrap();
    assert!(o.status.success());
    assert_eq!(String::from_utf8_lossy(&o.stdout), "one-gate: PASS\n");
}

#[test]
fn fails_on_the_current_directory_by_default() {
    let root = tree("fail", &GOOD.replace("  pull_request:\n", ""));
    let o = Command::new(env!("CARGO_BIN_EXE_one-gate"))
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("one-gate: FAIL: ci.yml: job 'gate'"));
}
