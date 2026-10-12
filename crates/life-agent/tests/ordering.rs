//! Invariants of the prediction/observation time order, the absence of a copy path, and basic
//! learnability (docs/APPENDIX-FIRST-EXPERIMENT.md A1).

use life_agent::*;
use life_interface::*;

fn cfg(seed: u64) -> AgentConfig {
    AgentConfig { seed, ..AgentConfig::default() }
}

fn obs(t: Tick, events: &[(u8, u8)]) -> Observation {
    Observation { tick: t, events: events.iter().map(|&(b, v)| ObsEvent { block: b, value: v }).collect() }
}

/// Every evaluated prediction was made strictly before the event it is evaluated against, and
/// after the last observation it could have used.
#[test]
fn every_evaluation_uses_a_prediction_made_before_the_event() {
    let mut a = Agent::new(cfg(1));
    let mut rng = Rng::new(5);
    let mut n_eval = 0;
    for t in 1..=5000u64 {
        let mut ev = Vec::new();
        if rng.below(4) == 0 {
            ev.push((0u8, rng.below(6) as u8));
        }
        if t % 20 == 0 {
            ev.push((4u8, rng.below(16) as u8));
        }
        let rep = a.tick(&obs(t, &ev));
        for e in &rep.evaluations {
            assert!(e.made_at < t, "made_at {} not before tick {}", e.made_at, t);
            assert_eq!(e.made_at, t - 1, "a prediction is re-issued every tick");
            assert!(e.last_obs_tick < t);
            assert!(e.last_obs_tick <= e.made_at);
            n_eval += 1;
        }
    }
    assert!(n_eval > 1000);
}

/// No copy path: with iid uniform tokens, surprise stays at chance (log2 6 = 2.585 bits). A path
/// from the current observation to the prediction of the same observation would drive it down.
#[test]
fn iid_tokens_stay_at_chance_for_every_arm() {
    for (hidden, readout) in [(true, true), (false, true), (false, false)] {
        let mut c = cfg(2);
        c.learn_hidden = hidden;
        c.learn_readout = readout;
        let mut a = Agent::new(c);
        let mut rng = Rng::new(9);
        let (mut sum, mut cnt) = (0.0f64, 0u64);
        for t in 1..=30_000u64 {
            let ev = if t % 3 == 0 { vec![(0u8, rng.below(6) as u8)] } else { vec![] };
            let rep = a.tick(&obs(t, &ev));
            if t > 20_000 {
                for e in &rep.evaluations {
                    sum += e.surprise_bits as f64;
                    cnt += 1;
                }
            }
        }
        let mean = sum / cnt as f64;
        assert!(mean > 2.585 - 0.15, "arm hidden={hidden} readout={readout}: mean surprise {mean} below chance");
        assert!(mean < 2.585 + 0.6, "arm hidden={hidden} readout={readout}: mean surprise {mean} far above chance");
    }
}

/// Learnability: a deterministic 4-cycle of tokens becomes predictable for the full and the
/// readout-only arms; the frozen arm does not learn it.
#[test]
fn a_deterministic_cycle_is_learned_unless_frozen() {
    let mut results = Vec::new();
    for (hidden, readout) in [(true, true), (false, true), (false, false)] {
        let mut c = cfg(3);
        c.learn_hidden = hidden;
        c.learn_readout = readout;
        let mut a = Agent::new(c);
        let cycle = [0u8, 3, 1, 4];
        let (mut sum, mut cnt) = (0.0f64, 0u64);
        let mut k = 0usize;
        for t in 1..=40_000u64 {
            let ev = if t % 5 == 0 {
                k += 1;
                vec![(0u8, cycle[k % 4])]
            } else {
                vec![]
            };
            let rep = a.tick(&obs(t, &ev));
            if t > 30_000 {
                for e in &rep.evaluations {
                    sum += e.surprise_bits as f64;
                    cnt += 1;
                }
            }
        }
        results.push((hidden, readout, sum / cnt as f64));
    }
    let full = results[0].2;
    let readout = results[1].2;
    let frozen = results[2].2;
    eprintln!("cycle test: full={full:.3} readout={readout:.3} frozen={frozen:.3} bits");
    assert!(full < 0.5, "full arm surprise {full}");
    assert!(readout < 0.5, "readout arm surprise {readout}");
    assert!(frozen > 1.5, "frozen arm surprise {frozen}");
}

/// Determinism: same seed and inputs give identical state and weight hashes.
#[test]
fn same_seed_same_inputs_same_hashes() {
    let mut a = Agent::new(cfg(4));
    let mut b = Agent::new(cfg(4));
    let mut rng = Rng::new(11);
    for t in 1..=3000u64 {
        let ev = if rng.below(3) == 0 { vec![(0u8, rng.below(6) as u8), (1u8, rng.below(6) as u8)] } else { vec![] };
        let o = obs(t, &ev);
        a.tick(&o);
        b.tick(&o);
    }
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.weights_hash(), b.weights_hash());
}

/// The state reset used by analysis probes clears what it claims to clear and nothing else.
#[test]
fn reset_state_keeps_weights() {
    let mut a = Agent::new(cfg(6));
    let mut rng = Rng::new(12);
    for t in 1..=2000u64 {
        let ev = if rng.below(3) == 0 { vec![(0u8, rng.below(6) as u8)] } else { vec![] };
        a.tick(&obs(t, &ev));
    }
    let w = a.weights_hash();
    a.reset_state();
    assert_eq!(a.weights_hash(), w);
    assert!(a.state().iter().all(|&v| v == 0.0));
    let rep = a.tick(&obs(2001, &[(0, 2)]));
    assert!(rep.evaluations.is_empty(), "no stored prediction survives a reset");
}
