//! The gate must be able to fail: delete, alter, stale the manifest, or unseat the README hero,
//! and `rll-hero check` exits 1 naming the cause. The committed repo must also check green.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(root: &Path, arg: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rll-hero"))
        .env("RLL_HERO_ROOT", root)
        .arg(arg)
        .output()
        .expect("spawn rll-hero")
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// A scratch root holding a fresh hero, its manifest and a README that opens with it.
fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("rll-hero-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("README.md"),
        "# T\n\n![hero](docs/hero.png)\n\ntext\n",
    )
    .unwrap();
    let o = run(&root, "write");
    assert!(o.status.success(), "{}", err(&o));
    root
}

#[test]
fn the_committed_hero_is_current() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let o = run(&repo, "check");
    assert!(o.status.success(), "{}", err(&o));
    assert!(String::from_utf8_lossy(&o.stdout).contains("is current, sha256"));
}

#[test]
fn a_fresh_scratch_root_checks_green() {
    let root = scratch("green");
    assert!(run(&root, "check").status.success());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn a_missing_hero_is_red() {
    let root = scratch("missing");
    fs::remove_file(root.join("docs/hero.png")).unwrap();
    let o = run(&root, "check");
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("docs/hero.png is missing"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn an_altered_hero_is_red() {
    let root = scratch("altered");
    let p = root.join("docs/hero.png");
    let mut png = fs::read(&p).unwrap();
    let mid = png.len() / 2;
    png[mid] ^= 0xff;
    fs::write(&p, png).unwrap();
    let o = run(&root, "check");
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("stale or altered"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn a_missing_or_stale_manifest_is_red() {
    let root = scratch("manifest");
    let p = root.join("evidence/hero.jsonl");
    let row = fs::read_to_string(&p).unwrap();
    fs::write(&p, row.replace("\"bytes\":", "\"bytes\":1")).unwrap();
    let o = run(&root, "check");
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("evidence/hero.jsonl is stale"));
    fs::remove_file(&p).unwrap();
    let o = run(&root, "check");
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("evidence/hero.jsonl is missing"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn a_readme_that_does_not_open_with_the_hero_is_red() {
    let root = scratch("readme");
    fs::write(
        root.join("README.md"),
        "# T\n\ntext first\n\n![h](docs/hero.png)\n",
    )
    .unwrap();
    let o = run(&root, "check");
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("does not open with"));
    fs::remove_file(root.join("README.md")).unwrap();
    let o = run(&root, "check");
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("README.md:"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn no_argument_prints_usage() {
    let o = Command::new(env!("CARGO_BIN_EXE_rll-hero"))
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(err(&o).contains("usage"));
}

#[test]
fn help_prints_usage_and_succeeds() {
    let o = Command::new(env!("CARGO_BIN_EXE_rll-hero"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).contains("usage: rll-hero"));
}
