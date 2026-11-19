//! `rll-hero write` regenerates docs/hero.png and evidence/hero.jsonl; `rll-hero check` is the
//! gate (exit 1 when the hero is missing or stale). The repo root is `RLL_HERO_ROOT` or `.`.

use std::path::PathBuf;
use std::process::exit;

fn main() {
    let root = PathBuf::from(std::env::var("RLL_HERO_ROOT").unwrap_or_else(|_| ".".into()));
    match std::env::args().nth(1).as_deref() {
        Some("write") => {
            let sha = rll_hero::write(&root).expect("write hero");
            println!("hero: wrote {} sha256 {sha}", rll_hero::HERO);
        }
        Some("check") => match rll_hero::verify(&root) {
            Ok(sha) => println!("hero: {} is current, sha256 {sha}", rll_hero::HERO),
            Err(why) => {
                eprintln!("hero: RED: {why}");
                exit(1);
            }
        },
        _ => {
            eprintln!("usage: rll-hero write|check");
            exit(2);
        }
    }
}
