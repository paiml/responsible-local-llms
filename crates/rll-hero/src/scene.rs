//! The hero scene: four panels, each drawn from numbers this repo's own offline demos produce.
//! Quantization sizes (lesson 1.3), local vs hosted cost (1.4), Elo ratings (2.5), red-team
//! results (3.5). No number here is typed in by hand except the model size and the speed that
//! lesson 4 measures live.

use crate::canvas::*;
use ch1_sovereign::{economics::Inputs, quantize};
use ch2_evals::elo;
use ch3_redteam::redteam::{self, CategoryRow};
use rll_client::{Config, Runtime};

/// Parameters of the model whose file size panel 1 shows.
pub const PARAMS: f64 = 8e9;
/// Tokens per second that lesson 4 measures on the course machine (illustrative offline).
pub const TOKENS_PER_SECOND: f64 = 40.0;

pub struct Data {
    pub sizes: Vec<(&'static str, f64)>,
    pub inputs: Inputs,
    pub ratings: Vec<(String, f64)>,
    pub safe: Vec<CategoryRow>,
    pub vulnerable: Vec<CategoryRow>,
}

/// Run the offline demos and keep what the panels need.
pub fn gather() -> Data {
    let sizes = quantize::LEVELS
        .iter()
        .map(|&(name, bits)| (name, quantize::size_bytes(PARAMS, bits)))
        .collect();
    let inputs = Inputs {
        hardware_usd: 2400.0,
        lifetime_months: 36.0,
        watts: 250.0,
        kwh_usd: 0.15,
        hosted_usd_per_mtok: 3.0,
        tokens_per_second: TOKENS_PER_SECOND,
        rented_gpu_usd_per_hour: 2.0,
        busy_hours_per_day: 4.0,
    };
    let ratings = elo::run(&mut Vec::new())
        .expect("elo demo runs offline")
        .ratings;
    let cfg = Config {
        runtime: Runtime::Apr,
        base_url: Runtime::Apr.default_base_url().into(),
        model: "recorded".into(),
    };
    let rt = redteam::run(&cfg, false, &mut Vec::new()).expect("red-team demo runs offline");
    Data {
        sizes,
        inputs,
        ratings,
        safe: redteam::by_category(&rt.model_sampled),
        vulnerable: redteam::by_category(&rt.vulnerable),
    }
}

const PW: usize = 580;
const PH: usize = 250;

fn panel(c: &mut Canvas, x: usize, y: usize, title: &str, note: &str) {
    c.rect(x, y, PW, PH, PANEL);
    c.text(x + 16, y + 14, title, 2, TEXT);
    c.text(x + 16, y + 34, note, 1, MUTED);
}

fn sizes(c: &mut Canvas, x: usize, y: usize, d: &Data) {
    panel(
        c,
        x,
        y,
        "SIZE ON DISK",
        "8B MODEL, GGUF QUANTIZATION LEVELS",
    );
    let base = y + PH - 40;
    let top = d.sizes.iter().map(|s| s.1).fold(0.0, f64::max);
    c.rect(x + 20, base, PW - 40, 2, GRID);
    for (i, (name, bytes)) in d.sizes.iter().enumerate() {
        let h = (bytes / top * 140.0) as usize;
        let bx = x + 50 + i * 180;
        c.rect(bx, base - h, 120, h, [GREEN, BLUE, AMBER][i % 3]);
        c.text(
            bx,
            base - h - 22,
            &format!("{:.1} GB", bytes / 1e9),
            2,
            TEXT,
        );
        c.text(bx, base + 12, name, 2, MUTED);
    }
}

fn cost(c: &mut Canvas, x: usize, y: usize, d: &Data) {
    panel(
        c,
        x,
        y,
        "LOCAL VS HOSTED COST",
        "USD PER MONTH AGAINST MILLION TOKENS PER MONTH",
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
    c.text(
        mid + 8,
        y0 + 4,
        &format!("BREAK-EVEN {be:.0} MTOK/MONTH"),
        1,
        AMBER,
    );
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
