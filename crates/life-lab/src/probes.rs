//! Probes (docs/APPENDIX-FIRST-EXPERIMENT.md A5, A6) and the per-seed `evaluate` driver (A8).
//!
//! All probes run on analysis copies with weight change disabled. Every copy sees a stream that
//! was generated once from a checkpointed world with its own seeds (never a main life's future).
//! Specification decisions fixed here before any formal run:
//! * every probe starts with a state reset and a 200-tick warm-up that is not scored;
//! * the history probe scores the next 1,000 ticks (A5); usefulness streams score 5,000 ticks so
//!   that per-site subsets (P3) have enough events;
//! * KL and surprise are in bits; "insufficient" = fewer than 30 scored events.

use crate::common::*;
use crate::traces::change_ticks;
use life_agent::Agent;
use life_interface::*;
use life_run::{make_probe_stream, ProbeStream};
use life_trace::{read_each, Rec};
use life_world::World;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const WARMUP: u64 = 200;
pub const HIST_TICKS: u64 = 1000;
pub const EVAL_TICKS: u64 = 5000;
pub const MIN_EVENTS: u64 = 30;
pub const P2_CHECKPOINTS: [u64; 5] = [200_000, 400_000, 600_000, 800_000, 1_000_000];
pub const P1_T1: u64 = 200_000;
pub const P1_T2: u64 = 1_000_000;

pub fn probe_seed(world_seed: u64, t: Tick, tag: u64) -> u64 {
    hash_mix(hash_mix(hash_mix(HASH_SEED, world_seed ^ 0x9E37), t), tag)
}

fn obs(t: Tick, events: &[(u8, u8)]) -> Observation {
    Observation { tick: t, events: events.iter().map(|&(b, v)| ObsEvent { block: b, value: v }).collect() }
}

// ---------------------------------------------------------------- history dependence (D_hist)

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Hist {
    pub overall: f64,
    pub per_block: [f64; NUM_BLOCKS],
    pub n: [u64; NUM_BLOCKS],
    pub n_total: u64,
}

/// D_hist = mean KL(p̂₂ ‖ p̂₁) over the scored events of a shared stream, both copies reset.
pub fn d_hist(a1: &Agent, a2: &Agent, stream: &[Observation], warm_end: Tick) -> Hist {
    let e1 = run_copy(a1, stream, true);
    let e2 = run_copy(a2, stream, true);
    assert_eq!(e1.len(), e2.len(), "copies evaluated different event sets");
    let mut per = [0.0f64; NUM_BLOCKS];
    let mut n = [0u64; NUM_BLOCKS];
    for (x, y) in e1.iter().zip(&e2) {
        assert_eq!((x.tick, x.block, x.value), (y.tick, y.block, y.value));
        if x.tick <= warm_end {
            continue;
        }
        per[x.block as usize] += kl_bits(&y.probs, &x.probs);
        n[x.block as usize] += 1;
    }
    let total: u64 = n.iter().sum();
    let overall = if total > 0 { per.iter().sum::<f64>() / total as f64 } else { f64::NAN };
    for k in 0..NUM_BLOCKS {
        per[k] = if n[k] > 0 { per[k] / n[k] as f64 } else { f64::NAN };
    }
    Hist { overall, per_block: per, n, n_total: total }
}

// ---------------------------------------------------------------- usefulness (surprise on E_t)

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Surprise {
    pub mean: f64,
    pub n: u64,
}

/// `ok(tick, site)` selects the scored events (site = position in effect at that tick).
pub fn surprise_on(agent: &Agent, ps: &ProbeStream, warm_end: Tick, block: u8, ok: &dyn Fn(Tick, u8) -> bool) -> Surprise {
    let evs = run_copy(agent, &ps.obs, true);
    let mut s = 0.0;
    let mut n = 0u64;
    for e in evs {
        if e.tick <= warm_end || e.block != block {
            continue;
        }
        if !ok(e.tick, site_at(&ps.positions, e.tick)) {
            continue;
        }
        s += e.surprise_bits;
        n += 1;
    }
    Surprise { mean: if n > 0 { s / n as f64 } else { f64::NAN }, n }
}

/// The set of (tick, site) pairs whose emission was a forced-cycle symbol (after `touch`).
pub fn forced_set(ps: &ProbeStream) -> std::collections::HashSet<(Tick, u8)> {
    ps.emissions.iter().filter(|e| e.4).map(|e| (e.0, e.1)).collect()
}

