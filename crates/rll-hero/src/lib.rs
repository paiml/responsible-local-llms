//! The README hero image, drawn by Rust from this repo's own offline demo outputs.
//!
//! `write` regenerates `docs/hero.png` and its manifest row `evidence/hero.jsonl`; `verify` is
//! the CI gate: it regenerates in memory and goes RED when the image is missing, differs from
//! the regeneration (stale or altered), the manifest row is missing or stale, or the README
//! does not open with the image.

pub mod canvas;
pub mod png;
pub mod scene;
pub mod sha256;

use std::fs;
use std::path::Path;

pub const HERO: &str = "docs/hero.png";
pub const MANIFEST: &str = "evidence/hero.jsonl";
const OPENER_END: &str = "](docs/hero.png)";

/// The PNG bytes, the same on every run.
pub fn render() -> Vec<u8> {
    scene::draw(&scene::gather()).to_png()
}

/// The manifest row for a PNG.
pub fn manifest_row(png: &[u8]) -> String {
    format!(
        "{{\"file\":\"{HERO}\",\"sha256\":\"{}\",\"bytes\":{},\"width\":{},\"height\":{},\"generator\":\"rll-hero\"}}\n",
        sha256::hex(png),
        png.len(),
        canvas::W,
        canvas::H
    )
}

/// Regenerate the image and its manifest row under `root`. Returns the sha256.
pub fn write(root: &Path) -> std::io::Result<String> {
    let png = render();
    fs::create_dir_all(root.join("docs"))?;
    fs::create_dir_all(root.join("evidence"))?;
    fs::write(root.join(HERO), &png)?;
    fs::write(root.join(MANIFEST), manifest_row(&png))?;
    Ok(sha256::hex(&png))
}

/// The README opens with the hero: the first line after the title is the image.
pub fn opens_with_hero(readme: &str) -> bool {
    readme
        .lines()
        .skip(1)
        .find(|l| !l.trim().is_empty())
        .is_some_and(|l| l.starts_with("![") && l.ends_with(OPENER_END))
}

/// The gate. `Ok` carries the sha256; `Err` names what is wrong.
pub fn verify(root: &Path) -> Result<String, String> {
    let want = render();
    let have = fs::read(root.join(HERO)).map_err(|e| format!("{HERO} is missing: {e}"))?;
    if have != want {
        return Err(format!(
            "{HERO} is stale or altered: sha256 {} but regenerating gives {}; run `make hero-write`",
            sha256::hex(&have),
            sha256::hex(&want)
        ));
    }
    let row = fs::read_to_string(root.join(MANIFEST))
        .map_err(|e| format!("{MANIFEST} is missing: {e}"))?;
    if row != manifest_row(&want) {
        return Err(format!("{MANIFEST} is stale; run `make hero-write`"));
    }
    let readme =
        fs::read_to_string(root.join("README.md")).map_err(|e| format!("README.md: {e}"))?;
    if !opens_with_hero(&readme) {
        return Err("README.md does not open with ![...](docs/hero.png)".into());
    }
    Ok(sha256::hex(&want))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opener_must_be_the_first_line_after_the_title() {
        assert!(opens_with_hero("# T\n\n![x](docs/hero.png)\n\ntext"));
        assert!(!opens_with_hero("# T\n\ntext\n\n![x](docs/hero.png)"));
        assert!(!opens_with_hero("# T\n"));
    }

    #[test]
    fn rendering_twice_gives_the_same_bytes() {
        assert_eq!(render(), render());
    }
}
