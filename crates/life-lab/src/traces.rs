//! Trace-level tools: audit (time-order and numerics), summary (windows), compare (identical
//! streams across arms), changes (structural-change ticks from the truth log).

use crate::common::*;
use life_interface::*;
use life_trace::{read_each, read_vec, Rec};
use life_world::TruthEvent;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub fn audit(m: &Args) {
    let dir = PathBuf::from(req(m, "run"));
    let mut n_eval = 0u64;
    let mut n_viol = 0u64;
    let mut n_numeric = 0u64;
    let mut n_obs = 0u64;
    let mut last_obs_tick = 0u64;
    let mut n_order_viol = 0u64;
    let mut first_viol: Option<(u64, u64)> = None;
    read_each(&dir.join("trace.bin"), |r| match r {
        Rec::Eval { tick, made_at, last_obs, .. } => {
            n_eval += 1;
            if !(made_at < tick && last_obs <= made_at) {
                n_viol += 1;
                if first_viol.is_none() {
                    first_viol = Some((tick, made_at));
                }
            }
        }
        Rec::Obs { tick, .. } => {
            n_obs += 1;
            if tick <= last_obs_tick {
                n_order_viol += 1;
            }
            last_obs_tick = tick;
        }
        Rec::Numeric { .. } => n_numeric += 1,
        _ => {}
    })
    .expect("read trace");
    let ok = n_viol == 0 && n_order_viol == 0;
    println!("audit {}: evaluations={} time_order_violations={} obs_records={} obs_order_violations={} numeric_records={} -> {}", dir.display(), n_eval, n_viol, n_obs, n_order_viol, n_numeric, if ok { "OK" } else { "FAIL" });
    if let Some((t, m)) = first_viol {
        println!("  first violation: tick {t} made_at {m}");
    }
    if !ok {
        std::process::exit(1);
    }
}

#[derive(Serialize, Default, Clone)]
pub struct Window {
    pub start: u64,
    pub end: u64,
    pub n_events: [u64; NUM_BLOCKS],
    pub mean_surprise: [f64; NUM_BLOCKS],
    pub mean_entropy: [f64; NUM_BLOCKS],
    pub dw_hidden_norm: f64,
    pub dw_out_norm: f64,
    pub clips_hidden: u64,
    pub clips_out: u64,
    pub learn_ticks: u64,
    pub homeo_applications: u64,
    pub homeo_dw_norm: f64,
    pub homeo_mean_rms: f64,
    pub homeo_frac_scaled: f64,
    pub homeo_saturated_unit_applications: u64,
    pub homeo_clips: u64,
    pub state_mean_rrms: f64,
    pub state_max_x: f64,
    pub body_min_resource: i32,
    pub numeric_records: u64,
    pub n_here_forced: u64,
    pub s_here_forced: f64,
    pub n_here_free: u64,
    pub s_here_free: f64,
}