/// Analyzer reference: surprise of the world's own generative conditional on the block-0 events of
/// a stream: the grammar row (mode-aware, cross-site influence ignored) for free emissions, the
/// cycle symbol for forced emissions, both passed through the misread model
/// p(shown = v) = (1 - eps) d[v] + eps (1 - d[v]) / (A - 1).
pub fn truth_reference(world: &World, ps: &ProbeStream, warm_end: Tick, ok: &dyn Fn(Tick, u8) -> bool) -> Surprise {
    let mut hist: Vec<(u8, u8)> = (0..life_world::K).map(|s| world.last2(s)).collect();
    let mut dist_at: BTreeMap<(Tick, u8), [f32; 6]> = BTreeMap::new();
    let mut ems = ps.emissions.clone();
    ems.sort();
    for (t, site, sym, mode, forced) in ems {
        let (a, b) = hist[site as usize];
        let d = if forced {
            let mut d = [0f32; 6];
            d[sym as usize] = 1.0;
            d
        } else {
            world.true_next_dist_mode(site as usize, a, b, mode)
        };
        dist_at.insert((t, site), d);
        hist[site as usize] = (b, sym);
    }
    let eps = world.config().noise_here as f64;
    let mut s = 0.0;
    let mut n = 0u64;
    for o in &ps.obs {
        if o.tick <= warm_end {
            continue;
        }
        let site = site_at(&ps.positions, o.tick);
        if !ok(o.tick, site) {
            continue;
        }
        for ev in &o.events {
            if ev.block != 0 {
                continue;
            }
            if let Some(d) = dist_at.get(&(o.tick, site)) {
                let v = world.unpermute(0, ev.value) as usize;
                let dv = d[v] as f64;
                let p = (1.0 - eps) * dv + eps * (1.0 - dv) / 5.0;
                s += -p.max(1e-7).log2();
                n += 1;
            }
        }
    }
    Surprise { mean: if n > 0 { s / n as f64 } else { f64::NAN }, n }
}

/// Per-site Laplace-smoothed marginal of block-0 values, estimated from the main life's own
/// observations up to each tick in `ts` (position tracked from block-3 events). Candidate A's
/// floor (A9).
pub fn marginal_tables(trace: &Path, world: &World, ts: &[Tick]) -> Vec<[[f64; 6]; 8]> {
    let mut counts = [[1.0f64; 6]; 8];
    let mut pos = 0u8;
    let mut out: Vec<[[f64; 6]; 8]> = Vec::new();
    let mut next = 0usize;
    read_each(trace, |r| {
        if let Rec::Obs { tick, events } = r {
            while next < ts.len() && tick > ts[next] {
                out.push(counts);
                next += 1;
            }
            for &(b, v) in &events {
                if b == 3 {
                    pos = world.unpermute(3, v);
                }
            }
            for &(b, v) in &events {
                if b == 0 {
                    counts[pos as usize][v as usize] += 1.0;
                }
            }
        }
    })
    .expect("read trace");
    while out.len() < ts.len() {
        out.push(counts);
    }
    for t in out.iter_mut() {
        for row in t.iter_mut() {
            let z: f64 = row.iter().sum();
            for v in row.iter_mut() {
                *v /= z;
            }
        }
    }
    out
}

pub fn floor_surprise(table: &[[f64; 6]; 8], ps: &ProbeStream, warm_end: Tick, ok: &dyn Fn(Tick, u8) -> bool) -> Surprise {
    let mut s = 0.0;
    let mut n = 0u64;
    for o in &ps.obs {
        if o.tick <= warm_end {
            continue;
        }
        let site = site_at(&ps.positions, o.tick);
        if !ok(o.tick, site) {
            continue;
        }
        for ev in &o.events {
            if ev.block == 0 {
                s += -table[site as usize][ev.value as usize].log2();
                n += 1;
            }
        }
    }
    Surprise { mean: if n > 0 { s / n as f64 } else { f64::NAN }, n }
}

// ---------------------------------------------------------------- order probe (A6)

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct OrderResult {
    /// mean symmetric KL between the predictions after A,B,C and after A,C,B
    pub d_order: f64,
    pub kl_truth_abc: f64,
    pub kl_truth_acb: f64,
    pub n: u64,
}

