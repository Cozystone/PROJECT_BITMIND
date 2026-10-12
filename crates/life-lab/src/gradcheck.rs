//! Finite-difference check of the eligibility-trace update (docs/APPENDIX-FIRST-EXPERIMENT.md A2).
//!
//! The accumulated dry-run update of the actual learner code over a short sequence is compared
//! with the central finite-difference gradient of the summed natural-log loss of the same
//! sequence, at fixed weights. Reported per group (W_rec, W_in, W_out incl. biases): cosine
//! similarity and norm ratio ‖update‖/‖gradient‖. With W_rec = 0 the trace is the exact
//! derivative, so cosine ≈ 1 validates the implementation; with recurrence the trace ignores the
//! paths through other units, and with random feedback (B) the learning signal is not the
//! gradient at all, so those numbers quantify the approximation rather than test correctness.

use crate::common::*;
use life_agent::{Agent, AgentConfig, Feedback};
use life_interface::*;
use life_run::make_probe_stream;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize, Debug, Clone)]
pub struct GroupResult {
    pub group: String,
    pub cosine: f64,
    pub norm_ratio: f64,
    pub n_params: usize,
}

#[derive(Serialize, Debug, Clone)]
pub struct GradcheckResult {
    pub setting: String,
    pub n: usize,
    pub ticks: usize,
    pub evaluations: usize,
    pub delta: f32,
    pub groups: Vec<GroupResult>,
    pub cosine_all: f64,
}

fn synthetic_stream(seed: u64, t0: Tick, ticks: u64) -> Vec<Observation> {
    let mut rng = Rng::new(seed);
    let mut v = Vec::new();
    for dt in 1..=ticks {
        let t = t0 + dt;
        let mut ev = Vec::new();
        if dt % 4 == 0 {
            ev.push(ObsEvent { block: 0, value: rng.below(6) as u8 });
        }
        if dt % 7 == 0 {
            ev.push(ObsEvent { block: 1, value: rng.below(6) as u8 });
        }
        if dt % 20 == 0 {
            ev.push(ObsEvent { block: 4, value: rng.below(16) as u8 });
        }
        if dt == 1 {
            ev.push(ObsEvent { block: 3, value: 2 });
        }
        v.push(Observation { tick: t, events: ev });
    }
    v
}

/// Summed natural-log loss of a learning-disabled copy over the stream.
fn loss_of(agent: &Agent, stream: &[Observation]) -> (f64, usize) {
    let mut a = agent.clone();
    a.set_learning(false, false, false);
    let mut l = 0.0f64;
    let mut n = 0;
    for o in stream {
        let rep = a.tick(o);
        for e in rep.evaluations {
            l += -(e.p_of_value as f64).ln();
            n += 1;
        }
    }
    (l, n)
}

fn cosine(a: &[f64], b: &[f64]) -> (f64, f64) {
    let dot: f64 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let nb: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    (dot / (na * nb).max(1e-30), na / nb.max(1e-30))
}