pub fn summarize(dir: &Path, window: u64) -> Vec<Window> {
    // truth join: forced-cycle emissions (tick, site); position from block-3 observations
    let truth: Vec<TruthEvent> = read_vec(&dir.join("truth.bin")).unwrap_or_default();
    let mut forced: std::collections::HashSet<(u64, u8)> = std::collections::HashSet::new();
    for e in &truth {
        if let TruthEvent::Emission { tick, site, forced: true, .. } = e {
            forced.insert((*tick, *site));
        }
    }
    let mut pos = 0u8;
    let mut fsum = vec![0.0f64];
    let mut fcnt = vec![0u64];
    let mut usum = vec![0.0f64];
    let mut ucnt = vec![0u64];
    let mut wins: Vec<Window> = Vec::new();
    let mut sums = vec![[0.0f64; NUM_BLOCKS]];
    let mut hsums = vec![[0.0f64; NUM_BLOCKS]];
    let mut dwh = vec![0.0f64];
    let mut dwo = vec![0.0f64];
    let mut hdw = vec![0.0f64];
    let mut hrms = vec![0.0f64];
    let mut hscaled = vec![0.0f64];
    let mut hunits = vec![0u64];
    let mut srrms = vec![0.0f64];
    let mut sn = vec![0u64];
    let idx = |t: u64| ((t.saturating_sub(1)) / window) as usize;
    let ensure = |wins: &mut Vec<Window>, sums: &mut Vec<[f64; 6]>, hsums: &mut Vec<[f64; 6]>, dwh: &mut Vec<f64>, dwo: &mut Vec<f64>, hdw: &mut Vec<f64>, hrms: &mut Vec<f64>, hscaled: &mut Vec<f64>, hunits: &mut Vec<u64>, srrms: &mut Vec<f64>, sn: &mut Vec<u64>, i: usize| {
        while wins.len() <= i {
            let k = wins.len() as u64;
            wins.push(Window { start: k * window + 1, end: (k + 1) * window, body_min_resource: i32::MAX, ..Default::default() });
            sums.push([0.0; 6]);
            hsums.push([0.0; 6]);
            dwh.push(0.0);
            dwo.push(0.0);
            hdw.push(0.0);
            hrms.push(0.0);
            hscaled.push(0.0);
            hunits.push(0);
            srrms.push(0.0);
            sn.push(0);
        }
    };
    // the first window entry exists already (index 0)
    wins.push(Window { start: 1, end: window, body_min_resource: i32::MAX, ..Default::default() });
    read_each(&dir.join("trace.bin"), |r| match r {
        Rec::Eval { tick, block, surprise, entropy, .. } => {
            let i = idx(tick);
            ensure(&mut wins, &mut sums, &mut hsums, &mut dwh, &mut dwo, &mut hdw, &mut hrms, &mut hscaled, &mut hunits, &mut srrms, &mut sn, i);
            while fsum.len() <= i {
                fsum.push(0.0);
                fcnt.push(0);
                usum.push(0.0);
                ucnt.push(0);
            }
            wins[i].n_events[block as usize] += 1;
            sums[i][block as usize] += surprise as f64;
            hsums[i][block as usize] += entropy as f64;
            if block == 0 {
                if forced.contains(&(tick, pos)) {
                    fsum[i] += surprise as f64;
                    fcnt[i] += 1;
                } else {
                    usum[i] += surprise as f64;
                    ucnt[i] += 1;
                }
            }
        }
        Rec::Obs { events, .. } => {
            for (b, v) in events {
                if b == 3 {
                    pos = v;
                }
            }
        }
        Rec::Learn { tick, dw_in2, dw_rec2, dw_out2, clips_hidden, clips_out, .. } => {
            let i = idx(tick);
            ensure(&mut wins, &mut sums, &mut hsums, &mut dwh, &mut dwo, &mut hdw, &mut hrms, &mut hscaled, &mut hunits, &mut srrms, &mut sn, i);
            dwh[i] += (dw_in2 + dw_rec2) as f64;
            dwo[i] += dw_out2 as f64;
            wins[i].clips_hidden += clips_hidden as u64;
            wins[i].clips_out += clips_out as u64;
            wins[i].learn_ticks += 1;
        }
        Rec::Homeo { tick, rms, scale, dw2, saturated, clips } => {
            let i = idx(tick);
            ensure(&mut wins, &mut sums, &mut hsums, &mut dwh, &mut dwo, &mut hdw, &mut hrms, &mut hscaled, &mut hunits, &mut srrms, &mut sn, i);
            wins[i].homeo_applications += 1;
            wins[i].homeo_saturated_unit_applications += saturated as u64;
            wins[i].homeo_clips += clips as u64;
            hdw[i] += dw2 as f64;
            for v in &rms {
                hrms[i] += *v as f64;
            }
            for s in &scale {
                if *s != 1.0 {
                    hscaled[i] += 1.0;
                }
            }
            hunits[i] += rms.len() as u64;
        }
        Rec::State { tick, x_max, r_rms, .. } => {
            let i = idx(tick);
            ensure(&mut wins, &mut sums, &mut hsums, &mut dwh, &mut dwo, &mut hdw, &mut hrms, &mut hscaled, &mut hunits, &mut srrms, &mut sn, i);
            srrms[i] += r_rms as f64;
            sn[i] += 1;
            if (x_max as f64) > wins[i].state_max_x {
                wins[i].state_max_x = x_max as f64;
            }
        }
        Rec::Body { tick, resource } => {
            let i = idx(tick);
            ensure(&mut wins, &mut sums, &mut hsums, &mut dwh, &mut dwo, &mut hdw, &mut hrms, &mut hscaled, &mut hunits, &mut srrms, &mut sn, i);
            if resource < wins[i].body_min_resource {
                wins[i].body_min_resource = resource;
            }
        }
        Rec::Numeric { tick, .. } => {
            let i = idx(tick);
            ensure(&mut wins, &mut sums, &mut hsums, &mut dwh, &mut dwo, &mut hdw, &mut hrms, &mut hscaled, &mut hunits, &mut srrms, &mut sn, i);
            wins[i].numeric_records += 1;
        }
        _ => {}
    })
    .expect("read trace");
    for i in 0..wins.len() {
        for k in 0..NUM_BLOCKS {
            let n = wins[i].n_events[k];
            wins[i].mean_surprise[k] = if n > 0 { sums[i][k] / n as f64 } else { f64::NAN };
            wins[i].mean_entropy[k] = if n > 0 { hsums[i][k] / n as f64 } else { f64::NAN };
        }
        if i < fsum.len() {
            wins[i].n_here_forced = fcnt[i];
            wins[i].s_here_forced = if fcnt[i] > 0 { fsum[i] / fcnt[i] as f64 } else { f64::NAN };
            wins[i].n_here_free = ucnt[i];
            wins[i].s_here_free = if ucnt[i] > 0 { usum[i] / ucnt[i] as f64 } else { f64::NAN };
        }
        wins[i].dw_hidden_norm = dwh[i].sqrt();
        wins[i].dw_out_norm = dwo[i].sqrt();
        wins[i].homeo_dw_norm = hdw[i].sqrt();
        wins[i].homeo_mean_rms = if hunits[i] > 0 { hrms[i] / hunits[i] as f64 } else { f64::NAN };
        wins[i].homeo_frac_scaled = if hunits[i] > 0 { hscaled[i] / hunits[i] as f64 } else { f64::NAN };
        wins[i].state_mean_rrms = if sn[i] > 0 { srrms[i] / sn[i] as f64 } else { f64::NAN };
        if wins[i].body_min_resource == i32::MAX {
            wins[i].body_min_resource = -1;
        }
    }
    wins
}

