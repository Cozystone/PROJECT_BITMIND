//! Development diagnostics: per-unit weight statistics from a checkpoint joined with the last
//! homeostasis record of its run (which units run away, which stay silent), and a small sweep of
//! the synthetic 4-cycle learnability test over initial scales and learning rates. Development
//! seeds only; nothing here enters the evidence.

use crate::common::*;
use life_agent::{Agent, AgentConfig};
use life_interface::*;
use life_trace::{read_each, Rec};
use std::path::PathBuf;

pub fn inspect(m: &Args) {
    let path = PathBuf::from(req(m, "ckpt"));
    let c = life_run::load_checkpoint(&path).expect("checkpoint");
    let a = &c.life.agent;
    let n = a.n();
    let mm = OBS_DIM;
    let wmax = a.config().w_max_hidden;
    let mut last_rms: Vec<f32> = Vec::new();
    if let Some(run) = m.get("run") {
        let t = c.life.tick;
        read_each(&PathBuf::from(run).join("trace.bin"), |r| {
            if let Rec::Homeo { tick, rms, .. } = r {
                if tick <= t {
                    last_rms = rms;
                }
            }
        })
        .expect("read trace");
    }
    println!("checkpoint {} tick {} n={} arm cfg: learn_hidden={} learn_readout={} homeostasis={} feedback={:?} ltc={}", path.display(), c.life.tick, n, a.config().learn_hidden, a.config().learn_readout, a.config().homeostasis, a.config().feedback, a.config().ltc);
    println!("{:>4} {:>7} {:>8} {:>7} {:>8} {:>8} {:>8} {:>6}", "unit", "tau", "rms_last", "h_gain", "|W_in|", "|W_rec|", "max|w|", "clip");
    let mut n_clip_units = 0;
    let mut n_clip_w = 0;
    let mut silent = 0;
    for i in 0..n {
        let win = &a.w_in()[i * mm..(i + 1) * mm];
        let wrec = &a.w_rec()[i * n..(i + 1) * n];
        let nin = win.iter().map(|v| v * v).sum::<f32>().sqrt();
        let nrec = wrec.iter().map(|v| v * v).sum::<f32>().sqrt();
        let mx = win.iter().chain(wrec).fold(0f32, |m, v| m.max(v.abs()));
        let clip = win.iter().chain(wrec).filter(|v| v.abs() >= wmax * 0.995).count();
        if clip > 0 {
            n_clip_units += 1;
            n_clip_w += clip;
        }
        let rms = last_rms.get(i).cloned().unwrap_or(f32::NAN);
        if rms < 0.5 * a.config().rms_target {
            silent += 1;
        }
        if m.contains_key("all") || clip > 0 || rms < 0.5 * a.config().rms_target || rms > 1.5 * a.config().rms_target {
            println!("{:>4} {:>7.1} {:>8.3} {:>7.3} {:>8.3} {:>8.3} {:>8.3} {:>6}", i, a.tau()[i], rms, a.homeostatic_gain()[i], nin, nrec, mx, clip);
        }
    }
    let mut out_max = 0f32;
    for k in 0..NUM_BLOCKS {
        out_max = a.w_out(k).iter().fold(out_max, |m, v| m.max(v.abs()));
    }
    println!("units with clipped incoming weights: {n_clip_units}/{n} ({n_clip_w} weights at |w|>={wmax}); units with rms < half target: {silent}; max |W_out| = {out_max:.3}");
}

fn cycle_run(cfg: AgentConfig, hidden: bool, readout: bool) -> f64 {
    let mut c = cfg;
    c.learn_hidden = hidden;
    c.learn_readout = readout;
    let mut a = Agent::new(c);
    let cycle = [0u8, 3, 1, 4];
    let (mut sum, mut cnt) = (0.0f64, 0u64);
    let mut k = 0usize;
    for t in 1..=40_000u64 {
        let events = if t % 5 == 0 {
            k += 1;
            vec![ObsEvent { block: 0, value: cycle[k % 4] }]
        } else {
            vec![]
        };
        let rep = a.tick(&Observation { tick: t, events });
        if t > 30_000 {
            for e in &rep.evaluations {
                sum += e.surprise_bits as f64;
                cnt += 1;
            }
        }
    }
    sum / cnt as f64
}

pub fn devsweep(m: &Args) {
    let seed: u64 = get(m, "seed", 3);
    println!("synthetic 4-cycle (period 5 ticks), mean block-0 surprise over ticks 30k-40k, seed {seed}");
    println!("{:>6} {:>6} {:>8} {:>8} {:>8} | {:>8} {:>8} {:>8}", "g_in", "g_rec", "eta_out", "eta_hid", "rms*", "full", "readout", "frozen");
    for g_in in [0.5f32, 1.0, 2.0] {
        for g_rec in [0.5f32, 1.0] {
            for eta_out in [0.02f32, 0.05] {
                for eta_h in [0.002f32, 0.0005] {
                    let cfg = AgentConfig { g_in, g_rec, eta_out, eta_hidden: eta_h, seed, ..AgentConfig::default() };
                    let full = cycle_run(cfg.clone(), true, true);
                    let ro = cycle_run(cfg.clone(), false, true);
                    let fr = cycle_run(cfg.clone(), false, false);
                    println!("{:>6.2} {:>6.2} {:>8.3} {:>8.4} {:>8.2} | {:>8.3} {:>8.3} {:>8.3}", g_in, g_rec, eta_out, eta_h, cfg.rms_target, full, ro, fr);
                }
            }
        }
    }
}
