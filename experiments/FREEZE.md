# Freeze: first experiment (bitmind-life-first-v0.1)

Frozen on 2026-10-12, before any formal seed was run.

- tag: `bitmind-life-first-v0.1` (the commit that adds this file; the implementation commit is ee2cfc93dae22cecce6911ce45223ee48d3e7bc5)
- formal-config.json sha256: e10b7004384e8e0cd4a6fe04c7245ace5bdfdc3680186e3470e6fad3a2df1467
- guard: `scripts/check_freeze_life.ps1` (tag at HEAD, clean tree, config hash, no recompilation, barrier)
- formal seeds: 101-110 (`scripts/run_formal.sh`), development seeds 1-3 excluded from evidence
- what is frozen: code of all six crates, `experiments/formal-config.json`, the arms and their
  schedule in `scripts/run_seed.sh`, the probe definitions and gate constants in
  `crates/life-lab/src/{probes,gate}.rs`, and the documents `docs/APPENDIX-FIRST-EXPERIMENT.md`
  (A1-A10) and `docs/MEASUREMENT-PLAN.md`
- recorded expectation (A10, Finding 5 and the development picture): P1-P3 against the birth
  network are expected to pass; the hidden-plasticity verdict is expected to be "not shown"; the
  control worlds are expected to show a surprise reduction of the same size as the structured
  world, so the structure-specific gain is the quantity to read
- after the formal run, any code change that improves results is a new experiment under a new tag
