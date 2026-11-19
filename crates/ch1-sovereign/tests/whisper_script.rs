//! Run scripts/whisper-pipeline.sh offline with a stub whisper binary and check its
//! dry-run output against the Rust contract; run the shell test cases too.

use ch1_sovereign::whisper::merged_contract_holds;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn script(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts")
        .join(name)
}

#[test]
fn dry_run_output_satisfies_the_rust_contract() {
    let dir = std::env::temp_dir().join(format!("rll-whisper-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let stub = dir.join("whisper-stub");
    std::fs::write(
        &stub,
        "#!/usr/bin/env bash\nwhile [ \"$#\" -gt 0 ]; do [ \"$1\" = -f ] && echo \"heard $(basename \"$2\")\"; shift; done\n",
    )
    .unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let (a, b) = (dir.join("a.wav"), dir.join("b.wav"));
    std::fs::write(&a, b"").unwrap();
    std::fs::write(&b, b"").unwrap();
    let o = Command::new(script("whisper-pipeline.sh"))
        .env("WHISPER_BIN", &stub)
        .env("WHISPER_MODEL", "x")
        .env("RLL_DRY_RUN", "1")
        .args([&a, &b])
        .output()
        .unwrap();
    assert!(o.status.success());
    let out = String::from_utf8(o.stdout).unwrap();
    let (merged, last) = out.trim_end().rsplit_once('\n').unwrap();
    assert_eq!(merged, "heard a.wav\nheard b.wav");
    assert_eq!(
        last,
        "contract: merged transcript has one non-empty line per file OK"
    );
    assert!(merged_contract_holds(merged, 2));
    assert!(!merged_contract_holds(merged, 3));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn shell_test_cases_pass() {
    let o = Command::new(script("test-whisper-pipeline.sh"))
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(String::from_utf8_lossy(&o.stdout).contains("whisper-pipeline test OK"));
}
