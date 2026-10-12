# BITMIND-LIFE

BITMIND-LIFE is a persistent-agent research prototype. It evaluates change over a continuous lifetime, not only correctness on resettable tasks.

**Status:** design only. This repository contains no code yet; implementation starts only after
the design documents are reviewed and approved.

## The one question of the first stage

> Does this system, living one continuous life, recover after failures and adapt better as experience accumulates?

Nothing more is claimed. Not consciousness, general intelligence, autonomy, or real-world
readiness.

## Documents

| document | content |
|---|---|
| [docs/RESEARCH-QUESTION.md](docs/RESEARCH-QUESTION.md) | hypothesis, what would support or refute it, first research gate |
| [docs/CONTINUOUS-WORLD-SPEC.md](docs/CONTINUOUS-WORLD-SPEC.md) | the minimal continuous world: external state, body, observations, actions, change over time |
| [docs/ARCHITECTURE-OPTIONS.md](docs/ARCHITECTURE-OPTIONS.md) | three architectures compared; the chosen one; how the free-energy principle becomes computation |
| [docs/MEASUREMENT-PLAN.md](docs/MEASUREMENT-PLAN.md) | lifetime metrics, graphs, baselines, gate decision rules, falsification plan |
| [docs/OBSERVABILITY.md](docs/OBSERVABILITY.md) | LifeTrace, the learner/analyzer information barrier, replay, provenance, diff, profiling, state narrator rules |
| [docs/SAFETY-AND-SCOPE.md](docs/SAFETY-AND-SCOPE.md) | scope limits, forbidden capabilities and claims, reporting discipline |
| [docs/INHERITED-ASSETS.md](docs/INHERITED-ASSETS.md) | what may be considered from the archived BITMIND evidence research, and what is not carried over |

## Origin

This repository is independent. It shares no commits, tests or evaluation criteria with the
archived BITMIND evidence-settlement research (repository `Cozystone/THE_ULTIMATE`, tag
`bitmind-evidence-research-archive`). No code is copied from it. Candidate ideas are listed in
`docs/INHERITED-ASSETS.md`, and each must be re-justified and re-tested before use.