pub fn summary(m: &Args) {
    let dir = PathBuf::from(req(m, "run"));
    let window: u64 = get(m, "window", 100_000);
    let wins = summarize(&dir, window);
    let truth: Vec<TruthEvent> = read_vec(&dir.join("truth.bin")).unwrap_or_default();
    let (mut mf, mut rs, mut sc, mut ms) = (0, 0, 0, 0);
    let mut change_ticks = Vec::new();
    for e in &truth {
        match e {
            TruthEvent::MaintenanceFailure { .. } => mf += 1,
            TruthEvent::ExternalRescue { .. } => rs += 1,
            TruthEvent::StructuralChange { tick, .. } => {
                sc += 1;
                change_ticks.push(*tick);
            }
            TruthEvent::ModeSwitch { .. } => ms += 1,
            _ => {}
        }
    }
    println!("summary {} (window {})", dir.display(), window);
    println!("truth: structural_changes={} at {:?}; mode_switches={}; maintenance_failures={}; rescues={}", sc, change_ticks, ms, mf, rs);
    println!("{:>8} {:>7} {:>7} {:>5} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>9} {:>9} {:>6} {:>9} {:>6} {:>6} {:>7} {:>7} {:>7} {:>6}", "end", "S_here", "S_forc", "%forc", "S_free", "S_left", "S_right", "S_pos", "S_inter", "S_echo", "dW_hid", "dW_out", "clips", "homeo_dW", "h_sat", "h_clip", "h_rms", "r_rms", "x_max", "Rmin");
    for w in &wins {
        let nh = (w.n_here_forced + w.n_here_free).max(1) as f64;
        println!("{:>8} {:>7} {:>7} {:>5.0} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>9.4} {:>9.4} {:>6} {:>9.4} {:>6} {:>6} {:>7} {:>7} {:>7.2} {:>6}", w.end, f3(w.mean_surprise[0]), f3(w.s_here_forced), 100.0 * w.n_here_forced as f64 / nh, f3(w.s_here_free), f3(w.mean_surprise[1]), f3(w.mean_surprise[2]), f3(w.mean_surprise[3]), f3(w.mean_surprise[4]), f3(w.mean_surprise[5]), w.dw_hidden_norm, w.dw_out_norm, w.clips_hidden + w.clips_out, w.homeo_dw_norm, w.homeo_saturated_unit_applications, w.homeo_clips, f3(w.homeo_mean_rms), f3(w.state_mean_rrms), w.state_max_x, w.body_min_resource);
    }
    let out = dir.join("summary_windows.json");
    std::fs::write(&out, serde_json::to_string_pretty(&wins).unwrap()).expect("write");
    println!("written {}", out.display());
}

