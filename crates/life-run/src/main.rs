//! life-run CLI.
//!   life-run run --out DIR --arm full --world-seed 1 [--variant structured|shuffled|iid]
//!            [--perm SEED] [--ticks N] [--agent-seed S] [--action-seed S] [--noise-seed S]
//!            [--ckpt-every N] [--state-every N] [--no-trace]
//!   life-run branch --run DIR --at T --mode fresh|frozen --ticks N --out DIR [--fresh-seed S]

use life_agent::AgentConfig;
use life_run::*;
use life_world::{Variant, WorldConfig};
use std::collections::HashMap;
use std::path::PathBuf;

fn parse_args() -> (String, HashMap<String, String>) {
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

fn get<T: std::str::FromStr>(m: &HashMap<String, String>, k: &str, d: T) -> T {
    m.get(k).and_then(|v| v.parse().ok()).unwrap_or(d)
}

fn main() {
    let (cmd, m) = parse_args();
    match cmd.as_str() {
        "run" => {
            let world_seed: u64 = get(&m, "world-seed", 1);
            let variant = match m.get("variant").map(|s| s.as_str()).unwrap_or("structured") {
                "shuffled" => Variant::Shuffled,
                "iid" => Variant::Iid,
                _ => Variant::Structured,
            };
            let mut world = WorldConfig::new(world_seed, get(&m, "noise-seed", world_seed.wrapping_mul(7919) + 3), variant);
            world.label_perm_seed = m.get("perm").and_then(|s| s.parse().ok());
            // base configuration: a frozen file (--config) or the defaults; CLI values override
            let base: Option<RunConfig> = m.get("config").map(|p| serde_json::from_str(&std::fs::read_to_string(p).expect("read config")).expect("parse config"));
            let base_agent = base.as_ref().map(|b| b.agent.clone()).unwrap_or_default();
            if let Some(b) = base.as_ref() {
                let mut w = b.world.clone();
                w.seed_world = world.seed_world;
                w.seed_noise = world.seed_noise;
                w.variant = world.variant;
                w.label_perm_seed = world.label_perm_seed;
                world = w;
            }
            let agent = AgentConfig {
                seed: get(&m, "agent-seed", world_seed.wrapping_mul(104729) + 11),
                n: get(&m, "n", base_agent.n),
                g_in: get(&m, "g-in", base_agent.g_in),
                g_in_held: get(&m, "g-in-held", base_agent.g_in_held),
                g_rec: get(&m, "g-rec", base_agent.g_rec),
                eta_hidden: get(&m, "eta-hidden", base_agent.eta_hidden),
                eta_out: get(&m, "eta-out", base_agent.eta_out),
                eta_h: get(&m, "eta-h", base_agent.eta_h),
                rms_target: get(&m, "rms-target", base_agent.rms_target),
                tau_min: get(&m, "tau-min", base_agent.tau_min),
                tau_max: get(&m, "tau-max", base_agent.tau_max),
                w_max_hidden: get(&m, "w-max-hidden", base_agent.w_max_hidden),
                w_max_out: get(&m, "w-max-out", base_agent.w_max_out),
                t_h: get(&m, "t-h", base_agent.t_h),
                h_max: get(&m, "h-max", base_agent.h_max),
                ..base_agent
            };
            let cfg = RunConfig {
                out_dir: PathBuf::from(m.get("out").expect("--out")),
                arm: m.get("arm").cloned().unwrap_or_else(|| "full".into()),
                world,
                agent,
                seed_action: get(&m, "action-seed", world_seed.wrapping_mul(15485863) + 5),
                ticks: get(&m, "ticks", base.as_ref().map(|b| b.ticks).unwrap_or(1_000_000)),
                checkpoint_every: get(&m, "ckpt-every", base.as_ref().map(|b| b.checkpoint_every).unwrap_or(50_000)),
                state_every: get(&m, "state-every", base.as_ref().map(|b| b.state_every).unwrap_or(100)),
                trace: !m.contains_key("no-trace"),
            };
            let s = run_life(&cfg).expect("run");
            println!("{}", serde_json::to_string_pretty(&s).unwrap());
        }
        "branch" => {
            let run = PathBuf::from(m.get("run").expect("--run"));
            let at: u64 = get(&m, "at", 0);
            let mode = match m.get("mode").map(|s| s.as_str()).unwrap_or("frozen") {
                "fresh" => BranchMode::Fresh,
                _ => BranchMode::Frozen,
            };
            let out = PathBuf::from(m.get("out").expect("--out"));
            let s = branch(&run, at, mode, get(&m, "fresh-seed", 999), get(&m, "ticks", 50_000), &out).expect("branch");
            println!("{}", serde_json::to_string_pretty(&s).unwrap());
        }
        _ => {
            eprintln!("usage: life-run run|branch ...");
            std::process::exit(2);
        }
    }
}