fn feed_sequence(base: &Agent, site: u8, t0: Tick, seq: [u8; 3]) -> Vec<f32> {
    let mut a = base.clone();
    for dt in 1..=26u64 {
        let t = t0 + dt;
        let o = match dt {
            1 => obs(t, &[(3, site)]),
            6 => obs(t, &[(0, seq[0])]),
            16 => obs(t, &[(0, seq[1])]),
            26 => obs(t, &[(0, seq[2])]),
            _ => obs(t, &[]),
        };
        a.tick(&o);
    }
    a.predictions()[0].clone()
}

/// After a shared warm-up, present A,B,C and A,C,B (10 ticks apart, position announced first) at
/// every site and compare the block-0 predictions with each other and with the grammar's truth.
pub fn order_probe(agent: &Agent, warm: &[Observation], world: &World, seed: u64, triples_per_site: usize) -> OrderResult {
    let mut base = agent.clone();
    base.reset_state();
    base.set_learning(false, false, false);
    for o in warm {
        base.tick(o);
    }
    let t0 = warm.last().map(|o| o.tick).unwrap_or(0);
    let mut rng = Rng::new(seed);
    let (mut d, mut ka, mut kb, mut n) = (0.0, 0.0, 0.0, 0u64);
    for s in 0..life_world::K as u8 {
        for _ in 0..triples_per_site {
            let a = rng.below(6) as u8;
            let b = rng.below(6) as u8;
            let mut c = rng.below(6) as u8;
            while c == b {
                c = rng.below(6) as u8;
            }
            let p_abc = feed_sequence(&base, s, t0, [a, b, c]);
            let p_acb = feed_sequence(&base, s, t0, [a, c, b]);
            d += 0.5 * (kl_bits(&p_abc, &p_acb) + kl_bits(&p_acb, &p_abc));
            ka += kl_bits(&world.true_next_dist(s as usize, b, c), &p_abc);
            kb += kl_bits(&world.true_next_dist(s as usize, c, b), &p_acb);
            n += 1;
        }
    }
    let nf = n as f64;
    OrderResult { d_order: d / nf, kl_truth_abc: ka / nf, kl_truth_acb: kb / nf, n }
}

// ---------------------------------------------------------------- re-adaptation (A6)

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Readapt {
    pub label: String,
    pub t_c: u64,
    pub sites: Vec<u8>,
    pub window_means: Vec<f64>,
    pub window_counts: Vec<u64>,
    pub mean_30k_50k: f64,
    pub slope_last_20k_per_10k: f64,
    pub converged: bool,
    pub n_events: u64,
}

/// Block-0 surprise at `sites` from a trace, in `win`-tick windows over `horizon` ticks after t_c.
/// `pos0` is the position in effect at t_c (needed for branch traces that start there).
pub fn readapt_from_trace(label: &str, trace: &Path, t_c: Tick, horizon: u64, win: u64, sites: &[u8], pos0: Option<u8>) -> Readapt {
    let nwin = (horizon / win) as usize;
    let mut sums = vec![0.0f64; nwin];
    let mut cnts = vec![0u64; nwin];
    let mut pos: Option<u8> = pos0;
    read_each(trace, |r| match r {
        Rec::Obs { tick, events } => {
            for (b, v) in events {
                if b == 3 {
                    pos = Some(v);
                }
            }
            let _ = tick;
        }
        Rec::Eval { tick, block, surprise, .. } => {
            if block == 0 && tick > t_c && tick <= t_c + horizon {
                if let Some(p) = pos {
                    if sites.contains(&p) {
                        let i = ((tick - t_c - 1) / win) as usize;
                        sums[i] += surprise as f64;
                        cnts[i] += 1;
                    }
                }
            }
        }
        _ => {}
    })
    .expect("read trace");
    let means: Vec<f64> = (0..nwin).map(|i| if cnts[i] > 0 { sums[i] / cnts[i] as f64 } else { f64::NAN }).collect();
    let from = (30_000 / win) as usize;
    let tail: Vec<f64> = means[from.min(nwin)..].iter().cloned().filter(|v| !v.is_nan()).collect();
    let mean_tail = mean(&tail);
    let last_from = nwin.saturating_sub((20_000 / win) as usize);
    let xs: Vec<f64> = (last_from..nwin).map(|i| (i as f64 + 0.5) * win as f64 / 10_000.0).collect();
    let ys: Vec<f64> = (last_from..nwin).map(|i| means[i]).collect();
    let valid: Vec<(f64, f64)> = xs.iter().cloned().zip(ys.iter().cloned()).filter(|(_, y)| !y.is_nan()).collect();
    let sl = if valid.len() >= 2 { slope(&valid.iter().map(|v| v.0).collect::<Vec<_>>(), &valid.iter().map(|v| v.1).collect::<Vec<_>>()) } else { f64::NAN };
    Readapt { label: label.into(), t_c, sites: sites.to_vec(), window_means: means, window_counts: cnts.clone(), mean_30k_50k: mean_tail, slope_last_20k_per_10k: sl, converged: sl.is_finite() && sl.abs() < 0.01, n_events: cnts.iter().sum() }
}