fn f3(v: f64) -> String {
    if v.is_nan() {
        "-".into()
    } else {
        format!("{v:.3}")
    }
}

/// Require identical input and action streams across run directories (A7).
pub fn compare(m: &Args) {
    let dirs: Vec<PathBuf> = req(m, "runs").split(',').map(PathBuf::from).collect();
    let actions_only = m.contains_key("actions-only");
    let mut ok = true;
    let mut first: Option<(u64, u64, u64)> = None;
    for d in &dirs {
        let s = match load_summary(d) {
            Some(s) => s,
            None => {
                println!("{}: no summary.json", d.display());
                ok = false;
                continue;
            }
        };
        println!("{:<40} arm={:<10} ticks={:<8} input_hash={:016x} action_hash={:016x} status={}", d.display(), s.arm, s.ticks, s.input_hash, s.action_hash, s.status);
        match first {
            None => first = Some((s.input_hash, s.action_hash, s.ticks)),
            Some((ih, ah, tk)) => {
                if s.action_hash != ah || s.ticks != tk || (!actions_only && s.input_hash != ih) {
                    ok = false;
                }
            }
        }
    }
    println!("compare: {}", if ok { "IDENTICAL streams" } else { "STREAMS DIFFER" });
    if !ok {
        std::process::exit(1);
    }
}

pub fn change_ticks(dir: &Path) -> Vec<(u64, Vec<u8>)> {
    let truth: Vec<TruthEvent> = read_vec(&dir.join("truth.bin")).unwrap_or_default();
    let mut v = Vec::new();
    for e in &truth {
        if let TruthEvent::StructuralChange { tick, sites, cross_moved_from, cross_moved_to } = e {
            let mut s = sites.clone();
            if let Some(a) = cross_moved_from {
                if !s.contains(a) {
                    s.push(*a);
                }
            }
            if let Some(a) = cross_moved_to {
                if !s.contains(a) {
                    s.push(*a);
                }
            }
            v.push((*tick, s));
        }
    }
    v
}

pub fn changes(m: &Args) {
    let dir = PathBuf::from(req(m, "run"));
    for (t, sites) in change_ticks(&dir) {
        println!("{t} {}", sites.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(","));
    }
}
