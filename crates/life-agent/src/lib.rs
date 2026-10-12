//! life-agent: candidate C (docs/ARCHITECTURE-OPTIONS.md section 3, docs/APPENDIX-FIRST-EXPERIMENT.md).
//!
//! A continuous-time leaky-integrator recurrent network, f32, deterministic operation order.
//! The only plasticity is a local three-factor rule driven by the error of the network's own
//! prediction of the next event of each observation block. The order inside one tick is fixed by
//! `Agent::tick`: evaluate (stored predictions against the events that arrived) -> learn ->
//! incorporate the observation -> predict. A prediction evaluated against an event was therefore
//! stored before that event existed (`Evaluation::made_at < tick`).
//!
//! This crate depends on `life-interface` only. It contains no world names, channel ids as
//! constants beyond the block layout, or action-specific branches.

use life_interface::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Feedback {
    /// L = B e with a fixed random B (direct feedback alignment).
    Random,
    /// L = W_out^T e (weight transport; ablation).
    WeightTransport,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentConfig {
    pub n: usize,
    pub tau_min: f32,
    pub tau_max: f32,
    pub eta_hidden: f32,
    pub eta_out: f32,
    pub eta_h: f32,
    pub rms_target: f32,
    pub t_h: u64,
    /// cumulative homeostatic gain of a unit's incoming weights stays within [1/h_max, h_max]
    pub h_max: f32,
    pub w_max_hidden: f32,
    pub w_max_out: f32,
    pub g_rec: f32,
    /// birth std of W_in columns of pulse blocks (tokens, action echo)
    pub g_in: f32,
    /// birth std of W_in columns of held blocks (position, interoception), whose sustained input
    /// sets a fixed point and saturates units if drawn as large as pulse weights
    pub g_in_held: f32,
    pub feedback: Feedback,
    pub ltc: bool,
    pub learn_hidden: bool,
    pub learn_readout: bool,
    pub homeostasis: bool,
    /// all weight change stops at this tick (the `freeze_at` arm)
    pub freeze_at: Option<Tick>,
    pub seed: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        AgentConfig {
            n: 64,
            tau_min: 4.0,
            tau_max: 400.0,
            eta_hidden: 0.002,
            eta_out: 0.02,
            eta_h: 0.05,
            rms_target: 0.4,
            t_h: 500,
            h_max: 4.0,
            w_max_hidden: 6.0,
            w_max_out: 6.0,
            g_rec: 1.0,
            g_in: 2.0,
            g_in_held: 0.5,
            feedback: Feedback::Random,
            ltc: false,
            learn_hidden: true,
            learn_readout: true,
            homeostasis: true,
            freeze_at: None,
            seed: 7,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Evaluation {
    pub block: u8,
    pub value: u8,
    /// tick at which the evaluated prediction was computed (always < the event tick)
    pub made_at: Tick,
    /// last observation incorporated before that prediction
    pub last_obs_tick: Tick,
    pub surprise_bits: f32,
    pub p_of_value: f32,
    pub entropy_bits: f32,
    /// the evaluated prediction itself
    pub probs: Vec<f32>,
}

#[derive(Clone, Debug, Default)]
pub struct LearnStats {
    pub dw_in2: f32,
    pub dw_rec2: f32,
    pub dw_out2: f32,
    pub clips_hidden: u32,
    pub clips_out: u32,
    pub l_norm2: f32,
}

#[derive(Clone, Debug)]
pub struct HomeoStats {
    pub rms: Vec<f32>,
    pub scale: Vec<f32>,
    pub dw2: f32,
    /// units whose cumulative gain sits at the bound this application
    pub saturated: u32,
    /// weights held at the bound |w| <= w_max_hidden by this application
    pub clips: u32,
}

#[derive(Clone, Debug, Default)]
pub struct TickReport {
    pub tick: Tick,
    pub evaluations: Vec<Evaluation>,
    pub learn: Option<LearnStats>,
    pub homeo: Option<HomeoStats>,
    pub x_max_abs: f32,
    pub r_rms: f32,
    pub numerical_failure: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Agent {
    cfg: AgentConfig,
    n: usize,
    m: usize,
    x: Vec<f32>,
    r: Vec<f32>,
    tau: Vec<f32>,
    lam: Vec<f32>,
    w_in: Vec<f32>,
    w_rec: Vec<f32>,
    b: Vec<f32>,
    w_out: Vec<Vec<f32>>,
    c_out: Vec<Vec<f32>>,
    bfb: Vec<f32>,
    eps_in: Vec<f32>,
    eps_rec: Vec<f32>,
    o: Vec<f32>,
    pred_probs: Vec<Vec<f32>>,
    pred_made_at: Vec<Option<Tick>>,
    pred_last_obs: Vec<Tick>,
    hom_sumsq: Vec<f32>,
    hom_n: u64,
    hom_gain: Vec<f32>,
    w_g: Vec<f32>,
    w_gi: Vec<f32>,
    b_g: Vec<f32>,
    a_vec: Vec<f32>,
    failed: Option<Tick>,
    learn_hidden: bool,
    learn_readout: bool,
    homeostasis: bool,
    /// analysis only (gradcheck): accumulate the update instead of applying it
    dry: bool,
    dry_rec: Vec<f32>,
    dry_in: Vec<f32>,
    dry_out: Vec<Vec<f32>>,
    dry_cout: Vec<Vec<f32>>,
}

fn softmax_into(logits: &[f32], out: &mut Vec<f32>) {
    let mx = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    out.clear();
    let mut sum = 0.0;
    for &l in logits {
        let e = (l - mx).exp();
        out.push(e);
        sum += e;
    }
    for v in out.iter_mut() {
        *v /= sum;
    }
}

impl Agent {
    pub fn new(cfg: AgentConfig) -> Agent {
        let n = cfg.n;
        let m = OBS_DIM;
        let mut rng = Rng::new(cfg.seed);
        // RNG consumption order (fixed): tau, w_in, w_rec, w_out per block, bfb, LTC matrices
        let mut tau = Vec::with_capacity(n);
        for _ in 0..n {
            let u = rng.f32();
            tau.push(cfg.tau_min * (cfg.tau_max / cfg.tau_min).powf(u));
        }
        let lam: Vec<f32> = tau.iter().map(|&t| t / (t + 1.0)).collect();
        let mut col_gain = vec![cfg.g_in; m];
        for k in 0..NUM_BLOCKS {
            if BLOCK_HELD[k] {
                for q in 0..BLOCK_SIZES[k] {
                    col_gain[block_offset(k) + q] = cfg.g_in_held;
                }
            }
        }
        let w_in: Vec<f32> = (0..n * m).map(|idx| rng.normal() * col_gain[idx % m]).collect();
        let sd_rec = cfg.g_rec / (n as f32).sqrt();
        let w_rec: Vec<f32> = (0..n * n).map(|_| rng.normal() * sd_rec).collect();
        let sd_out = 1.0 / (n as f32).sqrt();
        let mut w_out = Vec::with_capacity(NUM_BLOCKS);
        let mut c_out = Vec::with_capacity(NUM_BLOCKS);
        for k in 0..NUM_BLOCKS {
            w_out.push((0..BLOCK_SIZES[k] * n).map(|_| rng.normal() * sd_out).collect::<Vec<f32>>());
            c_out.push(vec![0.0f32; BLOCK_SIZES[k]]);
        }
        let bfb: Vec<f32> = (0..n * m).map(|_| rng.normal() * sd_out).collect();
        let w_g: Vec<f32> = (0..n * n).map(|_| rng.normal() * sd_rec).collect();
        let w_gi: Vec<f32> = (0..n * m).map(|_| rng.normal() * 0.25).collect();
        let b_g = vec![-2.0f32; n];
        let a_vec: Vec<f32> = (0..n).map(|_| rng.normal()).collect();
        let pred_probs = (0..NUM_BLOCKS).map(|k| vec![1.0 / BLOCK_SIZES[k] as f32; BLOCK_SIZES[k]]).collect();
        Agent {
            learn_hidden: cfg.learn_hidden,
            learn_readout: cfg.learn_readout,
            homeostasis: cfg.homeostasis,
            cfg,
            n,
            m,
            x: vec![0.0; n],
            r: vec![0.0; n],
            tau,
            lam,
            w_in,
            w_rec,
            b: vec![0.0; n],
            w_out,
            c_out,
            bfb,
            eps_in: vec![0.0; n * m],
            eps_rec: vec![0.0; n * n],
            o: vec![0.0; m],
            pred_probs,
            pred_made_at: vec![None; NUM_BLOCKS],
            pred_last_obs: vec![0; NUM_BLOCKS],
            hom_sumsq: vec![0.0; n],
            hom_n: 0,
            hom_gain: vec![1.0; n],
            w_g,
            w_gi,
            b_g,
            a_vec,
            failed: None,
            dry: false,
            dry_rec: Vec::new(),
            dry_in: Vec::new(),
            dry_out: Vec::new(),
            dry_cout: Vec::new(),
        }
    }

    pub fn config(&self) -> &AgentConfig {
        &self.cfg
    }
    pub fn n(&self) -> usize {
        self.n
    }
    pub fn rates(&self) -> &[f32] {
        &self.r
    }
    pub fn state(&self) -> &[f32] {
        &self.x
    }
    pub fn predictions(&self) -> &[Vec<f32>] {
        &self.pred_probs
    }
    pub fn failed_at(&self) -> Option<Tick> {
        self.failed
    }
    pub fn tau(&self) -> &[f32] {
        &self.tau
    }
    pub fn homeostatic_gain(&self) -> &[f32] {
        &self.hom_gain
    }
    pub fn w_rec(&self) -> &[f32] {
        &self.w_rec
    }
    pub fn w_in(&self) -> &[f32] {
        &self.w_in
    }
    pub fn w_out(&self, k: usize) -> &[f32] {
        &self.w_out[k]
    }
    pub fn set_w_rec(&mut self, i: usize, j: usize, v: f32) {
        self.w_rec[i * self.n + j] = v;
    }
    pub fn set_w_in(&mut self, i: usize, j: usize, v: f32) {
        self.w_in[i * self.m + j] = v;
    }
    pub fn set_w_out(&mut self, k: usize, q: usize, i: usize, v: f32) {
        self.w_out[k][q * self.n + i] = v;
    }
    pub fn c_out(&self, k: usize) -> &[f32] {
        &self.c_out[k]
    }
    pub fn set_c_out(&mut self, k: usize, q: usize, v: f32) {
        self.c_out[k][q] = v;
    }
    /// Analysis only (gradcheck): change the learning rates of a copy.
    pub fn set_rates(&mut self, eta_hidden: f32, eta_out: f32) {
        self.cfg.eta_hidden = eta_hidden;
        self.cfg.eta_out = eta_out;
    }
    /// Analysis only (gradcheck): accumulate the plasticity update of every evaluation tick into
    /// separate buffers instead of applying it; weights stay fixed, traces still advance.
    pub fn set_dry_run(&mut self, on: bool) {
        self.dry = on;
        if on {
            self.dry_rec = vec![0.0; self.n * self.n];
            self.dry_in = vec![0.0; self.n * self.m];
            self.dry_out = (0..NUM_BLOCKS).map(|k| vec![0.0; BLOCK_SIZES[k] * self.n]).collect();
            self.dry_cout = (0..NUM_BLOCKS).map(|k| vec![0.0; BLOCK_SIZES[k]]).collect();
        }
    }
    pub fn dry_update(&self) -> (&[f32], &[f32], &[Vec<f32>], &[Vec<f32>]) {
        (&self.dry_rec, &self.dry_in, &self.dry_out, &self.dry_cout)
    }

    /// Analysis copies only: zero the fast state, traces, held inputs and stored predictions.
    pub fn reset_state(&mut self) {
        for v in self.x.iter_mut() {
            *v = 0.0;
        }
        for v in self.r.iter_mut() {
            *v = 0.0;
        }
        for v in self.eps_in.iter_mut() {
            *v = 0.0;
        }
        for v in self.eps_rec.iter_mut() {
            *v = 0.0;
        }
        for v in self.o.iter_mut() {
            *v = 0.0;
        }
        for k in 0..NUM_BLOCKS {
            self.pred_made_at[k] = None;
        }
        for v in self.hom_sumsq.iter_mut() {
            *v = 0.0;
        }
        self.hom_n = 0;
    }

    pub fn set_learning(&mut self, hidden: bool, readout: bool, homeostasis: bool) {
        self.learn_hidden = hidden;
        self.learn_readout = readout;
        self.homeostasis = homeostasis;
    }
    pub fn learning(&self) -> (bool, bool, bool) {
        (self.learn_hidden, self.learn_readout, self.homeostasis)
    }

    pub fn state_hash(&self) -> u64 {
        hash_f32s(HASH_SEED, &self.x)
    }
    pub fn weights_hash(&self) -> u64 {
        let mut h = hash_f32s(HASH_SEED, &self.w_in);
        h = hash_f32s(h, &self.w_rec);
        h = hash_f32s(h, &self.b);
        for k in 0..NUM_BLOCKS {
            h = hash_f32s(h, &self.w_out[k]);
            h = hash_f32s(h, &self.c_out[k]);
        }
        h
    }

    /// One tick: evaluate -> learn -> incorporate -> predict -> (homeostasis).
    pub fn tick(&mut self, obs: &Observation) -> TickReport {
        let t = obs.tick;
        let n = self.n;
        let m = self.m;
        let mut rep = TickReport { tick: t, ..Default::default() };
        if self.failed.is_some() {
            rep.numerical_failure = true;
            return rep;
        }
        if let Some(f) = self.cfg.freeze_at {
            if t >= f {
                self.learn_hidden = false;
                self.learn_readout = false;
                self.homeostasis = false;
            }
        }

        // ---- 1. evaluate stored predictions against the events that arrived
        let mut e = vec![0f32; m];
        let mut active = [false; NUM_BLOCKS];
        for ev in &obs.events {
            let k = ev.block as usize;
            let v = ev.value as usize;
            if let Some(made) = self.pred_made_at[k] {
                assert!(made < t, "prediction time-order violated: made_at {} >= tick {}", made, t);
                let p = &self.pred_probs[k];
                let off = block_offset(k);
                let sz = BLOCK_SIZES[k];
                let pv = p[v].max(1e-7);
                let mut h = 0.0f32;
                for q in 0..sz {
                    if p[q] > 0.0 {
                        h -= p[q] * p[q].log2();
                    }
                }
                for q in 0..sz {
                    e[off + q] = (if q == v { 1.0 } else { 0.0 }) - p[q];
                }
                active[k] = true;
                rep.evaluations.push(Evaluation { block: ev.block, value: ev.value, made_at: made, last_obs_tick: self.pred_last_obs[k], surprise_bits: -pv.log2(), p_of_value: pv, entropy_bits: h, probs: p.clone() });
            }
        }

        // ---- 2. learn from the errors (uses r(t-1) and traces up to x(t-1))
        if active.iter().any(|&a| a) && (self.learn_readout || self.learn_hidden) {
            let mut st = LearnStats::default();
            if self.learn_hidden {
                let mut l = vec![0f32; n];
                match self.cfg.feedback {
                    Feedback::Random => {
                        for i in 0..n {
                            let row = &self.bfb[i * m..(i + 1) * m];
                            let mut s = 0.0f32;
                            for k in 0..NUM_BLOCKS {
                                if active[k] {
                                    let off = block_offset(k);
                                    for q in 0..BLOCK_SIZES[k] {
                                        s += row[off + q] * e[off + q];
                                    }
                                }
                            }
                            l[i] = s;
                        }
                    }
                    Feedback::WeightTransport => {
                        for k in 0..NUM_BLOCKS {
                            if !active[k] {
                                continue;
                            }
                            let off = block_offset(k);
                            let w = &self.w_out[k];
                            for q in 0..BLOCK_SIZES[k] {
                                let eq = e[off + q];
                                if eq != 0.0 {
                                    for i in 0..n {
                                        l[i] += w[q * n + i] * eq;
                                    }
                                }
                            }
                        }
                    }
                }
                st.l_norm2 = l.iter().map(|v| v * v).sum();
                let wmax = self.cfg.w_max_hidden;
                let dry = self.dry;
                for i in 0..n {
                    let g = l[i] * (1.0 - self.r[i] * self.r[i]);
                    if g == 0.0 {
                        continue;
                    }
                    let eg = self.cfg.eta_hidden * g;
                    let eps_r = &self.eps_rec[i * n..(i + 1) * n];
                    if dry {
                        let drow = &mut self.dry_rec[i * n..(i + 1) * n];
                        for j in 0..n {
                            drow[j] += eg * eps_r[j];
                        }
                        let eps_i = &self.eps_in[i * m..(i + 1) * m];
                        let drow = &mut self.dry_in[i * m..(i + 1) * m];
                        for j in 0..m {
                            drow[j] += eg * eps_i[j];
                        }
                        continue;
                    }
                    let row_r = &mut self.w_rec[i * n..(i + 1) * n];
                    for j in 0..n {
                        let d = eg * eps_r[j];
                        let w = &mut row_r[j];
                        *w += d;
                        st.dw_rec2 += d * d;
                        if w.abs() > wmax {
                            *w = w.signum() * wmax;
                            st.clips_hidden += 1;
                        }
                    }
                    let row_i = &mut self.w_in[i * m..(i + 1) * m];
                    let eps_i = &self.eps_in[i * m..(i + 1) * m];
                    for j in 0..m {
                        let d = eg * eps_i[j];
                        let w = &mut row_i[j];
                        *w += d;
                        st.dw_in2 += d * d;
                        if w.abs() > wmax {
                            *w = w.signum() * wmax;
                            st.clips_hidden += 1;
                        }
                    }
                }
            }
            if self.learn_readout {
                let wmax = self.cfg.w_max_out;
                let dry = self.dry;
                for k in 0..NUM_BLOCKS {
                    if !active[k] {
                        continue;
                    }
                    let off = block_offset(k);
                    for q in 0..BLOCK_SIZES[k] {
                        let eq = self.cfg.eta_out * e[off + q];
                        if dry {
                            self.dry_cout[k][q] += eq;
                            let drow = &mut self.dry_out[k][q * n..(q + 1) * n];
                            for i in 0..n {
                                drow[i] += eq * self.r[i];
                            }
                            continue;
                        }
                        self.c_out[k][q] += eq;
                        let row = &mut self.w_out[k][q * n..(q + 1) * n];
                        for i in 0..n {
                            let d = eq * self.r[i];
                            let w = &mut row[i];
                            *w += d;
                            st.dw_out2 += d * d;
                            if w.abs() > wmax {
                                *w = w.signum() * wmax;
                                st.clips_out += 1;
                            }
                        }
                    }
                }
            }
            rep.learn = Some(st);
        }

        // ---- 3. incorporate the observation into the state
        for k in 0..NUM_BLOCKS {
            if !BLOCK_HELD[k] {
                let off = block_offset(k);
                for q in 0..BLOCK_SIZES[k] {
                    self.o[off + q] = 0.0;
                }
            }
        }
        for ev in &obs.events {
            let k = ev.block as usize;
            let off = block_offset(k);
            if BLOCK_HELD[k] {
                for q in 0..BLOCK_SIZES[k] {
                    self.o[off + q] = 0.0;
                }
            }
            self.o[off + ev.value as usize] = 1.0;
        }
        let active_in: Vec<usize> = (0..m).filter(|&j| self.o[j] != 0.0).collect();
        let r_prev = self.r.clone();
        let mut u = vec![0f32; n];
        for i in 0..n {
            let row = &self.w_rec[i * n..(i + 1) * n];
            let mut s = self.b[i];
            for j in 0..n {
                s += row[j] * r_prev[j];
            }
            for &j in &active_in {
                s += self.w_in[i * m + j] * self.o[j];
            }
            u[i] = s;
        }
        let mut x_new = vec![0f32; n];
        if self.cfg.ltc {
            for i in 0..n {
                let row = &self.w_g[i * n..(i + 1) * n];
                let mut z = self.b_g[i];
                for j in 0..n {
                    z += row[j] * r_prev[j];
                }
                for &j in &active_in {
                    z += self.w_gi[i * m + j] * self.o[j];
                }
                let g = 1.0 / (1.0 + (-z).exp());
                let inv_tau = 1.0 / self.tau[i];
                x_new[i] = (self.x[i] + u[i] * inv_tau + g * self.a_vec[i]) / (1.0 + inv_tau + g);
            }
        } else {
            for i in 0..n {
                let lam = self.lam[i];
                x_new[i] = lam * self.x[i] + (1.0 - lam) * u[i];
            }
        }
        if self.learn_hidden {
            for i in 0..n {
                let lam = self.lam[i];
                let oml = 1.0 - lam;
                let row = &mut self.eps_rec[i * n..(i + 1) * n];
                for j in 0..n {
                    row[j] = lam * row[j] + oml * r_prev[j];
                }
                let row = &mut self.eps_in[i * m..(i + 1) * m];
                for v in row.iter_mut() {
                    *v *= lam;
                }
                for &j in &active_in {
                    row[j] += oml * self.o[j];
                }
            }
        }
        let mut xmax = 0.0f32;
        let mut bad = false;
        for &v in &x_new {
            if !v.is_finite() {
                bad = true;
            }
            xmax = xmax.max(v.abs());
        }
        if bad {
            self.failed = Some(t);
            rep.numerical_failure = true;
            return rep;
        }
        self.x = x_new;
        let mut ss = 0.0f32;
        for i in 0..n {
            self.r[i] = self.x[i].tanh();
            ss += self.r[i] * self.r[i];
        }
        rep.x_max_abs = xmax;
        rep.r_rms = (ss / n as f32).sqrt();

        // ---- 4. predict the next event of every block
        let mut logits: Vec<f32> = Vec::with_capacity(16);
        for k in 0..NUM_BLOCKS {
            let sz = BLOCK_SIZES[k];
            logits.clear();
            for q in 0..sz {
                let row = &self.w_out[k][q * n..(q + 1) * n];
                let mut s = self.c_out[k][q];
                for i in 0..n {
                    s += row[i] * self.r[i];
                }
                logits.push(s);
            }
            softmax_into(&logits, &mut self.pred_probs[k]);
            self.pred_made_at[k] = Some(t);
            self.pred_last_obs[k] = t;
        }

        // ---- 5. activity regulation (innate; not learning; logged separately)
        for i in 0..n {
            self.hom_sumsq[i] += self.r[i] * self.r[i];
        }
        self.hom_n += 1;
        if self.homeostasis && t % self.cfg.t_h == 0 && self.hom_n > 0 {
            let mut rms = vec![0f32; n];
            let mut scale = vec![1f32; n];
            let mut dw2 = 0.0f32;
            let mut saturated = 0u32;
            let mut clips = 0u32;
            let wmax = self.cfg.w_max_hidden;
            let hmax = self.cfg.h_max;
            for i in 0..n {
                rms[i] = (self.hom_sumsq[i] / self.hom_n as f32).sqrt();
                let mut s = (1.0 + self.cfg.eta_h * (self.cfg.rms_target - rms[i])).clamp(0.8, 1.25);
                // bounded cumulative gain: a unit that cannot reach the target does not grow forever
                let g = self.hom_gain[i] * s;
                if g > hmax {
                    s = hmax / self.hom_gain[i];
                    saturated += 1;
                } else if g < 1.0 / hmax {
                    s = (1.0 / hmax) / self.hom_gain[i];
                    saturated += 1;
                }
                self.hom_gain[i] *= s;
                scale[i] = s;
                if s != 1.0 {
                    for j in 0..n {
                        let w = &mut self.w_rec[i * n + j];
                        let mut nw = *w * s;
                        if nw.abs() > wmax {
                            nw = nw.signum() * wmax;
                            clips += 1;
                        }
                        let d = nw - *w;
                        dw2 += d * d;
                        *w = nw;
                    }
                    for j in 0..m {
                        let w = &mut self.w_in[i * m + j];
                        let mut nw = *w * s;
                        if nw.abs() > wmax {
                            nw = nw.signum() * wmax;
                            clips += 1;
                        }
                        let d = nw - *w;
                        dw2 += d * d;
                        *w = nw;
                    }
                }
                self.hom_sumsq[i] = 0.0;
            }
            self.hom_n = 0;
            rep.homeo = Some(HomeoStats { rms, scale, dw2, saturated, clips });
        }
        rep
    }
}