/// Position in effect at `t` in a main-run trace (block-3 events; structured worlds).
pub fn position_at(trace: &Path, t: Tick) -> u8 {
    let mut pos = 0u8;
    read_each(trace, |r| {
        if let Rec::Obs { tick, events } = r {
            if tick <= t {
                for (b, v) in events {
                    if b == 3 {
                        pos = v;
                    }
                }
            }
        }
    })
    .expect("read trace");
    pos
}

// ---------------------------------------------------------------- commands

pub fn probe_history_cmd(m: &Args) {
    let dir = PathBuf::from(req(m, "run"));
    let t1: u64 = get(m, "a", P1_T1);
    let t2: u64 = get(m, "b", P1_T2);
    let c1 = ckpt(&dir, t1);
    let c2 = ckpt(&dir, t2);
    let seed = probe_seed(c1.life.world.config().seed_world, t1, 1);
    let ps = make_probe_stream(&c1.life.world, seed, WARMUP + HIST_TICKS);
    let h = d_hist(&c1.life.agent, &c2.life.agent, &ps.obs, t1 + WARMUP);
    println!("D_hist({t1},{t2}) overall={} bits over {} events; per block {:?} (n {:?})", fmt(h.overall), h.n_total, h.per_block.iter().map(|v| fmt(*v)).collect::<Vec<_>>(), h.n);
}

pub fn usefulness_cmd(m: &Args) {
    let dirs: Vec<PathBuf> = req(m, "runs").split(',').map(PathBuf::from).collect();
    let t: u64 = get(m, "at", 200_000);
    let base = ckpt(&dirs[0], t);
    let seed = probe_seed(base.life.world.config().seed_world, t, 2);
    let ps = make_probe_stream(&base.life.world, seed, WARMUP + EVAL_TICKS);
    let all = |_t: Tick, _s: u8| true;
    let forced = forced_set(&ps);
    let free = |t: Tick, s: u8| !forced.contains(&(t, s));
    let tr = truth_reference(&base.life.world, &ps, t + WARMUP, &all);
    let trf = truth_reference(&base.life.world, &ps, t + WARMUP, &free);
    println!("E_{t}: {} scored block-0 events ({} free of forced cycles); generative reference {} bits (free: {})", tr.n, trf.n, fmt(tr.mean), fmt(trf.mean));
    for d in &dirs {
        let c = ckpt(d, t);
        let s = surprise_on(&c.life.agent, &ps, t + WARMUP, 0, &all);
        let sf = surprise_on(&c.life.agent, &ps, t + WARMUP, 0, &free);
        println!("{:<40} S_here(E_{t}) = {} bits (n {}); free-only {} (n {})", d.display(), fmt(s.mean), s.n, fmt(sf.mean), sf.n);
    }
}

pub fn probe_order_cmd(m: &Args) {
    let dirs: Vec<PathBuf> = req(m, "runs").split(',').map(PathBuf::from).collect();
    let t: u64 = get(m, "at", 1_000_000);
    let base = ckpt(&dirs[0], t);
    let seed = probe_seed(base.life.world.config().seed_world, t, 3);
    let ps = make_probe_stream(&base.life.world, seed, WARMUP);
    for d in &dirs {
        let c = ckpt(d, t);
        let r = order_probe(&c.life.agent, &ps.obs, &base.life.world, seed ^ 7, 20);
        println!("{:<40} D_order={} KL_truth(ABC)={} KL_truth(ACB)={} n={}", d.display(), fmt(r.d_order), fmt(r.kl_truth_abc), fmt(r.kl_truth_acb), r.n);
    }
}

