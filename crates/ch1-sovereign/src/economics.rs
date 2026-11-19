//! Lesson 4: local economics. Measure the hardware, then compare a fixed
//! monthly cost against a per-token price, and stop paying for idle GPUs.

use crate::client_for;
use rll_client::{ChatRequest, Config, Error};
use std::io::Write;

/// Inputs to the comparison. All prices are in US dollars.
pub struct Inputs {
    pub hardware_usd: f64,
    pub lifetime_months: f64,
    pub watts: f64,
    pub kwh_usd: f64,
    pub hosted_usd_per_mtok: f64,
    pub tokens_per_second: f64,
    pub rented_gpu_usd_per_hour: f64,
    pub busy_hours_per_day: f64,
}

impl Inputs {
    /// The monthly cost of owning the machine, whether or not it works.
    pub fn fixed_monthly_usd(&self) -> f64 {
        self.hardware_usd / self.lifetime_months
    }

    /// Electricity per million generated tokens, paid only while generating.
    pub fn energy_usd_per_mtok(&self) -> f64 {
        let seconds = 1e6 / self.tokens_per_second;
        self.watts * seconds / 3.6e6 * self.kwh_usd
    }

    pub fn local_cost(&self, mtok_per_month: f64) -> f64 {
        self.fixed_monthly_usd() + self.energy_usd_per_mtok() * mtok_per_month
    }

    pub fn hosted_cost(&self, mtok_per_month: f64) -> f64 {
        self.hosted_usd_per_mtok * mtok_per_month
    }

    /// Monthly volume (million tokens) above which local is cheaper, if it ever is.
    pub fn breakeven_mtok(&self) -> Option<f64> {
        let margin = self.hosted_usd_per_mtok - self.energy_usd_per_mtok();
        (margin > 0.0).then(|| self.fixed_monthly_usd() / margin)
    }

    /// What a rented GPU costs per month for the hours nobody uses it.
    pub fn idle_waste_usd(&self) -> f64 {
        self.rented_gpu_usd_per_hour * (24.0 - self.busy_hours_per_day) * 30.0
    }
}

pub struct Summary {
    pub inputs: Inputs,
    pub breakeven: Option<f64>,
}

impl Summary {
    /// At break-even the two costs match; below it hosted wins, above it local wins.
    /// With no break-even (slow hardware: energy per token costs more than the hosted
    /// price), hosted must be cheaper at every volume, small and large.
    pub fn contract_holds(&self) -> bool {
        let i = &self.inputs;
        match self.breakeven {
            None => [1.0, 1e6]
                .iter()
                .all(|&v| i.hosted_cost(v) < i.local_cost(v)),
            Some(m) => {
                (i.local_cost(m) - i.hosted_cost(m)).abs() < 1e-6
                    && i.hosted_cost(m * 0.5) < i.local_cost(m * 0.5)
                    && i.local_cost(m * 2.0) < i.hosted_cost(m * 2.0)
            }
        }
    }
}

pub fn run(cfg: &Config, live: bool, out: &mut impl Write) -> Result<Summary, Error> {
    let client = client_for(cfg.runtime, Some(&cfg.base_url), live);
    let c = client.chat(&ChatRequest::new(&cfg.model, "Say hello."))?;
    let inputs = Inputs {
        hardware_usd: 2400.0,
        lifetime_months: 36.0,
        watts: 250.0,
        kwh_usd: 0.15,
        hosted_usd_per_mtok: 3.0,
        tokens_per_second: c.tokens_per_second(),
        rented_gpu_usd_per_hour: 2.0,
        busy_hours_per_day: 4.0,
    };
    Ok(summarize(inputs, out))
}

