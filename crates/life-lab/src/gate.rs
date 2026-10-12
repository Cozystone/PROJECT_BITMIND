//! Aggregation over formal seeds and the pre-registered gate (docs/APPENDIX-FIRST-EXPERIMENT.md
//! A8). Written and committed before any formal result exists. Thresholds are constants here and
//! are not read from the command line.

use crate::common::*;
use crate::probes::SeedEval;
use serde::Serialize;
use std::path::PathBuf;

pub const P1_MIN_EFFECT: f64 = 0.05;
pub const P1_MIN_SEEDS: usize = 9;
pub const P2_MIN_EFFECT: f64 = 0.10;
pub const P2_MIN_SEEDS: usize = 9;
pub const P3_MIN_EFFECT: f64 = 0.05;
pub const P3_MIN_SEEDS: usize = 8;
pub const HIDDEN_MIN_EFFECT: f64 = 0.05;
pub const HIDDEN_MIN_SEEDS: usize = 8;
pub const MAX_INVALID: usize = 2;

#[derive(Serialize, Debug, Clone)]
pub struct Stat {
    pub name: String,
    pub per_seed: Vec<(u64, f64)>,
    pub median: f64,
    pub iqr: (f64, f64),
    pub ci95_median: (f64, f64),
    pub n_positive: usize,
    pub n_at_least_min_effect: usize,
    pub n_seeds: usize,
    pub pass: bool,
    pub rule: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct GateResult {
    pub seeds_requested: Vec<u64>,
    pub seeds_valid: Vec<u64>,
    pub seeds_invalid: Vec<(u64, String)>,
    pub unstable: bool,
    pub p1: Stat,
    pub p2: Stat,
    pub p3: Stat,
    pub hidden_plasticity: Stat,
    pub p3_insufficient_seeds: Vec<u64>,
    pub verdict: String,
    pub hidden_verdict: String,
    pub side_reports: Vec<String>,
}

const SENTINEL: f64 = -1e300;

fn null_to_sentinel(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Null => *v = serde_json::Value::from(SENTINEL),
        serde_json::Value::Array(a) => a.iter_mut().for_each(null_to_sentinel),
        serde_json::Value::Object(o) => o.iter_mut().for_each(|(k, v)| {
            // genuine Option fields stay null
            if !(k == "order_full" || k == "order_birth") {
                null_to_sentinel(v)
            }
        }),
        _ => {}
    }
}

fn unsentinel(x: &mut f64) {
    if *x <= SENTINEL {
        *x = f64::NAN;
    }
}

/// serde_json writes NaN as null; read it back as NaN.
fn read_eval(p: &std::path::Path) -> Result<SeedEval, String> {
    let t = std::fs::read_to_string(p).map_err(|e| e.to_string())?;
    let mut v: serde_json::Value = serde_json::from_str(&t).map_err(|e| e.to_string())?;
    null_to_sentinel(&mut v);
    let mut e: SeedEval = serde_json::from_value(v).map_err(|e| e.to_string())?;
    for m in [&mut e.p1_d_hist, &mut e.p3_s, &mut e.p3_s_at_t1, &mut e.p3_s_free_unchanged, &mut e.p3_s_forced] {
        m.values_mut().for_each(unsentinel);
    }
    for m in [&mut e.p2_s, &mut e.p2_s_free] {
        m.values_mut().for_each(|v| v.iter_mut().for_each(unsentinel));
    }
    e.p1_blocks.values_mut().for_each(|a| a.iter_mut().for_each(unsentinel));
    for v in [&mut e.p2_floor, &mut e.p2_truth_ref, &mut e.p2_truth_ref_free] {
        v.iter_mut().for_each(unsentinel);
    }
    for o in e.order.values_mut() {
        unsentinel(&mut o.d_order);
        unsentinel(&mut o.kl_truth_abc);
        unsentinel(&mut o.kl_truth_acb);
    }
    for va in e.variants.values_mut() {
        unsentinel(&mut va.d_hist);
        va.s_full.iter_mut().for_each(unsentinel);
        va.s_birth.iter_mut().for_each(unsentinel);
        for o in [&mut va.order_full, &mut va.order_birth] {
            if let Some(o) = o {
                unsentinel(&mut o.d_order);
                unsentinel(&mut o.kl_truth_abc);
                unsentinel(&mut o.kl_truth_acb);
            }
        }
    }
    Ok(e)
}

