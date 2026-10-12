# BITMIND-LIFE

BITMIND-LIFE is a persistent-agent research prototype. It evaluates change over a continuous lifetime, not only correctness on resettable tasks.

**Status:** first experiment implemented (approved scope: `docs/APPENDIX-FIRST-EXPERIMENT.md`);
development phase complete; formal seeds run only from the frozen tag (`experiments/FREEZE.md`).
Random actions are the control condition of this round: nothing about action selection,
self-maintenance or instinct is claimed.

## The one question of the first stage (revised 2026-10-12)

> Does experience leave lasting traces in the system, and do those traces change what it predicts and does later?

The system has no external task, answer or reward. It is not a foraging or survival agent.
Nothing more is claimed than what the measurements show. Not consciousness, general
intelligence, autonomy, or real-world readiness.

## Documents

| document | content |
|---|---|
| [docs/RESEARCH-QUESTION.md](docs/RESEARCH-QUESTION.md) | the question without an external task; hypothesis H1; what refutes it |
| [docs/THEORY-AUDIT.md](docs/THEORY-AUDIT.md) | Friston 2010, Neftci et al. 2019, Hasani et al. 2020: claims, actual methods, what is taken, what is not, our untested hypotheses, what was not read |
| [docs/ARCHITECTURE-OPTIONS.md](docs/ARCHITECTURE-OPTIONS.md) | candidates A (probability tables, demoted to baseline), B (spiking, deferred), C (continuous-time rate network with local plasticity, chosen); exact state and learning equations |
| [docs/CONTINUOUS-WORLD-SPEC.md](docs/CONTINUOUS-WORLD-SPEC.md) | the ring world: temporally structured events, a small action interface, no meanings; the body as a disclosed maintenance condition; control worlds |
| [docs/MEASUREMENT-PLAN.md](docs/MEASUREMENT-PLAN.md) | the six "experience leaves traces" measures, arms and controls, chain and blocking tests, the gate |
| [docs/FIRST-EXPERIMENT.md](docs/FIRST-EXPERIMENT.md) | the one hypothesis of this round, the smallest implementation, budget, stop criteria |
| [docs/OBSERVABILITY.md](docs/OBSERVABILITY.md) | LifeTrace, the learner/analyzer barrier, replay, provenance, diff, profiling, neural-dynamics records, narrator rules |
| [docs/SAFETY-AND-SCOPE.md](docs/SAFETY-AND-SCOPE.md) | scope limits, forbidden capabilities and claims, maintenance conditions vs instincts, reporting discipline |
| [docs/INHERITED-ASSETS.md](docs/INHERITED-ASSETS.md) | what may be considered from the archived BITMIND evidence research, and what is not carried over |
| [docs/APPENDIX-FIRST-EXPERIMENT.md](docs/APPENDIX-FIRST-EXPERIMENT.md) | approved amendments A1-A9 (time order, traces as approximation, arms, homeostasis, probes, controls, the concrete gate) and the development phase A10 (findings, amendments, frozen configuration) |

## Code

| crate | role |
|---|---|
| `crates/life-interface` | observation and action types shared by world and agent; no truth fields; SplitMix64 streams; stream hashes |
| `crates/life-world` | the ring world and its control variants (shuffled, iid, label permutation); body and rescue; analyzer-only truth log |
| `crates/life-agent` | candidate C: leaky-integrator recurrent network, local three-factor plasticity on next-observation error, bounded homeostasis; depends on `life-interface` only (`scripts/check_barrier.ps1`) |
| `crates/life-trace` | append-only binary records (observations, evaluations, state, plasticity, homeostasis, hashes) |
| `crates/life-run` | one life: world + agent + external random action sampler; checkpoints; branches; probe streams |
| `crates/life-lab` | analysis: audit, summary, compare, gradcheck, probes (history, usefulness, order, re-adaptation), per-seed evaluate, gate |

```
cargo build --release && cargo test --release
./target/release/life-run run --out results/x --arm full --world-seed 1 --ticks 200000
./target/release/life-lab summary --run results/x
bash scripts/run_seed.sh 1 results/dev experiments/formal-config.json     # one seed, all arms
bash scripts/run_formal.sh results/formal 2                                # seeds 101-110 (freeze guard)
```

## Origin

This repository is independent. It shares no commits, tests or evaluation criteria with the
archived BITMIND evidence-settlement research (repository `Cozystone/THE_ULTIMATE`, tag
`bitmind-evidence-research-archive`). No code is copied from it. Candidate ideas are listed in
`docs/INHERITED-ASSETS.md`, and each must be re-justified and re-tested before use.
