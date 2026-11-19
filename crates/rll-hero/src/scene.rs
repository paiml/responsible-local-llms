//! The hero scene: four panels, each drawn from values this repo's own offline demos return.
//! Quantization sizes (lesson 1.3), local vs hosted cost (1.4), Elo ratings (2.5), red-team
//! results (3.5). Nothing here is typed in: each panel calls the demo's own `run` with the
//! demo's own default config and reads the summary it returns.

use crate::canvas::*;
use ch1_sovereign::{
    economics::{self, Inputs},
    quantize,
};
use ch2_evals::elo;
use ch3_redteam::redteam::{self, CategoryRow};
use rll_client::Config;

pub struct Data {
    pub sizes: Vec<(&'static str, f64)>,
    pub inputs: Inputs,
    pub ratings: Vec<(String, f64)>,
    pub safe: Vec<CategoryRow>,
    pub vulnerable: Vec<CategoryRow>,
}

/// The config the demo bins use with no `RLL_*` variables set (offline fixtures).
fn demo_config() -> Config {
    Config::from_lookup(|_| None).expect("default config parses")
}

/// Run the offline demos and keep what the panels need.
pub fn gather() -> Data {
    let cfg = demo_config();
    let sizes = quantize::run(&cfg, false, &mut Vec::new())
        .expect("quantize demo runs offline")
        .sizes;
    let inputs = economics::run(&cfg, false, &mut Vec::new())
        .expect("economics demo runs offline")
        .inputs;
    let ratings = elo::run(&mut Vec::new())
        .expect("elo demo runs offline")
        .ratings;
    let rt = redteam::run(&cfg, false, &mut Vec::new()).expect("red-team demo runs offline");
    Data {
        sizes,
        inputs,
        ratings,
        safe: redteam::by_category(&rt.model_sampled),
        vulnerable: redteam::by_category(&rt.vulnerable),
    }
}

/// Panel 1 note: the model size is the one lesson 1.3 sizes.
pub fn sizes_note() -> String {
    format!(
        "{:.1}B MODEL, GGUF QUANTIZATION LEVELS",
        quantize::PARAMS / 1e9
    )
}

/// Panel 1 bar label, in the units lesson 1.3 prints scaled to GB.
pub fn size_label(bytes: f64) -> String {
    format!("{:.2} GB", bytes / 1e9)
}

/// Panel 2 break-even label, the figure lesson 1.4 prints.
pub fn breakeven_label(mtok: f64) -> String {
    format!("BREAK-EVEN {mtok:.1} MTOK/MONTH")
}

/// Panel 2 note: the speed lesson 1.4 measures.
pub fn speed_note(tokens_per_second: f64) -> String {
    format!("USD PER MONTH VS MTOK PER MONTH, AT {tokens_per_second:.1} TOK/S")
}

const PW: usize = 580;
const PH: usize = 250;

fn panel(c: &mut Canvas, x: usize, y: usize, title: &str, note: &str) {
    c.rect(x, y, PW, PH, PANEL);
    c.text(x + 16, y + 14, title, 2, TEXT);
    c.text(x + 16, y + 34, note, 1, MUTED);
}

fn sizes(c: &mut Canvas, x: usize, y: usize, d: &Data) {
    panel(c, x, y, "SIZE ON DISK", &sizes_note());
    let base = y + PH - 40;
    let top = d.sizes.iter().map(|s| s.1).fold(0.0, f64::max);
    c.rect(x + 20, base, PW - 40, 2, GRID);
    for (i, (name, bytes)) in d.sizes.iter().enumerate() {
        let h = (bytes / top * 140.0) as usize;
        let bx = x + 50 + i * 180;
        c.rect(bx, base - h, 120, h, [GREEN, BLUE, AMBER][i % 3]);
        c.text(bx, base - h - 22, &size_label(*bytes), 2, TEXT);
        c.text(bx, base + 12, name, 2, MUTED);
    }
}

fn cost(c: &mut Canvas, x: usize, y: usize, d: &Data) {
    panel(
        c,
        x,
        y,
        "LOCAL VS HOSTED COST",
        &speed_note(d.inputs.tokens_per_second),
    );
    let be = d
        .inputs
        .breakeven_mtok()
        .expect("fast hardware breaks even");
    let (x0, x1, y0, y1) = (x + 40, x + PW - 20, y + 70, y + PH - 30);
    let top = d.inputs.hosted_cost(2.0 * be);
    let at = |v: f64| y1 - (v / top * (y1 - y0) as f64) as usize;
    c.rect(x0, y0, 2, y1 - y0, GRID);
    c.rect(x0, y1, x1 - x0, 2, GRID);
    for px in x0..x1 {
        let v = (px - x0) as f64 / (x1 - x0) as f64 * 2.0 * be;
        c.rect(px, at(d.inputs.hosted_cost(v)), 3, 3, BLUE);
        c.rect(px, at(d.inputs.local_cost(v)), 3, 3, GREEN);
    }
    let mid = (x0 + x1) / 2;
    c.rect(mid, y0, 2, y1 - y0, AMBER);
    c.text(mid + 8, y0 + 4, &breakeven_label(be), 1, AMBER);
    c.text(x0 + 10, y0 + 4, "HOSTED", 1, BLUE);
    c.text(x0 + 10, y0 + 16, "LOCAL", 1, GREEN);
}

fn ratings(c: &mut Canvas, x: usize, y: usize, d: &Data) {
    panel(
        c,
        x,
        y,
        "ELO RANKING",
        "THREE MODELS, HEAD TO HEAD ON THE SAME CASES",
    );
    let top = d.ratings.iter().map(|r| r.1).fold(0.0, f64::max);
    for (i, (name, r)) in d.ratings.iter().enumerate() {
        let ry = y + 80 + i * 50;
        let w = ((r - 800.0).max(0.0) / (top - 800.0) * 330.0) as usize;
        c.text(x + 16, ry + 8, name, 2, MUTED);
        c.rect(x + 150, ry, w, 32, [GREEN, BLUE, AMBER][i % 3]);
        c.text(x + 160 + w, ry + 8, &format!("{r:.0}"), 2, TEXT);
    }
}

fn redteam_panel(c: &mut Canvas, x: usize, y: usize, d: &Data) {
    panel(
        c,
        x,
        y,
        "RED TEAM",
        "ATTACKS DEFENDED PER CATEGORY: GREEN SAFE, RED VULNERABLE",
    );
    for (i, (safe, bad)) in d.safe.iter().zip(&d.vulnerable).enumerate() {
        let ry = y + 56 + i * 38;
        let wide = |r: &CategoryRow| (f64::from(r.passes) / f64::from(r.trials) * 300.0) as usize;
        c.text(x + 16, ry + 8, safe.category, 2, MUTED);
        c.rect(x + 150, ry, wide(safe), 14, GREEN);
        c.rect(x + 150, ry + 16, wide(bad), 14, RED);
        c.text(
            x + 160 + wide(safe),
            ry + 2,
            &format!("{}/{}", safe.passes, safe.trials),
            1,
            TEXT,
        );
        c.text(
            x + 160 + wide(bad),
            ry + 18,
            &format!("{}/{}", bad.passes, bad.trials),
            1,
            TEXT,
        );
    }
}

/// Draw the whole hero.
pub fn draw(d: &Data) -> Canvas {
    let mut c = Canvas::new();
    let title = "RESPONSIBLE GENAI AND LOCAL LLMS";
    c.text((W - Canvas::text_width(title, 4)) / 2, 18, title, 4, TEXT);
    let sub = "DRAWN BY RLL-HERO FROM THIS REPOS OWN OFFLINE DEMO OUTPUTS";
    c.text((W - Canvas::text_width(sub, 2)) / 2, 56, sub, 2, MUTED);
    sizes(&mut c, 40, 96, d);
    cost(&mut c, 660, 96, d);
    ratings(&mut c, 40, 366, d);
    redteam_panel(&mut c, 660, 366, d);
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use ch3_redteam::redteam::CategoryRow;

    #[test]
    fn size_panel_is_what_lesson_1_3_returns_and_prints() {
        let cfg = demo_config();
        let mut text = String::new();
        let demo = {
            let mut out = Vec::new();
            let s = quantize::run(&cfg, false, &mut out).unwrap();
            text.push_str(&String::from_utf8(out).unwrap());
            s
        };
        let d = gather();
        assert_eq!(d.sizes, demo.sizes);
        assert!((quantize::PARAMS - 0.6e9).abs() < 1.0);
        assert_eq!(sizes_note(), "0.6B MODEL, GGUF QUANTIZATION LEVELS");
        for (name, bytes) in &d.sizes {
            // The demo prints GiB; the bar label is the same bytes in GB.
            assert!(text.contains(name));
            assert_eq!(size_label(*bytes), format!("{:.2} GB", bytes / 1e9));
        }
        assert_eq!(size_label(d.sizes[0].1), "0.36 GB");
    }

    #[test]
    fn cost_panel_is_what_lesson_1_4_returns_and_prints() {
        let cfg = demo_config();
        let mut shown = Vec::new();
        let demo = economics::run(&cfg, false, &mut shown).unwrap();
        let shown = String::from_utf8(shown).unwrap();
        let d = gather();
        let (a, b) = (&d.inputs, &demo.inputs);
        assert_eq!(a.tokens_per_second, b.tokens_per_second);
        assert_eq!(a.tokens_per_second, 16.0);
        assert_eq!(a.hardware_usd, b.hardware_usd);
        assert_eq!(a.lifetime_months, b.lifetime_months);
        assert_eq!(a.watts, b.watts);
        assert_eq!(a.kwh_usd, b.kwh_usd);
        assert_eq!(a.hosted_usd_per_mtok, b.hosted_usd_per_mtok);
        let be = a.breakeven_mtok().unwrap();
        assert_eq!(Some(be), demo.breakeven);
        // The hero's break-even is the number the demo prints: 28.4.
        assert!(shown.contains(&format!("break-even     {be:>8.1} Mtok/month")));
        assert!(shown.contains(&format!(
            "measured       {:>8.1} tok/s",
            a.tokens_per_second
        )));
        assert_eq!(breakeven_label(be), "BREAK-EVEN 28.4 MTOK/MONTH");
        assert_eq!(
            speed_note(16.0),
            "USD PER MONTH VS MTOK PER MONTH, AT 16.0 TOK/S"
        );
    }

    #[test]
    fn elo_panel_is_what_lesson_2_5_returns() {
        let demo = elo::run(&mut Vec::new()).unwrap();
        assert_eq!(gather().ratings, demo.ratings);
    }

    #[test]
    fn red_team_panel_is_what_lesson_3_5_returns() {
        let demo = redteam::run(&demo_config(), false, &mut Vec::new()).unwrap();
        let d = gather();
        let rows = |v: &[CategoryRow]| -> Vec<(&'static str, u32, u32)> {
            v.iter().map(|r| (r.category, r.passes, r.trials)).collect()
        };
        assert_eq!(
            rows(&d.safe),
            rows(&redteam::by_category(&demo.model_sampled))
        );
        assert_eq!(
            rows(&d.vulnerable),
            rows(&redteam::by_category(&demo.vulnerable))
        );
        assert!(!d.safe.is_empty());
    }
}