fn stat(name: &str, vals: &[(u64, f64)], min_effect: f64, min_seeds: usize, rule_median: bool) -> Stat {
    let v: Vec<f64> = vals.iter().map(|x| x.1).filter(|x| x.is_finite()).collect();
    let med = median(&v);
    let n_pos = v.iter().filter(|x| **x > 0.0).count();
    let n_eff = v.iter().filter(|x| **x >= min_effect).count();
    let pass = if rule_median { med >= min_effect && n_pos >= min_seeds } else { n_eff >= min_seeds };
    let rule = if rule_median { format!("median >= {min_effect} bits and > 0 in >= {min_seeds} seeds") } else { format!(">= {min_effect} bits in >= {min_seeds} seeds") };
    Stat { name: name.into(), per_seed: vals.to_vec(), median: med, iqr: (quantile(&v, 0.25), quantile(&v, 0.75)), ci95_median: bootstrap_median_ci(&v, 0xB00757), n_positive: n_pos, n_at_least_min_effect: n_eff, n_seeds: v.len(), pass, rule }
}

pub fn run(m: &Args) {
    let root = PathBuf::from(req(m, "root"));
    let spec = m.get("seeds").cloned().unwrap_or_else(|| "101-110".into());
    let seeds: Vec<u64> = if let Some((a, b)) = spec.split_once('-') {
        (a.parse::<u64>().unwrap()..=b.parse::<u64>().unwrap()).collect()
    } else {
        spec.split(',').map(|s| s.parse().unwrap()).collect()
    };
    let mut valid = Vec::new();
    let mut invalid = Vec::new();
    let mut evals: Vec<SeedEval> = Vec::new();
    for &s in &seeds {
        let p = root.join(format!("seed{s}")).join("evaluate.json");
        let parsed = read_eval(&p);
        match parsed {
            Err(msg) => invalid.push((s, format!("evaluate.json unreadable: {msg}"))),
            Ok(e) => {
                let bad: Vec<String> = e.arms_status.iter().filter(|(a, st)| ["full", "frozen", "readout"].contains(&a.as_str()) && *st != "ok").map(|(a, st)| format!("{a}:{st}")).collect();
                if !bad.is_empty() {
                    invalid.push((s, bad.join(",")));
                } else if !e.streams_identical {
                    invalid.push((s, "streams not identical across arms".into()));
                } else {
                    valid.push(s);
                    evals.push(e);
                }
            }
        }
    }
    let unstable = invalid.iter().filter(|(_, r)| r.contains("numerically_invalid")).count() > MAX_INVALID;
    let g = |e: &SeedEval, a: &str| e.p1_d_hist.get(a).cloned().unwrap_or(f64::NAN);
    let p1v: Vec<(u64, f64)> = evals.iter().map(|e| (e.seed, g(e, "full") - g(e, "frozen"))).collect();
    let s2 = |e: &SeedEval, a: &str| e.p2_s.get(a).map(|v| mean(v)).unwrap_or(f64::NAN);
    let p2v: Vec<(u64, f64)> = evals.iter().map(|e| (e.seed, s2(e, "frozen") - s2(e, "full"))).collect();
    let mut p3_insufficient = Vec::new();
    let p3v: Vec<(u64, f64)> = evals
        .iter()
        .map(|e| {
            if !e.p3_sufficient {
                p3_insufficient.push(e.seed);
                (e.seed, f64::NAN)
            } else {
                (e.seed, e.p3_s.get("frozen").cloned().unwrap_or(f64::NAN) - e.p3_s.get("full").cloned().unwrap_or(f64::NAN))
            }
        })
        .collect();
    let hv: Vec<(u64, f64)> = evals.iter().map(|e| (e.seed, s2(e, "readout") - s2(e, "full"))).collect();
    let p1 = stat("P1 change: D_hist(full) - D_hist(frozen)", &p1v, P1_MIN_EFFECT, P1_MIN_SEEDS, true);
    let p2 = stat("P2 usefulness: S_frozen - S_full (block 0, mean over 5 checkpoints)", &p2v, P2_MIN_EFFECT, P2_MIN_SEEDS, true);
    let p3 = stat("P3 persistence: S_frozen - S_full(1e6) on E_2e5, unchanged sites", &p3v, P3_MIN_EFFECT, P3_MIN_SEEDS, false);
    let hid = stat("hidden plasticity: S_readout - S_full (P2 streams)", &hv, HIDDEN_MIN_EFFECT, HIDDEN_MIN_SEEDS, false);
    let passes = [p1.pass, p2.pass, p3.pass].iter().filter(|p| **p).count();
    let verdict = if unstable {
        "unstable, not supported".to_string()
    } else if passes == 3 {
        "supported".to_string()
    } else if passes > 0 {
        let names: Vec<&str> = [("P1", p1.pass), ("P2", p2.pass), ("P3", p3.pass)].iter().filter(|x| x.1).map(|x| x.0).collect();
        format!("partial ({} pass)", names.join(","))
    } else {
        "not supported".to_string()
    };
    let hidden_verdict = if hid.pass { "shown" } else { "not shown" }.to_string();
    let mut side = Vec::new();
    for a in ["readout", "homeo", "freeze_at", "fb_wt", "ltc"] {
        let v: Vec<f64> = evals.iter().filter_map(|e| e.p1_d_hist.get(a).cloned()).collect();
        if !v.is_empty() {
            side.push(format!("D_hist {a}: median {} (n {})", fmt(median(&v)), v.len()));
        }
        let v: Vec<f64> = evals.iter().filter_map(|e| e.p2_s.get(a).map(|x| mean(x))).collect();
        if !v.is_empty() {
            side.push(format!("S_here {a} (P2 streams): median {} (n {})", fmt(median(&v)), v.len()));
        }
    }
    // retention: how much of the 2e5 advantage over the birth network survives to 1e6 (unchanged sites)
    let ret: Vec<f64> = evals
        .iter()
        .filter(|e| e.p3_sufficient)
        .filter_map(|e| {
            let fr = e.p3_s.get("frozen")?;
            let f2 = e.p3_s.get("full")?;
            let f1 = e.p3_s_at_t1.get("full")?;
            let gain1 = fr - f1;
            if gain1.abs() < 1e-9 {
                None
            } else {
                Some((fr - f2) / gain1)
            }
        })
        .collect();
    side.push(format!("retention (S_frozen - S_full@1e6)/(S_frozen - S_full@2e5) on E_2e5 unchanged sites: median {} (n {})", fmt(median(&ret)), ret.len()));
    let free: Vec<f64> = evals.iter().filter_map(|e| Some(mean(e.p2_s_free.get("frozen")?) - mean(e.p2_s_free.get("full")?))).collect();
    side.push(format!("S_frozen - S_full on free (non-forced) block-0 events only: median {} (n {})", fmt(median(&free)), free.len()));
    let fl: Vec<f64> = evals.iter().map(|e| mean(&e.p2_floor)).collect();
    let tr: Vec<f64> = evals.iter().map(|e| mean(&e.p2_truth_ref)).collect();
    side.push(format!("marginal floor (P2 streams): median {}", fmt(median(&fl))));
    side.push(format!("grammar-conditional reference (P2 streams): median {}", fmt(median(&tr))));
    // structure-specific gain: the surprise reduction relative to the birth network in the
    // structured world minus the same quantity in each control world (same seed)
    for a in ["full_shuffled", "full_iid", "full_perm"] {
        let v: Vec<f64> = evals
            .iter()
            .filter_map(|e| {
                let va = e.variants.get(a)?;
                let structured = s2(e, "frozen") - s2(e, "full");
                Some(structured - (mean(&va.s_birth) - mean(&va.s_full)))
            })
            .collect();
        if !v.is_empty() {
            side.push(format!("structure-specific gain vs {a}: (S_birth - S_full)_structured - (S_birth - S_full)_{a} median {} IQR ({}, {}) (n {})", fmt(median(&v)), fmt(quantile(&v, 0.25)), fmt(quantile(&v, 0.75)), v.len()));
        }
    }
    for a in ["full_shuffled", "full_iid", "full_perm"] {
        let v: Vec<f64> = evals.iter().filter_map(|e| e.variants.get(a).map(|x| x.d_hist)).collect();
        let u: Vec<f64> = evals.iter().filter_map(|e| e.variants.get(a).map(|x| mean(&x.s_birth) - mean(&x.s_full))).collect();
        if !v.is_empty() {
            side.push(format!("{a}: D_hist median {}; S_birth - S_full median {}", fmt(median(&v)), fmt(median(&u))));
        }
    }
    let res = GateResult { seeds_requested: seeds, seeds_valid: valid, seeds_invalid: invalid, unstable, p1, p2, p3, hidden_plasticity: hid, p3_insufficient_seeds: p3_insufficient, verdict, hidden_verdict, side_reports: side };
    for s in [&res.p1, &res.p2, &res.p3, &res.hidden_plasticity] {
        println!("{}\n   rule: {}\n   per seed: {}\n   median={} IQR=({}, {}) CI95(median)=({}, {}) positive={}/{} >=min_effect={}/{} -> {}", s.name, s.rule, s.per_seed.iter().map(|(a, b)| format!("{a}:{}", fmt(*b))).collect::<Vec<_>>().join(" "), fmt(s.median), fmt(s.iqr.0), fmt(s.iqr.1), fmt(s.ci95_median.0), fmt(s.ci95_median.1), s.n_positive, s.n_seeds, s.n_at_least_min_effect, s.n_seeds, if s.pass { "PASS" } else { "FAIL" });
    }
    println!("valid seeds {:?}; invalid {:?}; P3 insufficient {:?}", res.seeds_valid, res.seeds_invalid, res.p3_insufficient_seeds);
    for s in &res.side_reports {
        println!("side: {s}");
    }
    println!("VERDICT: {}   hidden plasticity benefit: {}", res.verdict, res.hidden_verdict);
    let out = root.join("gate.json");
    std::fs::write(&out, serde_json::to_string_pretty(&res).unwrap()).expect("write");
    println!("written {}", out.display());
}
