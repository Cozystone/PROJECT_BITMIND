//! life-run: wires one life (world + agent + external action sampler), writes traces and
//! checkpoints, and supports analysis branches from checkpoints. Checkpoints and branches never
//! feed back into the main life.

use life_agent::{Agent, AgentConfig, Feedback, TickReport};
use life_interface::*;
use life_trace::{Rec, Writer};
use life_world::{TruthEvent, Variant, World, WorldConfig};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const ARMS: [&str; 7] = ["full", "readout", "frozen", "homeo", "freeze_at", "fb_wt", "ltc"];

/// Learning flags per arm (docs/APPENDIX-FIRST-EXPERIMENT.md A3).
pub fn arm_agent_config(arm: &str, base: &AgentConfig) -> AgentConfig {
    let mut c = base.clone();
    c.learn_hidden = true;
    c.learn_readout = true;
    c.homeostasis = true;
    c.feedback = Feedback::Random;
    c.ltc = false;
    c.freeze_at = None;
    match arm {
        "full" => {}
        "readout" => c.learn_hidden = false,
        "frozen" => {
            c.learn_hidden = false;
            c.learn_readout = false;
            c.homeostasis = false;
        }
        "homeo" => {
            c.learn_hidden = false;
            c.learn_readout = false;
        }
        "freeze_at" => c.freeze_at = Some(200_000),
        "fb_wt" => c.feedback = Feedback::WeightTransport,
        "ltc" => c.ltc = true,
        other => panic!("unknown arm {other}"),
    }
    c
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunConfig {
    pub out_dir: PathBuf,
    pub arm: String,
    pub world: WorldConfig,
    pub agent: AgentConfig,
    pub seed_action: u64,
    pub ticks: u64,
    pub checkpoint_every: u64,
    pub state_every: u64,
    pub trace: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunSummary {
    pub arm: String,
    pub ticks: u64,
    pub status: String,
    pub input_hash: u64,
    pub action_hash: u64,
    pub weights_hash: u64,
    pub state_hash: u64,
    pub elapsed_s: f64,
    pub ticks_per_s: f64,
    pub trace_records: u64,
    pub trace_bytes: u64,
    pub evaluations: u64,
    pub maintenance_failures: u64,
    pub rescues: u64,
    pub structural_changes: u64,
    pub numerical_failure_at: Option<u64>,
}

/// A life: world, agent and the external action sampler, advanced one tick at a time.
#[derive(Clone, Serialize, Deserialize)]
pub struct Life {
    pub world: World,
    pub agent: Agent,
    pub sampler: Rng,
    pub tick: Tick,
    pub input_hash: u64,
    pub action_hash: u64,
    pub evaluations: u64,
}

pub struct StepOut {
    pub obs: Observation,
    pub report: TickReport,
    pub action: Option<(u8, Tick)>,
}

impl Life {
    pub fn new(world: World, agent: Agent, seed_action: u64) -> Life {
        Life { world, agent, sampler: Rng::new(seed_action), tick: 0, input_hash: HASH_SEED, action_hash: HASH_SEED, evaluations: 0 }
    }

    /// One tick: (external) action request if idle, world step, agent tick.
    pub fn step(&mut self) -> StepOut {
        let t = self.tick + 1;
        let mut action = None;
        if !self.world.busy() && !self.world.incapacitated() {
            let a = Action::ALL[self.sampler.below(5) as usize];
            if let Ok(done) = self.world.request_action(a, t) {
                self.action_hash = hash_mix(self.action_hash, (t << 8) | a.index() as u64);
                action = Some((a.index(), done));
            }
        }
        let obs = self.world.step();
        debug_assert_eq!(obs.tick, t);
        for ev in &obs.events {
            self.input_hash = hash_mix(self.input_hash, (t << 16) | ((ev.block as u64) << 8) | ev.value as u64);
        }
        let report = self.agent.tick(&obs);
        self.evaluations += report.evaluations.len() as u64;
        self.tick = t;
        StepOut { obs, report, action }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Checkpoint {
    pub life: Life,
    pub truth_len: usize,
}

pub fn checkpoint_path(dir: &Path, tick: Tick) -> PathBuf {
    dir.join(format!("ckpt_{tick}.bin"))
}

pub fn save_checkpoint(dir: &Path, life: &Life, truth_len: usize) -> io::Result<PathBuf> {
    let p = checkpoint_path(dir, life.tick);
    let f = fs::File::create(&p)?;
    let mut w = io::BufWriter::new(f);
    bincode::serialize_into(&mut w, &Checkpoint { life: life.clone(), truth_len }).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(p)
}

pub fn load_checkpoint(path: &Path) -> io::Result<Checkpoint> {
    let f = fs::File::open(path)?;
    let r = io::BufReader::new(f);
    bincode::deserialize_from(r).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

/// Advance a life without tracing (deterministic replay).
pub fn replay_to(life: &mut Life, target: Tick) {
    while life.tick < target {
        life.step();
    }
}

fn write_tick(tr: &mut Writer, life: &Life, out: &StepOut, state_every: u64) -> io::Result<()> {
    let t = out.obs.tick;
    if let Some((a, done)) = out.action {
        tr.write(&Rec::Action { tick: t, action: a, done_at: done })?;
    }
    if !out.obs.events.is_empty() {
        tr.write(&Rec::Obs { tick: t, events: out.obs.events.iter().map(|e| (e.block, e.value)).collect() })?;
    }
    for e in &out.report.evaluations {
        tr.write(&Rec::Eval { tick: t, block: e.block, value: e.value, made_at: e.made_at, last_obs: e.last_obs_tick, surprise: e.surprise_bits, p_of_v: e.p_of_value, entropy: e.entropy_bits })?;
    }
    if let Some(l) = &out.report.learn {
        tr.write(&Rec::Learn { tick: t, dw_in2: l.dw_in2, dw_rec2: l.dw_rec2, dw_out2: l.dw_out2, clips_hidden: l.clips_hidden, clips_out: l.clips_out, l_norm2: l.l_norm2 })?;
    }
    if let Some(h) = &out.report.homeo {
        tr.write(&Rec::Homeo { tick: t, rms: h.rms.clone(), scale: h.scale.clone(), dw2: h.dw2, saturated: h.saturated, clips: h.clips })?;
    }
    if t % state_every == 0 {
        tr.write(&Rec::State { tick: t, x: life.agent.state().to_vec(), x_max: out.report.x_max_abs, r_rms: out.report.r_rms })?;
    }
    if t % 20 == 0 {
        tr.write(&Rec::Body { tick: t, resource: life.world.resource() })?;
    }
    if t % 1000 == 0 {
        tr.write(&Rec::Hash { tick: t, state: life.agent.state_hash(), weights: life.agent.weights_hash(), input: life.input_hash, actions: life.action_hash })?;
    }
    if out.report.numerical_failure {
        tr.write(&Rec::Numeric { tick: t, kind: "non-finite state".into() })?;
    }
    Ok(())
}

fn variant_name(v: Variant) -> &'static str {
    match v {
        Variant::Structured => "structured",
        Variant::Shuffled => "shuffled",
        Variant::Iid => "iid",
    }
}

fn count_truth(truth: &[TruthEvent]) -> (u64, u64, u64) {
    let (mut f, mut r, mut c) = (0, 0, 0);
    for e in truth {
        match e {
            TruthEvent::MaintenanceFailure { .. } => f += 1,
            TruthEvent::ExternalRescue { .. } => r += 1,
            TruthEvent::StructuralChange { .. } => c += 1,
            _ => {}
        }
    }
    (f, r, c)
}

/// Run one life from birth. Writes `trace.bin`, `truth.bin`, `ckpt_*.bin`, `summary.json`.
pub fn run_life(cfg: &RunConfig) -> io::Result<RunSummary> {
    fs::create_dir_all(&cfg.out_dir)?;
    fs::write(cfg.out_dir.join("config.json"), serde_json::to_string_pretty(cfg).unwrap())?;
    let agent_cfg = arm_agent_config(&cfg.arm, &cfg.agent);
    let world = World::new(cfg.world.clone());
    let agent = Agent::new(agent_cfg.clone());
    let mut life = Life::new(world, agent, cfg.seed_action);
    let mut truth: Vec<TruthEvent> = Vec::new();
    let mut tr = if cfg.trace { Some(Writer::create(&cfg.out_dir.join("trace.bin"))?) } else { None };
    if let Some(tr) = tr.as_mut() {
        tr.write(&Rec::Meta { arm: cfg.arm.clone(), world_seed: cfg.world.seed_world, agent_seed: agent_cfg.seed, action_seed: cfg.seed_action, variant: variant_name(cfg.world.variant).into(), note: format!("label_perm={:?}", cfg.world.label_perm_seed) })?;
    }
    let p0 = save_checkpoint(&cfg.out_dir, &life, 0)?;
    if let Some(tr) = tr.as_mut() {
        tr.write(&Rec::Checkpoint { tick: 0, path: p0.display().to_string() })?;
    }
    let start = Instant::now();
    let mut status = "ok".to_string();
    let mut numerical_failure_at = None;
    while life.tick < cfg.ticks {
        let out = life.step();
        if let Some(tr) = tr.as_mut() {
            write_tick(tr, &life, &out, cfg.state_every)?;
        }
        if out.report.numerical_failure {
            status = "numerically_invalid".into();
            numerical_failure_at = Some(out.obs.tick);
            truth.extend(life.world.take_truth());
            break;
        }
        if life.tick % 10_000 == 0 {
            truth.extend(life.world.take_truth());
        }
        if cfg.checkpoint_every > 0 && life.tick % cfg.checkpoint_every == 0 {
            let p = save_checkpoint(&cfg.out_dir, &life, truth.len())?;
            if let Some(tr) = tr.as_mut() {
                tr.write(&Rec::Checkpoint { tick: life.tick, path: p.display().to_string() })?;
            }
        }
    }
    truth.extend(life.world.take_truth());
    let elapsed = start.elapsed().as_secs_f64();
    let records = match tr {
        Some(w) => w.finish()?,
        None => 0,
    };
    life_trace::write_vec(&cfg.out_dir.join("truth.bin"), &truth)?;
    let bytes = fs::metadata(cfg.out_dir.join("trace.bin")).map(|m| m.len()).unwrap_or(0);
    let (mf, rs, sc) = count_truth(&truth);
    let summary = RunSummary {
        arm: cfg.arm.clone(),
        ticks: life.tick,
        status,
        input_hash: life.input_hash,
        action_hash: life.action_hash,
        weights_hash: life.agent.weights_hash(),
        state_hash: life.agent.state_hash(),
        elapsed_s: elapsed,
        ticks_per_s: life.tick as f64 / elapsed.max(1e-9),
        trace_records: records,
        trace_bytes: bytes,
        evaluations: life.evaluations,
        maintenance_failures: mf,
        rescues: rs,
        structural_changes: sc,
        numerical_failure_at,
    };
    fs::write(cfg.out_dir.join("summary.json"), serde_json::to_string_pretty(&summary).unwrap())?;
    Ok(summary)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchMode {
    /// a new random agent (same architecture) born at the branch tick
    Fresh,
    /// the main agent's weights at the branch tick, all weight change off
    Frozen,
}

/// Load the latest checkpoint at or before `t_branch` in `run_dir`.
pub fn load_checkpoint_before(run_dir: &Path, t_branch: Tick) -> io::Result<(Checkpoint, Tick)> {
    let mut best: Option<Tick> = None;
    for entry in fs::read_dir(run_dir)? {
        let name = entry?.file_name().to_string_lossy().to_string();
        if let Some(num) = name.strip_prefix("ckpt_").and_then(|s| s.strip_suffix(".bin")) {
            if let Ok(t) = num.parse::<Tick>() {
                if t <= t_branch && best.map(|b| t > b).unwrap_or(true) {
                    best = Some(t);
                }
            }
        }
    }
    let t = best.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no checkpoint"))?;
    Ok((load_checkpoint(&checkpoint_path(run_dir, t))?, t))
}

/// Branch a life at `t_branch`: replay to that tick, modify the agent per `mode`, then run
/// `ticks` more ticks with tracing into `out_dir`. The main life is untouched.
pub fn branch(run_dir: &Path, t_branch: Tick, mode: BranchMode, fresh_seed: u64, ticks: u64, out_dir: &Path) -> io::Result<RunSummary> {
    let (ck, _) = load_checkpoint_before(run_dir, t_branch)?;
    let mut life = ck.life;
    replay_to(&mut life, t_branch);
    life.world.take_truth();
    match mode {
        BranchMode::Fresh => {
            let mut c = life.agent.config().clone();
            c.seed = fresh_seed;
            c.freeze_at = None;
            life.agent = Agent::new(c);
        }
        BranchMode::Frozen => life.agent.set_learning(false, false, false),
    }
    fs::create_dir_all(out_dir)?;
    let mut tr = Writer::create(&out_dir.join("trace.bin"))?;
    tr.write(&Rec::Meta { arm: format!("branch_{mode:?}"), world_seed: life.world.config().seed_world, agent_seed: life.agent.config().seed, action_seed: 0, variant: variant_name(life.world.config().variant).into(), note: format!("branch at {t_branch}") })?;
    let start = Instant::now();
    let end = t_branch + ticks;
    let mut status = "ok".to_string();
    let mut nf = None;
    let mut truth = Vec::new();
    while life.tick < end {
        let out = life.step();
        write_tick(&mut tr, &life, &out, 10)?;
        if out.report.numerical_failure {
            status = "numerically_invalid".into();
            nf = Some(out.obs.tick);
            break;
        }
    }
    truth.extend(life.world.take_truth());
    life_trace::write_vec(&out_dir.join("truth.bin"), &truth)?;
    let elapsed = start.elapsed().as_secs_f64();
    let records = tr.finish()?;
    let (mf, rs, sc) = count_truth(&truth);
    let summary = RunSummary { arm: format!("branch_{mode:?}"), ticks: life.tick, status, input_hash: life.input_hash, action_hash: life.action_hash, weights_hash: life.agent.weights_hash(), state_hash: life.agent.state_hash(), elapsed_s: elapsed, ticks_per_s: ticks as f64 / elapsed.max(1e-9), trace_records: records, trace_bytes: 0, evaluations: life.evaluations, maintenance_failures: mf, rescues: rs, structural_changes: sc, numerical_failure_at: nf };
    fs::write(out_dir.join("summary.json"), serde_json::to_string_pretty(&summary).unwrap())?;
    Ok(summary)
}

/// A probe stream: a plausible continuation of `world` with its own randomness and its own
/// action sampler. Agent-independent (observations do not depend on any agent), so every copy
/// evaluated on it sees the identical stream. Never fed to a main life. `emissions` and
/// `positions` are analyzer-side truth (site, true symbol, mode; position per tick).
pub struct ProbeStream {
    pub obs: Vec<Observation>,
    /// (tick, position) with the position in effect from that tick on; first entry = start
    pub positions: Vec<(Tick, u8)>,
    /// (tick, site, true symbol, mode, forced) for every emission during the stream
    pub emissions: Vec<(Tick, u8, u8, u8, bool)>,
    pub mode_at_start: u8,
}

pub fn make_probe_stream(world: &World, seed: u64, ticks: u64) -> ProbeStream {
    let mut w = world.clone();
    w.fork_rngs(seed);
    let mode_at_start = w.mode();
    let mut sampler = Rng::new(seed ^ 0xA11CE);
    let mut obs = Vec::with_capacity(ticks as usize);
    let mut positions = vec![(w.tick(), w.pos() as u8)];
    let end = w.tick() + ticks;
    while w.tick() < end {
        let t = w.tick() + 1;
        if !w.busy() && !w.incapacitated() {
            let a = Action::ALL[sampler.below(5) as usize];
            let _ = w.request_action(a, t);
        }
        let o = w.step();
        for ev in &o.events {
            if ev.block == 3 {
                positions.push((o.tick, w.unpermute(3, ev.value)));
            }
        }
        obs.push(o);
    }
    let mut emissions = Vec::new();
    for e in w.take_truth() {
        if let TruthEvent::Emission { tick, site, symbol, mode, forced } = e {
            emissions.push((tick, site, symbol, mode, forced));
        }
    }
    ProbeStream { obs, positions, emissions, mode_at_start }
}
