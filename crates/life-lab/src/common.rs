//! Shared helpers for the analysis tools: argument parsing, information measures, statistics,
//! checkpoint access, and evaluation of agent copies on fixed streams.

use life_agent::Agent;
use life_interface::*;
use life_run::{checkpoint_path, load_checkpoint, Checkpoint, RunSummary};
use std::collections::HashMap;
use std::path::Path;

pub type Args = HashMap<String, String>;

pub fn parse_args() -> (String, Args) {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).cloned().unwrap_or_default();
    let mut map = HashMap::new();
    let mut i = 2;
    while i < args.len() {
        let k = args[i].trim_start_matches("--").to_string();
        if i + 1 < args.len() && !args[i + 1].starts_with("--") {
            map.insert(k, args[i + 1].clone());
            i += 2;
        } else {
            map.insert(k, "true".into());
            i += 1;
        }
    }
    (cmd, map)
}

pub fn get<T: std::str::FromStr>(m: &Args, k: &str, d: T) -> T {
    m.get(k).and_then(|v| v.parse().ok()).unwrap_or(d)
}

pub fn req(m: &Args, k: &str) -> String {
    m.get(k).cloned().unwrap_or_else(|| {
        eprintln!("missing --{k}");
        std::process::exit(2)
    })
}

/// KL(p || q) in bits with floors at 1e-7.
pub fn kl_bits(p: &[f32], q: &[f32]) -> f64 {
    let mut s = 0.0f64;
    for (a, b) in p.iter().zip(q) {
        let a = (*a as f64).max(1e-7);
        let b = (*b as f64).max(1e-7);
        s += a * (a / b).log2();
    }
    s.max(0.0)
}

pub fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        f64::NAN
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

pub fn median(v: &[f64]) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    let mut s: Vec<f64> = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = s.len();
    if n % 2 == 1 {
        s[n / 2]
    } else {
        0.5 * (s[n / 2 - 1] + s[n / 2])
    }
}

pub fn quantile(v: &[f64], q: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    let mut s: Vec<f64> = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pos = q * (s.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    s[lo] + (s[hi] - s[lo]) * (pos - lo as f64)
}

/// Bootstrap 95% CI of the median (1,000 resamples, fixed seed).
pub fn bootstrap_median_ci(v: &[f64], seed: u64) -> (f64, f64) {
    if v.len() < 2 {
        return (f64::NAN, f64::NAN);
    }
    let mut rng = Rng::new(seed);
    let mut meds = Vec::with_capacity(1000);
    for _ in 0..1000 {
        let sample: Vec<f64> = (0..v.len()).map(|_| v[rng.below(v.len() as u64) as usize]).collect();
        meds.push(median(&sample));
    }
    (quantile(&meds, 0.025), quantile(&meds, 0.975))
}

/// Least-squares slope of y against x.
pub fn slope(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    if x.len() < 2 {
        return f64::NAN;
    }
    let mx = mean(x);
    let my = mean(y);
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..x.len() {
        num += (x[i] - mx) * (y[i] - my);
        den += (x[i] - mx) * (x[i] - mx);
    }
    let _ = n;
    num / den
}

pub fn load_summary(dir: &Path) -> Option<RunSummary> {
    let s = std::fs::read_to_string(dir.join("summary.json")).ok()?;
    serde_json::from_str(&s).ok()
}

pub fn ckpt(dir: &Path, t: Tick) -> Checkpoint {
    load_checkpoint(&checkpoint_path(dir, t)).unwrap_or_else(|e| {
        eprintln!("cannot load checkpoint {} in {}: {e}", t, dir.display());
        std::process::exit(1)
    })
}

/// One evaluated prediction of a copy on a fixed stream.
#[derive(Clone, Debug)]
pub struct EvalEvent {
    pub tick: Tick,
    pub block: u8,
    pub value: u8,
    pub probs: Vec<f32>,
    pub surprise_bits: f64,
}

/// Run a copy of `agent` over `stream` with all weight change disabled. With `reset`, the fast
/// state, traces, held inputs and stored predictions are zeroed first (analysis copies only).
pub fn run_copy(agent: &Agent, stream: &[Observation], reset: bool) -> Vec<EvalEvent> {
    let mut a = agent.clone();
    if reset {
        a.reset_state();
    }
    a.set_learning(false, false, false);
    let mut out = Vec::new();
    for o in stream {
        let rep = a.tick(o);
        for e in rep.evaluations {
            out.push(EvalEvent { tick: o.tick, block: e.block, value: e.value, probs: e.probs, surprise_bits: e.surprise_bits as f64 });
        }
    }
    out
}

/// The position in effect at tick `t` given (tick, position) entries sorted by tick.
pub fn site_at(positions: &[(Tick, u8)], t: Tick) -> u8 {
    let mut p = positions[0].1;
    for &(pt, pos) in positions {
        if pt <= t {
            p = pos;
        } else {
            break;
        }
    }
    p
}

pub fn fmt(v: f64) -> String {
    if v.is_nan() {
        "nan".into()
    } else {
        format!("{v:.4}")
    }
}