/// Print the comparison and package it.
pub fn summarize(inputs: Inputs, out: &mut impl Write) -> Summary {
    let breakeven = inputs.breakeven_mtok();
    let _ = writeln!(
        out,
        "measured       {:>8.1} tok/s",
        inputs.tokens_per_second
    );
    let _ = writeln!(
        out,
        "fixed monthly  {:>8.2} USD",
        inputs.fixed_monthly_usd()
    );
    let _ = writeln!(
        out,
        "energy / Mtok  {:>8.4} USD",
        inputs.energy_usd_per_mtok()
    );
    let _ = writeln!(
        out,
        "idle GPU waste {:>8.2} USD/month",
        inputs.idle_waste_usd()
    );
    match breakeven {
        Some(m) => {
            let _ = writeln!(out, "break-even     {m:>8.1} Mtok/month");
        }
        None => {
            let _ = writeln!(out, "break-even     never");
        }
    }
    Summary { inputs, breakeven }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rll_client::Runtime;

    fn cfg() -> Config {
        Config {
            runtime: Runtime::Apr,
            base_url: Runtime::Apr.default_base_url().into(),
            model: "m".into(),
        }
    }

    fn inputs() -> Inputs {
        Inputs {
            hardware_usd: 360.0,
            lifetime_months: 36.0,
            watts: 360.0,
            kwh_usd: 0.1,
            hosted_usd_per_mtok: 2.0,
            tokens_per_second: 10.0,
            rented_gpu_usd_per_hour: 1.0,
            busy_hours_per_day: 24.0,
        }
    }

    #[test]
    fn fixed_cost_is_amortized() {
        assert!((inputs().fixed_monthly_usd() - 10.0).abs() < 1e-12);
    }

    #[test]
    fn energy_matches_hand_arithmetic() {
        // 1e6 tokens at 10 tok/s = 1e5 s at 360 W = 10 kWh = 1.0 USD at 0.10/kWh
        assert!((inputs().energy_usd_per_mtok() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn breakeven_equalizes_costs() {
        let i = inputs();
        let m = i.breakeven_mtok().unwrap();
        assert!((m - 10.0).abs() < 1e-9);
        assert!((i.local_cost(m) - i.hosted_cost(m)).abs() < 1e-9);
    }

    #[test]
    fn no_breakeven_when_energy_exceeds_the_hosted_price() {
        let mut i = inputs();
        i.hosted_usd_per_mtok = 0.5;
        assert_eq!(i.breakeven_mtok(), None);
    }

    #[test]
    fn idle_waste_is_zero_when_always_busy_and_grows_when_idle() {
        let mut i = inputs();
        assert_eq!(i.idle_waste_usd(), 0.0);
        i.busy_hours_per_day = 4.0;
        assert!((i.idle_waste_usd() - 600.0).abs() < 1e-9);
    }

    #[test]
    fn offline_run_satisfies_the_contract() {
        let mut buf = Vec::new();
        let s = run(&cfg(), false, &mut buf).unwrap();
        assert!(s.contract_holds());
        assert!(String::from_utf8(buf).unwrap().contains("break-even     "));
    }

    #[test]
    fn contract_fails_without_a_breakeven() {
        let s = Summary {
            inputs: inputs(),
            breakeven: None,
        };
        assert!(!s.contract_holds());
    }

    #[test]
    fn contract_holds_when_slow_hardware_never_breaks_even() {
        let mut i = inputs();
        i.tokens_per_second = 0.001;
        let s = summarize(i, &mut Vec::new());
        assert_eq!(s.breakeven, None);
        assert!(s.contract_holds());
    }

    #[test]
    fn summarize_reports_never_when_hosted_is_cheaper_per_token() {
        let mut i = inputs();
        i.tokens_per_second = 0.001;
        let mut buf = Vec::new();
        let s = summarize(i, &mut buf);
        assert_eq!(s.breakeven, None);
        assert!(String::from_utf8(buf).unwrap().contains("never"));
    }

    #[test]
    fn live_run_without_a_server_is_an_error() {
        let mut c = cfg();
        c.base_url = "http://127.0.0.1:1".into();
        let mut buf = Vec::new();
        assert!(run(&c, true, &mut buf).is_err());
    }
}
