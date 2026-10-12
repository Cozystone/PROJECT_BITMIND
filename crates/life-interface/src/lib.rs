//! life-interface: the only types shared between the world and the agent.
//!
//! Observations and actions only. No field of any type here carries world truth (hidden
//! generators, change times, noise flags). The agent crate depends on this crate and nothing else.

use serde::{Deserialize, Serialize};

pub type Tick = u64;

/// Observation blocks: each is a categorical channel delivered as events.
pub const NUM_BLOCKS: usize = 6;
pub const BLOCK_SIZES: [usize; NUM_BLOCKS] = [6, 6, 6, 8, 16, 5];
/// Held blocks keep their last value in the input vector until the next event; pulse blocks are
/// present for one tick only.
pub const BLOCK_HELD: [bool; NUM_BLOCKS] = [false, false, false, true, true, false];
pub const BLOCK_NAMES: [&str; NUM_BLOCKS] = ["token_here", "token_left", "token_right", "position", "interoception", "action_echo"];
pub const OBS_DIM: usize = 6 + 6 + 6 + 8 + 16 + 5;

pub fn block_offset(k: usize) -> usize {
    BLOCK_SIZES[..k].iter().sum()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Look,
    MoveLeft,
    MoveRight,
    Touch,
    Rest,
}

impl Action {
    pub const ALL: [Action; 5] = [Action::Look, Action::MoveLeft, Action::MoveRight, Action::Touch, Action::Rest];
    pub fn index(self) -> u8 {
        match self {
            Action::Look => 0,
            Action::MoveLeft => 1,
            Action::MoveRight => 2,
            Action::Touch => 3,
            Action::Rest => 4,
        }
    }
    pub fn from_index(i: u8) -> Action {
        Action::ALL[i as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObsEvent {
    pub block: u8,
    pub value: u8,
}

/// Everything the agent receives at one tick. Empty `events` is a valid observation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Observation {
    pub tick: Tick,
    pub events: Vec<ObsEvent>,
}

/// SplitMix64. Every stream (world timing, world symbols, sensor noise, action sampler, network
/// initialisation, probes) owns one instance seeded independently.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rng {
    s: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng { s: seed ^ 0x9E37_79B9_7F4A_7C15 }
    }
    pub fn next_u64(&mut self) -> u64 {
        self.s = self.s.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in [0, 1) with 24 bits.
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Standard normal (Box-Muller, one draw used).
    pub fn normal(&mut self) -> f32 {
        let u1 = self.f32().max(1e-7);
        let u2 = self.f32();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos()
    }
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.below(i as u64 + 1) as usize;
            v.swap(i, j);
        }
    }
    /// A new independent stream derived from this one.
    pub fn fork(&mut self, tag: u64) -> Rng {
        Rng::new(self.next_u64() ^ tag.wrapping_mul(0xD6E8_FEB8_6659_FD93))
    }
}

/// Running 64-bit hash (FNV-1a over 64-bit words) used to verify identical input and action
/// streams across arms.
pub const HASH_SEED: u64 = 0xcbf2_9ce4_8422_2325;
pub fn hash_mix(h: u64, v: u64) -> u64 {
    let mut h = h;
    for i in 0..8 {
        h ^= (v >> (8 * i)) & 0xff;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}
pub fn hash_f32s(mut h: u64, v: &[f32]) -> u64 {
    for x in v {
        h = hash_mix(h, x.to_bits() as u64);
    }
    h
}
