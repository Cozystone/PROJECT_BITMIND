//! life-trace: append-only record sink (docs/OBSERVABILITY.md). Agent-side records only; the
//! world's truth log is written as a separate file by the runner through `write_vec`.
//! The writer is one-way: no read API is exposed to the agent, and writing consumes no randomness.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Rec {
    Meta { arm: String, world_seed: u64, agent_seed: u64, action_seed: u64, variant: String, note: String },
    Obs { tick: u64, events: Vec<(u8, u8)> },
    /// One evaluated prediction: `made_at` < `tick` always.
    Eval { tick: u64, block: u8, value: u8, made_at: u64, last_obs: u64, surprise: f32, p_of_v: f32, entropy: f32 },
    State { tick: u64, x: Vec<f32>, x_max: f32, r_rms: f32 },
    Learn { tick: u64, dw_in2: f32, dw_rec2: f32, dw_out2: f32, clips_hidden: u32, clips_out: u32, l_norm2: f32 },
    Homeo { tick: u64, rms: Vec<f32>, scale: Vec<f32>, dw2: f32, saturated: u32, clips: u32 },
    Action { tick: u64, action: u8, done_at: u64 },
    Body { tick: u64, resource: i32 },
    Hash { tick: u64, state: u64, weights: u64, input: u64, actions: u64 },
    Numeric { tick: u64, kind: String },
    Checkpoint { tick: u64, path: String },
}

pub struct Writer {
    w: BufWriter<File>,
    pub records: u64,
}

impl Writer {
    pub fn create(path: &Path) -> io::Result<Writer> {
        let f = File::create(path)?;
        Ok(Writer { w: BufWriter::with_capacity(8 << 20, f), records: 0 })
    }
    pub fn write(&mut self, rec: &Rec) -> io::Result<()> {
        bincode::serialize_into(&mut self.w, rec).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        self.records += 1;
        Ok(())
    }
    pub fn finish(mut self) -> io::Result<u64> {
        self.w.flush()?;
        Ok(self.records)
    }
}

/// Stream every record of a trace file to `f`.
pub fn read_each(path: &Path, mut f: impl FnMut(Rec)) -> io::Result<u64> {
    let file = File::open(path)?;
    let mut r = BufReader::with_capacity(8 << 20, file);
    let mut n = 0u64;
    loop {
        match bincode::deserialize_from::<_, Rec>(&mut r) {
            Ok(rec) => {
                f(rec);
                n += 1;
            }
            Err(e) => {
                if let bincode::ErrorKind::Io(ioe) = &*e {
                    if ioe.kind() == io::ErrorKind::UnexpectedEof {
                        break;
                    }
                }
                return Err(io::Error::new(io::ErrorKind::Other, e));
            }
        }
    }
    Ok(n)
}

pub fn write_vec<T: Serialize>(path: &Path, items: &[T]) -> io::Result<()> {
    let f = File::create(path)?;
    let mut w = BufWriter::new(f);
    bincode::serialize_into(&mut w, items).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    w.flush()
}

pub fn read_vec<T: DeserializeOwned>(path: &Path) -> io::Result<Vec<T>> {
    let f = File::open(path)?;
    let r = BufReader::new(f);
    bincode::deserialize_from(r).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}