pub fn readapt_cmd(m: &Args) {
    let run = PathBuf::from(req(m, "run"));
    let t_c: u64 = get(m, "at", 0);
    let horizon: u64 = get(m, "horizon", 50_000);
    let win: u64 = get(m, "window", 5_000);
    let changes = change_ticks(&run);
    let sites = changes.iter().find(|(t, _)| *t == t_c).map(|(_, s)| s.clone()).unwrap_or_else(|| {
        eprintln!("no structural change at {t_c}; known: {:?}", changes.iter().map(|c| c.0).collect::<Vec<_>>());
        std::process::exit(1)
    });
    let pos0 = position_at(&run.join("trace.bin"), t_c);
    let mut results = vec![readapt_from_trace("continue", &run.join("trace.bin"), t_c, horizon, win, &sites, None)];
    for (label, key) in [("fresh", "fresh"), ("frozen", "frozen")] {
        if let Some(d) = m.get(key) {
            results.push(readapt_from_trace(label, &Path::new(d).join("trace.bin"), t_c, horizon, win, &sites, Some(pos0)));
        }
    }
    for r in &results {
        println!("{:<9} t_c={} sites={:?} n={} mean[30k,50k]={} slope/10k={} converged={} windows={:?}", r.label, r.t_c, r.sites, r.n_events, fmt(r.mean_30k_50k), fmt(r.slope_last_20k_per_10k), r.converged, r.window_means.iter().map(|v| fmt(*v)).collect::<Vec<_>>());
    }
    if let Some(out) = m.get("out") {
        std::fs::write(out, serde_json::to_string_pretty(&results).unwrap()).expect("write");
    }
}