pub fn check(base: &Agent, stream: &[Observation], delta: f32, setting: &str) -> GradcheckResult {
    let n = base.n();
    let m = OBS_DIM;
    // dry-run accumulated update (η = 1 so the buffers hold the raw rule output)
    let mut dry = base.clone();
    dry.set_learning(true, true, false);
    dry.set_rates(1.0, 1.0);
    dry.set_dry_run(true);
    let mut n_eval = 0;
    for o in stream {
        n_eval += dry.tick(o).evaluations.len();
    }
    let (d_rec, d_in, d_out, d_cout) = dry.dry_update();
    // descent direction = -gradient; the rule adds +η·L·r'·ε, so compare -update with the gradient
    let upd_rec: Vec<f64> = d_rec.iter().map(|v| -(*v as f64)).collect();
    let upd_in: Vec<f64> = d_in.iter().map(|v| -(*v as f64)).collect();
    let mut upd_out: Vec<f64> = Vec::new();
    for k in 0..NUM_BLOCKS {
        upd_out.extend(d_out[k].iter().map(|v| -(*v as f64)));
        upd_out.extend(d_cout[k].iter().map(|v| -(*v as f64)));
    }
    // finite differences
    let fd = |set: &dyn Fn(&mut Agent, f32), w0: f32| -> f64 {
        let mut p = base.clone();
        set(&mut p, w0 + delta);
        let (lp, _) = loss_of(&p, stream);
        let mut q = base.clone();
        set(&mut q, w0 - delta);
        let (lm, _) = loss_of(&q, stream);
        (lp - lm) / (2.0 * delta as f64)
    };
    let mut g_rec = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in 0..n {
            let w0 = base.w_rec()[i * n + j];
            g_rec.push(fd(&|a: &mut Agent, v: f32| a.set_w_rec(i, j, v), w0));
        }
    }
    let mut g_in = Vec::with_capacity(n * m);
    for i in 0..n {
        for j in 0..m {
            let w0 = base.w_in()[i * m + j];
            g_in.push(fd(&|a: &mut Agent, v: f32| a.set_w_in(i, j, v), w0));
        }
    }
    let mut g_out = Vec::new();
    for k in 0..NUM_BLOCKS {
        for q in 0..BLOCK_SIZES[k] {
            for i in 0..n {
                let w0 = base.w_out(k)[q * n + i];
                g_out.push(fd(&|a: &mut Agent, v: f32| a.set_w_out(k, q, i, v), w0));
            }
        }
        for q in 0..BLOCK_SIZES[k] {
            let w0 = base.c_out(k)[q];
            g_out.push(fd(&|a: &mut Agent, v: f32| a.set_c_out(k, q, v), w0));
        }
    }
    let mut groups = Vec::new();
    for (name, u, g) in [("W_rec", &upd_rec, &g_rec), ("W_in", &upd_in, &g_in), ("W_out+bias", &upd_out, &g_out)] {
        let (c, r) = cosine(u, g);
        groups.push(GroupResult { group: name.into(), cosine: c, norm_ratio: r, n_params: g.len() });
    }
    let mut ua = upd_rec.clone();
    ua.extend(&upd_in);
    ua.extend(&upd_out);
    let mut ga = g_rec.clone();
    ga.extend(&g_in);
    ga.extend(&g_out);
    let (ca, _) = cosine(&ua, &ga);
    GradcheckResult { setting: setting.into(), n, ticks: stream.len(), evaluations: n_eval, delta, groups, cosine_all: ca }
}

pub fn run(m: &Args) {
    let delta: f32 = get(m, "delta", 1e-2);
    let ticks: u64 = get(m, "ticks", 80);
    let mut results = Vec::new();
    if let Some(ck) = m.get("ckpt") {
        // trained network from a checkpoint, on a continuation of its own world
        let c = life_run::load_checkpoint(&PathBuf::from(ck)).expect("checkpoint");
        let ps = make_probe_stream(&c.life.world, 0xC4EC ^ c.life.tick, ticks);
        let agent = c.life.agent.clone();
        let fb = agent.config().feedback;
        results.push(check(&agent, &ps.obs, delta, &format!("checkpoint tick {} feedback {:?}", c.life.tick, fb)));
        let mut wt_cfg = agent.config().clone();
        wt_cfg.feedback = Feedback::WeightTransport;
        // same weights, exact learning signal: rebuild is not possible without copying weights,
        // so report the configured feedback only unless it already is weight transport
        let _ = wt_cfg;
    } else {
        let n: usize = get(m, "n", 8);
        for (grec, label) in [(0.0f32, "W_rec = 0 (trace exact)"), (1.0f32, "recurrent g_rec = 1")] {
            for fb in [Feedback::WeightTransport, Feedback::Random] {
                let cfg = AgentConfig { n, g_rec: grec, feedback: fb, homeostasis: false, seed: get(m, "seed", 21), ..AgentConfig::default() };
                let base = Agent::new(cfg);
                let stream = synthetic_stream(get(m, "stream-seed", 5), 0, ticks);
                results.push(check(&base, &stream, delta, &format!("{label}, L = {}", if fb == Feedback::Random { "B e (random)" } else { "W_out^T e" })));
            }
        }
    }
    for r in &results {
        println!("gradcheck [{}] n={} ticks={} evaluations={} delta={}", r.setting, r.n, r.ticks, r.evaluations, r.delta);
        for g in &r.groups {
            println!("   {:<11} cosine={:>8.4}  norm_ratio={:>8.4}  params={}", g.group, g.cosine, g.norm_ratio, g.n_params);
        }
        println!("   all         cosine={:>8.4}", r.cosine_all);
    }
    if let Some(out) = m.get("out") {
        std::fs::write(out, serde_json::to_string_pretty(&results).unwrap()).expect("write");
    }
}
