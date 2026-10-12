//! life-world: the ring world of docs/CONTINUOUS-WORLD-SPEC.md section 2 and its control variants.
//!
//! Randomness is split into independent streams: `rng_time` (emission intervals, mode switches,
//! structural-change schedule), `rng_sym` (grammar draws and symbols), `rng_noise` (sensor noise).
//! Given the same seeds and the same action sequence, the observation stream is identical. The
//! truth log (`TruthEvent`) is for the analyzer only; nothing in it reaches the agent.

use life_interface::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const K: usize = 8;
pub const A: usize = 6;
const DUR: [u64; 5] = [2, 6, 6, 10, 20];
const COST: [i32; 5] = [1, 6, 6, 10, 0];
const REST_GAIN: i32 = 8;
const INTERO_PERIOD: u64 = 20;
const INTERO_BUCKETS: i32 = 16;
const SHUFFLE_BLOCK: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Variant {
    Structured,
    Shuffled,
    Iid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldConfig {
    pub seed_world: u64,
    pub seed_noise: u64,
    pub variant: Variant,
    pub label_perm_seed: Option<u64>,
    pub noise_here: f32,
    pub noise_near: f32,
    pub drop_near: f32,
    pub mode_dwell: f32,
    pub change_min: u64,
    pub change_max: u64,
    pub forced_ticks: u64,
    pub forced_interval: u64,
    pub rescue_ticks: u64,
    pub resource_max: i32,
    pub rescue_resource: i32,
}

impl WorldConfig {
    pub fn new(seed_world: u64, seed_noise: u64, variant: Variant) -> WorldConfig {
        WorldConfig {
            seed_world,
            seed_noise,
            variant,
            label_perm_seed: None,
            noise_here: 0.04,
            noise_near: 0.08,
            drop_near: 0.10,
            mode_dwell: 3000.0,
            change_min: 150_000,
            change_max: 250_000,
            forced_ticks: 100,
            forced_interval: 5,
            rescue_ticks: 2000,
            resource_max: 1000,
            rescue_resource: 500,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Grammar {
    /// row = context (a * A + b) -> distribution over the next symbol
    table: Vec<[f32; A]>,
}

fn draw_grammar(rng: &mut Rng) -> Grammar {
    let mut table = Vec::with_capacity(A * A);
    for _ in 0..A * A {
        let p = rng.below(A as u64) as usize;
        let mut d = [0.05f32; A];
        d[p] = 0.75;
        table.push(d);
    }
    Grammar { table }
}

fn sample(d: &[f32; A], rng: &mut Rng) -> u8 {
    let u = rng.f32();
    let mut acc = 0.0;
    for (i, p) in d.iter().enumerate() {
        acc += p;
        if u < acc {
            return i as u8;
        }
    }
    (A - 1) as u8
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Site {
    g0: Grammar,
    g1: Option<Grammar>,
    /// (source site, map from the source's current symbol, strength)
    cross: Option<(usize, [u8; A], f32)>,
    mean_interval: u64,
    last2: (u8, u8),
    next_emit: Tick,
    forced_until: Tick,
    cycle: [u8; 3],
    cycle_pos: u8,
    buffer: VecDeque<u8>,
    marginal: [f32; A],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TruthEvent {
    Emission { tick: Tick, site: u8, symbol: u8, forced: bool, mode: u8 },
    ModeSwitch { tick: Tick, mode: u8 },
    StructuralChange { tick: Tick, sites: Vec<u8>, cross_moved_from: Option<u8>, cross_moved_to: Option<u8> },
    Misread { tick: Tick, block: u8, true_value: u8, shown: u8 },
    Dropped { tick: Tick, block: u8 },
    MaintenanceFailure { tick: Tick },
    ExternalRescue { tick: Tick },
    Resumption { tick: Tick },
    ActionStart { tick: Tick, action: u8, resource: i32 },
    ActionDone { tick: Tick, action: u8, pos: u8, resource: i32 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    cfg: WorldConfig,
    rng_time: Rng,
    rng_sym: Rng,
    rng_noise: Rng,
    sites: Vec<Site>,
    mode: u8,
    next_change: Tick,
    pos: usize,
    resource: i32,
    /// (action index, completion tick)
    action: Option<(u8, Tick)>,
    incapacitated_until: Option<Tick>,
    resumed_pending: bool,
    tick: Tick,
    delayed: Vec<(Tick, u8, u8)>,
    label_perm: Vec<Vec<u8>>,
    truth: Vec<TruthEvent>,
    pub changes: u32,
}

impl World {
    pub fn new(cfg: WorldConfig) -> World {
        let mut rng_time = Rng::new(cfg.seed_world);
        let mut rng_sym = rng_time.fork(1);
        let rng_noise = Rng::new(cfg.seed_noise);
        let mut sites = Vec::with_capacity(K);
        for _ in 0..K {
            let g0 = draw_grammar(&mut rng_sym);
            let mean_interval = 8 + rng_time.below(33);
            let cycle = [rng_sym.below(A as u64) as u8, rng_sym.below(A as u64) as u8, rng_sym.below(A as u64) as u8];
            let last2 = (rng_sym.below(A as u64) as u8, rng_sym.below(A as u64) as u8);
            let first = 1 + rng_time.below(mean_interval);
            sites.push(Site { g0, g1: None, cross: None, mean_interval, last2, next_emit: first, forced_until: 0, cycle, cycle_pos: 0, buffer: VecDeque::new(), marginal: [1.0 / A as f32; A] });
        }
        // three mode-dependent sites
        let mut idx: Vec<usize> = (0..K).collect();
        rng_sym.shuffle(&mut idx);
        for &s in &idx[..3] {
            sites[s].g1 = Some(draw_grammar(&mut rng_sym));
        }
        // two cross-site relations (each depends on its right-hand neighbour)
        for &s in &idx[3..5] {
            let mut map = [0u8; A];
            for m in map.iter_mut() {
                *m = rng_sym.below(A as u64) as u8;
            }
            sites[s].cross = Some(((s + 1) % K, map, 0.6));
        }
        // approximate per-site marginal (own grammar, mode 0, no cross) for the iid control
        for s in 0..K {
            let mut r = rng_sym.fork(100 + s as u64);
            let mut counts = [0u32; A];
            let mut ctx = sites[s].last2;
            for _ in 0..3000 {
                let d = sites[s].g0.table[ctx.0 as usize * A + ctx.1 as usize];
                let v = sample(&d, &mut r);
                counts[v as usize] += 1;
                ctx = (ctx.1, v);
            }
            for i in 0..A {
                sites[s].marginal[i] = counts[i] as f32 / 3000.0;
            }
        }
        let next_change = cfg.change_min + rng_time.below(cfg.change_max - cfg.change_min + 1);
        let label_perm = match cfg.label_perm_seed {
            Some(seed) => {
                let mut r = Rng::new(seed);
                (0..NUM_BLOCKS)
                    .map(|k| {
                        let mut v: Vec<u8> = (0..BLOCK_SIZES[k] as u8).collect();
                        if k <= 3 {
                            r.shuffle(&mut v);
                        }
                        v
                    })
                    .collect()
            }
            None => Vec::new(),
        };
        let resource = cfg.resource_max;
        World { cfg, rng_time, rng_sym, rng_noise, sites, mode: 0, next_change, pos: 0, resource, action: None, incapacitated_until: None, resumed_pending: false, tick: 0, delayed: Vec::new(), label_perm, truth: Vec::new(), changes: 0 }
    }

    pub fn tick(&self) -> Tick {
        self.tick
    }
    pub fn pos(&self) -> usize {
        self.pos
    }
    pub fn resource(&self) -> i32 {
        self.resource
    }
    pub fn busy(&self) -> bool {
        self.action.is_some()
    }
    pub fn incapacitated(&self) -> bool {
        self.incapacitated_until.is_some()
    }
    pub fn config(&self) -> &WorldConfig {
        &self.cfg
    }
    /// Analyzer only: drain the truth log.
    pub fn take_truth(&mut self) -> Vec<TruthEvent> {
        std::mem::take(&mut self.truth)
    }
    /// Analyzer only: give this (cloned) world its own randomness, for probe streams that are
    /// plausible continuations but never the main life's future.
    pub fn fork_rngs(&mut self, seed: u64) {
        let mut r = Rng::new(seed);
        self.rng_time = r.fork(1);
        self.rng_sym = r.fork(2);
        self.rng_noise = r.fork(3);
        self.truth.clear();
    }
    /// Analyzer only: the true next-symbol distribution at `site` after context (a, b) under the
    /// current mode, ignoring cross-site influence.
    pub fn true_next_dist(&self, site: usize, a: u8, b: u8) -> [f32; A] {
        let s = &self.sites[site];
        let g = if self.mode == 1 && s.g1.is_some() { s.g1.as_ref().unwrap() } else { &s.g0 };
        g.table[a as usize * A + b as usize]
    }
    /// Analyzer only: as `true_next_dist` but under an explicit mode.
    pub fn true_next_dist_mode(&self, site: usize, a: u8, b: u8, mode: u8) -> [f32; A] {
        let s = &self.sites[site];
        let g = if mode == 1 && s.g1.is_some() { s.g1.as_ref().unwrap() } else { &s.g0 };
        g.table[a as usize * A + b as usize]
    }
    pub fn mode(&self) -> u8 {
        self.mode
    }
    /// Analyzer only: the sites whose grammar depends on the hidden mode.
    pub fn mode_sites(&self) -> Vec<u8> {
        (0..K).filter(|&s| self.sites[s].g1.is_some()).map(|s| s as u8).collect()
    }
    /// Analyzer only: the last two true symbols of a site.
    pub fn last2(&self, site: usize) -> (u8, u8) {
        self.sites[site].last2
    }
    /// Analyzer only: invert the label permutation of a shown value.
    pub fn unpermute(&self, block: u8, shown: u8) -> u8 {
        if self.label_perm.is_empty() {
            return shown;
        }
        let p = &self.label_perm[block as usize];
        p.iter().position(|&v| v == shown).map(|i| i as u8).unwrap_or(shown)
    }

    fn bucket(&self) -> u8 {
        let b = (self.resource.max(0) as i64 * INTERO_BUCKETS as i64 / (self.cfg.resource_max as i64 + 1)) as i32;
        b.clamp(0, INTERO_BUCKETS - 1) as u8
    }

    fn grammar_next(&mut self, s: usize) -> u8 {
        let (a, b) = self.sites[s].last2;
        let use1 = self.mode == 1 && self.sites[s].g1.is_some();
        let dist = if use1 { self.sites[s].g1.as_ref().unwrap().table[a as usize * A + b as usize] } else { self.sites[s].g0.table[a as usize * A + b as usize] };
        let mut sym = sample(&dist, &mut self.rng_sym);
        if let Some((from, map, strength)) = self.sites[s].cross {
            if self.rng_sym.f32() < strength {
                sym = map[self.sites[from].last2.1 as usize];
            }
        }
        self.sites[s].last2 = (b, sym);
        sym
    }

    fn emit(&mut self, s: usize, t: Tick) -> (u8, bool) {
        let forced = self.sites[s].forced_until > t;
        let sym = if forced {
            let site = &mut self.sites[s];
            let v = site.cycle[(site.cycle_pos % 3) as usize];
            site.cycle_pos = (site.cycle_pos + 1) % 3;
            v
        } else {
            match self.cfg.variant {
                Variant::Structured => self.grammar_next(s),
                Variant::Shuffled => {
                    if self.sites[s].buffer.is_empty() {
                        let mut buf: Vec<u8> = (0..SHUFFLE_BLOCK).map(|_| self.grammar_next(s)).collect();
                        self.rng_sym.shuffle(&mut buf);
                        self.sites[s].buffer.extend(buf);
                    }
                    self.sites[s].buffer.pop_front().unwrap()
                }
                Variant::Iid => {
                    let m = self.sites[s].marginal;
                    sample(&m, &mut self.rng_sym)
                }
            }
        };
        let interval = if forced {
            self.cfg.forced_interval
        } else {
            let m = self.sites[s].mean_interval;
            let lo = m - m / 2;
            let hi = m + m / 2;
            lo + self.rng_time.below(hi - lo + 1)
        }
        .max(2);
        self.sites[s].next_emit = t + interval;
        (sym, forced)
    }

    fn structural_change(&mut self, t: Tick) {
        let mut changed = Vec::new();
        for _ in 0..2 {
            let s = self.rng_sym.below(K as u64) as usize;
            self.sites[s].g0 = draw_grammar(&mut self.rng_sym);
            if self.sites[s].g1.is_some() {
                self.sites[s].g1 = Some(draw_grammar(&mut self.rng_sym));
            }
            self.sites[s].buffer.clear();
            if !changed.contains(&(s as u8)) {
                changed.push(s as u8);
            }
        }
        let holders: Vec<usize> = (0..K).filter(|&s| self.sites[s].cross.is_some()).collect();
        let mut moved_to = None;
        let mut moved_from = None;
        if let Some(&h) = holders.first() {
            moved_from = Some(h as u8);
            let cross = self.sites[h].cross.take();
            let mut target = self.rng_sym.below(K as u64) as usize;
            while target == h || self.sites[target].cross.is_some() {
                target = (target + 1) % K;
            }
            self.sites[target].cross = cross.map(|(_, map, st)| ((target + 1) % K, map, st));
            moved_to = Some(target as u8);
        }
        self.changes += 1;
        self.truth.push(TruthEvent::StructuralChange { tick: t, sites: changed, cross_moved_from: moved_from, cross_moved_to: moved_to });
        self.next_change = t + self.cfg.change_min + self.rng_time.below(self.cfg.change_max - self.cfg.change_min + 1);
    }

    fn noisy_here(&mut self, t: Tick, sym: u8) -> u8 {
        if self.rng_noise.f32() < self.cfg.noise_here {
            let shown = ((sym as u64 + 1 + self.rng_noise.below(A as u64 - 1)) % A as u64) as u8;
            self.truth.push(TruthEvent::Misread { tick: t, block: 0, true_value: sym, shown });
            shown
        } else {
            sym
        }
    }

    fn noisy_near(&mut self, t: Tick, block: u8, sym: u8) -> Option<u8> {
        if self.rng_noise.f32() < self.cfg.drop_near {
            self.truth.push(TruthEvent::Dropped { tick: t, block });
            return None;
        }
        if self.rng_noise.f32() < self.cfg.noise_near {
            let shown = ((sym as u64 + 1 + self.rng_noise.below(A as u64 - 1)) % A as u64) as u8;
            self.truth.push(TruthEvent::Misread { tick: t, block, true_value: sym, shown });
            Some(shown)
        } else {
            Some(sym)
        }
    }

    fn complete_action(&mut self, a: u8, t: Tick, events: &mut Vec<ObsEvent>) {
        let left = (self.pos + K - 1) % K;
        let right = (self.pos + 1) % K;
        match a {
            0 => {
                let l = self.sites[left].last2.1;
                let r = self.sites[right].last2.1;
                if let Some(v) = self.noisy_near(t, 1, l) {
                    events.push(ObsEvent { block: 1, value: v });
                }
                if let Some(v) = self.noisy_near(t, 2, r) {
                    events.push(ObsEvent { block: 2, value: v });
                }
            }
            1 => {
                self.pos = left;
                events.push(ObsEvent { block: 3, value: self.pos as u8 });
            }
            2 => {
                self.pos = right;
                events.push(ObsEvent { block: 3, value: self.pos as u8 });
            }
            3 => {
                let s = self.pos;
                self.sites[s].forced_until = t + self.cfg.forced_ticks;
                self.sites[s].next_emit = t + self.cfg.forced_interval;
                self.sites[s].cycle_pos = 0;
            }
            _ => {}
        }
        events.push(ObsEvent { block: 5, value: a });
        events.push(ObsEvent { block: 4, value: self.bucket() });
        self.truth.push(TruthEvent::ActionDone { tick: t, action: a, pos: self.pos as u8, resource: self.resource });
    }

    /// Request an action at tick `t` (before `step()` advances to `t`). Refused while another
    /// action runs or the body is incapacitated.
    pub fn request_action(&mut self, a: Action, t: Tick) -> Result<Tick, ()> {
        if self.action.is_some() || self.incapacitated_until.is_some() {
            return Err(());
        }
        let i = a.index() as usize;
        self.resource -= COST[i];
        let done = t + DUR[i];
        self.action = Some((a.index(), done));
        if self.resumed_pending {
            self.resumed_pending = false;
            self.truth.push(TruthEvent::Resumption { tick: t });
        }
        self.truth.push(TruthEvent::ActionStart { tick: t, action: a.index(), resource: self.resource });
        Ok(done)
    }

    /// Advance one tick and return what the agent observes.
    pub fn step(&mut self) -> Observation {
        let t = self.tick + 1;
        self.tick = t;
        let mut events: Vec<ObsEvent> = Vec::new();
        if t == 1 {
            events.push(ObsEvent { block: 3, value: self.pos as u8 });
        }
        if self.rng_time.f32() < 1.0 / self.cfg.mode_dwell {
            self.mode ^= 1;
            self.truth.push(TruthEvent::ModeSwitch { tick: t, mode: self.mode });
        }
        if t == self.next_change {
            self.structural_change(t);
        }
        // body
        if let Some(until) = self.incapacitated_until {
            if t >= until {
                self.incapacitated_until = None;
                self.resource = self.cfg.rescue_resource;
                self.resumed_pending = true;
                self.truth.push(TruthEvent::ExternalRescue { tick: t });
            }
        } else {
            self.resource -= 1;
            if let Some((a, done)) = self.action {
                if a == Action::Rest.index() && done >= t {
                    self.resource += REST_GAIN;
                }
            }
            self.resource = self.resource.min(self.cfg.resource_max);
            if self.resource <= 0 {
                self.resource = 0;
                self.action = None;
                self.incapacitated_until = Some(t + self.cfg.rescue_ticks);
                self.truth.push(TruthEvent::MaintenanceFailure { tick: t });
            }
        }
        // action completion
        if let Some((a, done)) = self.action {
            if done == t {
                self.action = None;
                self.complete_action(a, t, &mut events);
            }
        }
        // emissions
        let pos = self.pos;
        let left = (pos + K - 1) % K;
        let right = (pos + 1) % K;
        for s in 0..K {
            if self.sites[s].next_emit == t {
                let (sym, forced) = self.emit(s, t);
                self.truth.push(TruthEvent::Emission { tick: t, site: s as u8, symbol: sym, forced, mode: self.mode });
                if s == pos {
                    let shown = self.noisy_here(t, sym);
                    events.push(ObsEvent { block: 0, value: shown });
                }
                if s == left {
                    self.delayed.push((t + 1, 1, sym));
                }
                if s == right {
                    self.delayed.push((t + 1, 2, sym));
                }
            }
        }
        let mut due: Vec<(Tick, u8, u8)> = self.delayed.iter().copied().filter(|d| d.0 == t).collect();
        due.sort();
        self.delayed.retain(|d| d.0 != t);
        for (_, blk, sym) in due {
            if let Some(v) = self.noisy_near(t, blk, sym) {
                events.push(ObsEvent { block: blk, value: v });
            }
        }
        if t % INTERO_PERIOD == 0 {
            events.push(ObsEvent { block: 4, value: self.bucket() });
        }
        if !self.label_perm.is_empty() {
            for ev in events.iter_mut() {
                let p = &self.label_perm[ev.block as usize];
                ev.value = p[ev.value as usize];
            }
        }
        Observation { tick: t, events }
    }
}
