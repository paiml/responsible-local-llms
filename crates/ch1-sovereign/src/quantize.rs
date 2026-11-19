//! Lesson 3: size and quantize for your machine. File size follows bits per
//! weight; speed is measured, never assumed.

use crate::client_for;
use rll_client::{ChatRequest, Config, Error};
use std::io::Write;

/// Bits per weight of the GGUF quantization types the course compares.
pub const LEVELS: [(&str, f64); 3] = [("q4_k_m", 4.85), ("q8_0", 8.5), ("f16", 16.0)];

/// Parameters of the model lesson 1.3 sizes (0.6B). The README hero reads it too.
pub const PARAMS: f64 = 0.6e9;

/// Model size in bytes for a parameter count at a bits-per-weight level.
pub fn size_bytes(params: f64, bits_per_weight: f64) -> f64 {
    params * bits_per_weight / 8.0
}

/// The size text lesson 1.3 prints for a byte count (`0.34 GiB`). The README hero uses it too.
pub fn gib_label(bytes: f64) -> String {
    format!("{:.2} GiB", bytes / (1u64 << 30) as f64)
}

pub struct Summary {
    pub sizes: Vec<(&'static str, f64)>,
    pub tokens_per_second: f64,
}

impl Summary {
    /// Smaller quantization means a smaller file, and the measured speed is a real number.
    pub fn contract_holds(&self) -> bool {
        let sorted = self.sizes.windows(2).all(|w| w[0].1 < w[1].1);
        sorted && self.tokens_per_second.is_finite() && self.tokens_per_second > 0.0
    }
}

pub fn run(cfg: &Config, live: bool, out: &mut impl Write) -> Result<Summary, Error> {
    let mut sizes = Vec::new();
    for (name, bpw) in LEVELS {
        let bytes = size_bytes(PARAMS, bpw);
        let _ = writeln!(out, "{name:<7} {:>11}", gib_label(bytes));
        sizes.push((name, bytes));
    }
    let client = client_for(cfg.runtime, Some(&cfg.base_url), live);
    let c = client.chat(&ChatRequest::new(&cfg.model, "Count from one to ten."))?;
    let tokens_per_second = c.tokens_per_second();
    let _ = writeln!(
        out,
        "{} measured {:.1} tok/s",
        cfg.runtime.name(),
        tokens_per_second
    );
    Ok(Summary {
        sizes,
        tokens_per_second,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rll_client::Runtime;

    fn cfg(rt: Runtime) -> Config {
        Config {
            runtime: rt,
            base_url: rt.default_base_url().into(),
            model: "m".into(),
        }
    }

    #[test]
    fn size_is_linear_in_bits() {
        assert!((size_bytes(8.0, 8.0) - 8.0).abs() < 1e-12);
        assert!((size_bytes(1e9, 4.0) - 5e8).abs() < 1.0);
    }

    #[test]
    fn offline_run_satisfies_the_contract() {
        let mut buf = Vec::new();
        let s = run(&cfg(Runtime::Apr), false, &mut buf).unwrap();
        assert!(s.contract_holds());
        // apr fixture: 8 tokens in 0.5 s
        assert!((s.tokens_per_second - 16.0).abs() < 1e-9);
        assert!(String::from_utf8(buf).unwrap().contains("q4_k_m"));
    }

    #[test]
    fn contract_rejects_unsorted_sizes_and_bad_speed() {
        let bad_order = Summary {
            sizes: vec![("a", 2.0), ("b", 1.0)],
            tokens_per_second: 5.0,
        };
        assert!(!bad_order.contract_holds());
        let zero = Summary {
            sizes: vec![("a", 1.0), ("b", 2.0)],
            tokens_per_second: 0.0,
        };
        assert!(!zero.contract_holds());
        let nan = Summary {
            sizes: vec![("a", 1.0), ("b", 2.0)],
            tokens_per_second: f64::NAN,
        };
        assert!(!nan.contract_holds());
    }

    #[test]
    fn live_run_without_a_server_is_an_error() {
        let mut c = cfg(Runtime::Apr);
        c.base_url = "http://127.0.0.1:1".into();
        let mut buf = Vec::new();
        assert!(run(&c, true, &mut buf).is_err());
    }
}
