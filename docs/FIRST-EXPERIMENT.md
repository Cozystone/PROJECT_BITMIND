# First experiment: the core hypothesis, the smallest implementation, the budget

2026-10-12. No code exists yet. Implementation starts only after approval.

## 1. The one hypothesis tested in this round

> In a continuous world with temporal structure, a 64-unit continuous-time recurrent network whose only plasticity is a local, forward-in-time three-factor rule driven by its own next-observation prediction error (candidate C) accumulates changes in its connections that (i) persist beyond its state dynamics, (ii) depend on the history of events, (iii) improve its later predictions, (iv) distinguish event order and (v) are revised when the world's relations change — beyond a plasticity-off clone and beyond worlds without temporal structure.

Nothing about action selection, viability, recovery, curiosity or agency is tested in this
round.

## 2. The smallest implementation (what gets built first, in order)

1. `life-interface`: `Observation`, `Action`, event types. No truth fields.
2. `life-world`: the ring world of `CONTINUOUS-WORLD-SPEC.md` section 2, its three control
   variants, the body resource and rescue, the truth log, deterministic seeding.
3. `life-agent`: candidate C exactly as in `ARCHITECTURE-OPTIONS.md` section 3: encoding,
   semi-implicit state step, softmax readouts, errors, eligibility traces, random feedback,
   three-factor update, weight bounds, homeostatic scaling, LTC flag. f32, deterministic
   operation order, no SIMD-dependent reductions (so hashes are reproducible). No dependency
   on `life-world` or `life-lab` (CI check).
4. `life-trace`: append-only binary records with the layers of `OBSERVABILITY.md`, chunked
   and compressed; separate agent-side and truth-side files; the neural-dynamics records of
   `OBSERVABILITY.md` section 7.
5. `life-run`: one life = (world seed, agent seed, action-stream seed, arm config); the fixed
   random action sampler; checkpoints every 50,000 ticks (analysis only).
6. `life-lab`: the analyzer: measures 1-6, chain and blocking tests, observer-invariance and
   determinism tests, profiler, aggregate tables (CSV/JSON).

Not built in this round: any action-selection rule, candidate B, Patchland, the state
narrator, a dashboard.

## 3. Budget (proposed; the run is not open-ended)

| item | budget |
|---|---|
| implementation and unit tests (steps 1-6) | 4 working sessions |
| profiling and the observer-invariance / determinism checks | 1 session |
| first comparison experiment | 10 lives × 11 arms × 10⁶ ticks |
| compute per life | N = 64, m = 47: about 25k multiply-adds per tick plus traces ≈ 10⁷ ops per 10³ ticks; one life in 2-6 minutes in Rust on this workstation; the whole battery under 4 hours, one process at a time |
| memory | agent < 5 MB; trace write buffer ≤ 8 MB; world < 1 MB |
| disk | ≈ 50-100 MB per life compressed (state subsampled 1/10; full detail ± 2,000 ticks around flagged events); battery ≈ 5-10 GB |
| analysis | 1-2 sessions |

If a life takes more than 30 minutes or a process exceeds 2 GB, that is reported as a
resource problem and fixed as a bug, not by shrinking the world.

## 4. Stop criteria (fixed now)

* The gate of `MEASUREMENT-PLAN.md` section 4 is evaluated once, after the full battery.
* **Supported:** the next round designs action selection from the agent's own predictions,
  pre-registered separately.
* **Partial or not supported:** candidate C is recorded as such. The world is not enlarged,
  the network is not enlarged, no rule is added and no threshold is moved to rescue it. A new
  hypothesis is written (candidate B, or a different learning signal), with its own
  pre-registration.
* During the formal battery, code, worlds and measure definitions are frozen (tag + guard). A
  bug found during the battery stops the battery; the affected runs are kept as a record and
  a new battery starts from a new tag.

## 5. What is disclosed as ours (innate structure and maintenance conditions)

The encoding; N; the τ range; A, B, W_g; η, η_out, η_h, r*, w_max, T_h; the body's resource
physics; the rescue rule; the action sampler. All seeded and logged. None of these is a
preference, a reward or a label. The first thing to look for when something "works" is which of
these made it work.

## 6. Distinguishing code improvement from experience

Two kinds of change are never mixed in a report:
* a change in results because the code, world or measures were changed (a new experiment);
* a change in the agent's behaviour within a frozen life because of its own experience (the
  object of study).

Every result line states the tag it was produced under.