// ---------------------------------------------------------------- per-seed evaluation (A8)

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VariantEval {
    pub d_hist: f64,
    pub d_hist_n: u64,
    pub s_full: Vec<f64>,
    pub s_birth: Vec<f64>,
    pub n: Vec<u64>,
    pub order_full: Option<OrderResult>,
    pub order_birth: Option<OrderResult>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SeedEval {
    pub seed: u64,
    pub arms_status: BTreeMap<String, String>,
    pub streams_identical: bool,
    pub p1_d_hist: BTreeMap<String, f64>,
    pub p1_blocks: BTreeMap<String, [f64; NUM_BLOCKS]>,
    pub p1_n_events: u64,
    pub p2_checkpoints: Vec<u64>,
    pub p2_s: BTreeMap<String, Vec<f64>>,
    /// side analysis: the same, restricted to events that were not forced-cycle emissions
    pub p2_s_free: BTreeMap<String, Vec<f64>>,
    pub p2_n: Vec<u64>,
    pub p2_n_free: Vec<u64>,
    pub p2_floor: Vec<f64>,
    pub p2_truth_ref: Vec<f64>,
    pub p2_truth_ref_free: Vec<f64>,
    pub p3_s: BTreeMap<String, f64>,
    /// side analysis: the same copies taken at 2e5 instead of 1e6 (what was known when E_2e5 was made)
    pub p3_s_at_t1: BTreeMap<String, f64>,
    /// components: free emissions at unchanged sites; forced-cycle emissions at any site
    pub p3_s_free_unchanged: BTreeMap<String, f64>,
    pub p3_n_free_unchanged: u64,
    pub p3_s_forced: BTreeMap<String, f64>,
    pub p3_n_forced: u64,
    pub p3_n: u64,
    pub p3_unchanged_sites: Vec<u8>,
    pub p3_sufficient: bool,
    pub order: BTreeMap<String, OrderResult>,
    pub variants: BTreeMap<String, VariantEval>,
    pub notes: Vec<String>,
}

const MAIN_ARMS: [&str; 7] = ["full", "frozen", "readout", "homeo", "freeze_at", "fb_wt", "ltc"];
const VARIANT_ARMS: [&str; 3] = ["full_shuffled", "full_iid", "full_perm"];

pub fn evaluate_cmd(m: &Args) {
    let root = PathBuf::from(req(m, "root"));
    let seed: u64 = get(m, "seed", 0);
    let mut notes = Vec::new();
    let mut arms_status = BTreeMap::new();
    let mut present: Vec<&str> = Vec::new();
    for a in MAIN_ARMS.iter().chain(VARIANT_ARMS.iter()) {
        match load_summary(&root.join(a)) {
            Some(s) => {
                arms_status.insert(a.to_string(), s.status.clone());
                if s.status == "ok" && s.ticks >= P1_T2 {
                    present.push(a);
                } else {
                    notes.push(format!("{a}: status {} ticks {} -> excluded", s.status, s.ticks));
                }
            }
            None => notes.push(format!("{a}: missing")),
        }
    }
    let need = |a: &str| present.contains(&a);
    if !need("full") || !need("frozen") {
        eprintln!("evaluate: full and frozen arms are required");
        std::process::exit(1);
    }
    // A7: identical streams among main arms (input and actions); variants share actions only.
    let full_s = load_summary(&root.join("full")).unwrap();
    let mut identical = true;
    for a in &present {
        let s = load_summary(&root.join(a)).unwrap();
        if s.action_hash != full_s.action_hash {
            identical = false;
            notes.push(format!("{a}: action stream differs from full"));
        }
        if MAIN_ARMS.contains(a) && s.input_hash != full_s.input_hash {
            identical = false;
            notes.push(format!("{a}: input stream differs from full"));
        }
    }
    let world_seed = full_s.input_hash ^ 0; // placeholder replaced below
    let _ = world_seed;
    let full_c1 = ckpt(&root.join("full"), P1_T1);
    let wseed = full_c1.life.world.config().seed_world;
    let all = |_t: Tick, _s: u8| true;

    // ---- P1 history dependence
    let hist_stream = make_probe_stream(&full_c1.life.world, probe_seed(wseed, P1_T1, 1), WARMUP + HIST_TICKS);
    let mut p1 = BTreeMap::new();
    let mut p1b = BTreeMap::new();
    let mut p1n = 0;
    for a in &present {
        if !MAIN_ARMS.contains(a) {
            continue;
        }
        let c1 = ckpt(&root.join(a), P1_T1);
        let c2 = ckpt(&root.join(a), P1_T2);
        let own = make_probe_stream(&c1.life.world, probe_seed(wseed, P1_T1, 1), WARMUP + HIST_TICKS);
        if own.obs.iter().zip(&hist_stream.obs).any(|(x, y)| x.events != y.events) {
            identical = false;
            notes.push(format!("{a}: world at {P1_T1} differs from full (probe streams differ)"));
        }
        let h = d_hist(&c1.life.agent, &c2.life.agent, &hist_stream.obs, P1_T1 + WARMUP);
        p1.insert(a.to_string(), h.overall);
        p1b.insert(a.to_string(), h.per_block);
        p1n = h.n_total;
    }

    // ---- P2 usefulness on E_t, P3 persistence on E_{2e5}
    let ts: Vec<u64> = P2_CHECKPOINTS.to_vec();
    let tables = marginal_tables(&root.join("full").join("trace.bin"), &full_c1.life.world, &ts);
    let mut p2: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut p2free: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut p2n = Vec::new();
    let mut p2nfree = Vec::new();
    let mut p2floor = Vec::new();
    let mut p2truth = Vec::new();
    let mut p2truth_free = Vec::new();
    let mut e_first: Option<ProbeStream> = None;
    for (ti, &t) in ts.iter().enumerate() {
        let cf = ckpt(&root.join("full"), t);
        let ps = make_probe_stream(&cf.life.world, probe_seed(wseed, t, 2), WARMUP + EVAL_TICKS);
        let forced = forced_set(&ps);
        let free = |tk: Tick, s: u8| !forced.contains(&(tk, s));
        let tr = truth_reference(&cf.life.world, &ps, t + WARMUP, &all);
        p2truth.push(tr.mean);
        p2truth_free.push(truth_reference(&cf.life.world, &ps, t + WARMUP, &free).mean);
        p2floor.push(floor_surprise(&tables[ti], &ps, t + WARMUP, &all).mean);
        let mut n_here = 0;
        let mut n_free = 0;
        for a in &present {
            if !MAIN_ARMS.contains(a) {
                continue;
            }
            let c = ckpt(&root.join(a), t);
            let s = surprise_on(&c.life.agent, &ps, t + WARMUP, 0, &all);
            p2.entry(a.to_string()).or_default().push(s.mean);
            n_here = s.n;
            let sf = surprise_on(&c.life.agent, &ps, t + WARMUP, 0, &free);
            p2free.entry(a.to_string()).or_default().push(sf.mean);
            n_free = sf.n;
        }
        p2n.push(n_here);
        p2nfree.push(n_free);
        if ti == 0 {
            e_first = Some(ps);
        }
    }
    let e_first = e_first.unwrap();
    let mut changed: Vec<u8> = Vec::new();
    for (t, sites) in change_ticks(&root.join("full")) {
        if t > P1_T1 && t <= P1_T2 {
            for s in sites {
                if !changed.contains(&s) {
                    changed.push(s);
                }
            }
        }
    }
    let unchanged: Vec<u8> = (0..life_world::K as u8).filter(|s| !changed.contains(s)).collect();
    let unch = unchanged.clone();
    let forced_first = forced_set(&e_first);
    let ff = forced_first.clone();
    // P3 events: forced-cycle emissions (a site's cycle never changes) and free emissions at
    // sites whose grammar was not redrawn and that were not part of a cross-relation move
    let p3_ok = move |t: Tick, s: u8| ff.contains(&(t, s)) || unch.contains(&s);
    let unch2 = unchanged.clone();
    let ff2 = forced_first.clone();
    let p3_free_ok = move |t: Tick, s: u8| !ff2.contains(&(t, s)) && unch2.contains(&s);
    let ff3 = forced_first.clone();
    let p3_forced_ok = move |t: Tick, s: u8| ff3.contains(&(t, s));
    let mut p3 = BTreeMap::new();
    let mut p3t1 = BTreeMap::new();
    let mut p3_free = BTreeMap::new();
    let mut p3_forced = BTreeMap::new();
    let mut p3n = 0;
    let mut p3n_free = 0;
    let mut p3n_forced = 0;
    for a in &present {
        if !MAIN_ARMS.contains(a) {
            continue;
        }
        let c = ckpt(&root.join(a), P1_T2);
        let s = surprise_on(&c.life.agent, &e_first, P1_T1 + WARMUP, 0, &p3_ok);
        p3.insert(a.to_string(), s.mean);
        p3n = s.n;
        let sf = surprise_on(&c.life.agent, &e_first, P1_T1 + WARMUP, 0, &p3_free_ok);
        p3_free.insert(a.to_string(), sf.mean);
        p3n_free = sf.n;
        let sc = surprise_on(&c.life.agent, &e_first, P1_T1 + WARMUP, 0, &p3_forced_ok);
        p3_forced.insert(a.to_string(), sc.mean);
        p3n_forced = sc.n;
        let c1 = ckpt(&root.join(a), P1_T1);
        let s1 = surprise_on(&c1.life.agent, &e_first, P1_T1 + WARMUP, 0, &p3_ok);
        p3t1.insert(a.to_string(), s1.mean);
    }

    // ---- S1 order probe at 1e6 (frozen = birth network)
    let cf = ckpt(&root.join("full"), P1_T2);
    let warm = make_probe_stream(&cf.life.world, probe_seed(wseed, P1_T2, 3), WARMUP);
    let mut order = BTreeMap::new();
    for a in &present {
        if !MAIN_ARMS.contains(a) {
            continue;
        }
        let c = ckpt(&root.join(a), P1_T2);
        order.insert(a.to_string(), order_probe(&c.life.agent, &warm.obs, &cf.life.world, probe_seed(wseed, P1_T2, 4), 20));
    }

    // ---- control worlds: full agent vs birth network on the variant's own streams
    let mut variants = BTreeMap::new();
    for a in &present {
        if !VARIANT_ARMS.contains(a) {
            continue;
        }
        let d = root.join(a);
        let c1 = ckpt(&d, P1_T1);
        let c2 = ckpt(&d, P1_T2);
        let birth = ckpt(&d, 0);
        let hs = make_probe_stream(&c1.life.world, probe_seed(wseed, P1_T1, 1), WARMUP + HIST_TICKS);
        let h = d_hist(&c1.life.agent, &c2.life.agent, &hs.obs, P1_T1 + WARMUP);
        let (mut sf, mut sb, mut nn) = (Vec::new(), Vec::new(), Vec::new());
        for &t in &ts {
            let c = ckpt(&d, t);
            let ps = make_probe_stream(&c.life.world, probe_seed(wseed, t, 2), WARMUP + EVAL_TICKS);
            let s1 = surprise_on(&c.life.agent, &ps, t + WARMUP, 0, &all);
            let s0 = surprise_on(&birth.life.agent, &ps, t + WARMUP, 0, &all);
            let _ = &s0;
            sf.push(s1.mean);
            sb.push(s0.mean);
            nn.push(s1.n);
        }
        let (of, ob) = if c2.life.world.config().label_perm_seed.is_none() {
            let w = make_probe_stream(&c2.life.world, probe_seed(wseed, P1_T2, 3), WARMUP);
            (Some(order_probe(&c2.life.agent, &w.obs, &c2.life.world, probe_seed(wseed, P1_T2, 4), 20)), Some(order_probe(&birth.life.agent, &w.obs, &c2.life.world, probe_seed(wseed, P1_T2, 4), 20)))
        } else {
            (None, None)
        };
        variants.insert(a.to_string(), VariantEval { d_hist: h.overall, d_hist_n: h.n_total, s_full: sf, s_birth: sb, n: nn, order_full: of, order_birth: ob });
    }

    let ev = SeedEval { seed, arms_status, streams_identical: identical, p1_d_hist: p1, p1_blocks: p1b, p1_n_events: p1n, p2_checkpoints: ts, p2_s: p2, p2_s_free: p2free, p2_n: p2n, p2_n_free: p2nfree, p2_floor: p2floor, p2_truth_ref: p2truth, p2_truth_ref_free: p2truth_free, p3_s: p3, p3_s_at_t1: p3t1, p3_s_free_unchanged: p3_free, p3_n_free_unchanged: p3n_free, p3_s_forced: p3_forced, p3_n_forced: p3n_forced, p3_n: p3n, p3_unchanged_sites: unchanged, p3_sufficient: p3n >= MIN_EVENTS, order, variants, notes };
    let out = root.join("evaluate.json");
    std::fs::write(&out, serde_json::to_string_pretty(&ev).unwrap()).expect("write");
    print_seed_eval(&ev);
    println!("written {}", out.display());
}

pub fn print_seed_eval(e: &SeedEval) {
    println!("== seed {} streams_identical={} ==", e.seed, e.streams_identical);
    println!("P1 D_hist(2e5,1e6) [{} events]: {}", e.p1_n_events, e.p1_d_hist.iter().map(|(a, v)| format!("{a}={}", fmt(*v))).collect::<Vec<_>>().join("  "));
    println!("P2 S_here on E_t, t={:?} (n {:?})", e.p2_checkpoints, e.p2_n);
    println!("   floor(marginal)={:?}  generative_ref={:?}  generative_ref(free)={:?}", e.p2_floor.iter().map(|v| fmt(*v)).collect::<Vec<_>>(), e.p2_truth_ref.iter().map(|v| fmt(*v)).collect::<Vec<_>>(), e.p2_truth_ref_free.iter().map(|v| fmt(*v)).collect::<Vec<_>>());
    for (a, v) in &e.p2_s {
        println!("   {:<10} {:?}  mean={}   free-only mean={} (n {:?})", a, v.iter().map(|x| fmt(*x)).collect::<Vec<_>>(), fmt(mean(v)), fmt(e.p2_s_free.get(a).map(|f| mean(f)).unwrap_or(f64::NAN)), e.p2_n_free);
    }
    println!("P3 S_here(1e6) on E_2e5, structure-unchanged events (forced cycles + free at unchanged sites {:?}) (n {}, sufficient={}): {}", e.p3_unchanged_sites, e.p3_n, e.p3_sufficient, e.p3_s.iter().map(|(a, v)| format!("{a}={}", fmt(*v))).collect::<Vec<_>>().join("  "));
    println!("   free at unchanged sites only (n {}): {}", e.p3_n_free_unchanged, e.p3_s_free_unchanged.iter().map(|(a, v)| format!("{a}={}", fmt(*v))).collect::<Vec<_>>().join("  "));
    println!("   forced cycles only (n {}): {}", e.p3_n_forced, e.p3_s_forced.iter().map(|(a, v)| format!("{a}={}", fmt(*v))).collect::<Vec<_>>().join("  "));
    println!("   same copies at 2e5: {}", e.p3_s_at_t1.iter().map(|(a, v)| format!("{a}={}", fmt(*v))).collect::<Vec<_>>().join("  "));
    for (a, o) in &e.order {
        println!("S1 order {:<10} D_order={} KL_truth(ABC)={} KL_truth(ACB)={}", a, fmt(o.d_order), fmt(o.kl_truth_abc), fmt(o.kl_truth_acb));
    }
    for (a, v) in &e.variants {
        println!("variant {:<14} D_hist={} S_full={:?} S_birth={:?}", a, fmt(v.d_hist), v.s_full.iter().map(|x| fmt(*x)).collect::<Vec<_>>(), v.s_birth.iter().map(|x| fmt(*x)).collect::<Vec<_>>());
    }
    for n in &e.notes {
        println!("note: {n}");
    }
}
