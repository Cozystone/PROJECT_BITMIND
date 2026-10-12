# Inherited assets: candidates only

Source: the archived BITMIND evidence-settlement research (`Cozystone/THE_ULTIMATE`, tag
`bitmind-evidence-research-archive`, `docs/ARCHIVE-DECISION.md`).

**Rule:** no code is copied. An idea below may be used only when the new agent needs it. It
must then be re-implemented or re-ported as an isolated component, with its own tests, and
with a stated reason tied to a measured need.

## Candidate ideas

| asset | what it is | when it might be considered | what must be re-verified |
|---|---|---|---|
| observation ≠ fact | readings carry source and standing; inferred values are marked | already reflected in B's sensor model (readings are evidence about latent truth) | sensor-model calibration on the new world |
| evidence keeps its subject | an update says which belief (table entry) it changes and why; a failure of one model part is not counted against another | already reflected: per-table counts and per-table change detection | that no update aggregates across unrelated tables |
| repeats are not independent | one source read repeatedly is not independent evidence | possibly for `look` (repeated readings of one cell) | that the sensor model does not double-count correlated errors |
| HDC binding and item memory | compact similarity-based encoding | only if option C is triggered by a measured generalisation gap | memory, determinism, and benefit over plain context tables in an ablation |
| read-only provenance tracing | traces that reconstruct why an answer happened | the design of LifeTrace | rebuilt for the new record schema |
| harness-only oracle bound | the analyzer measures what is reachable with truth, without feeding the learner | for "best achievable" references in analysis | strict separation from all policies |
| 16 GB watchdog, pre-registration discipline | process limits; criteria fixed before runs | from the start | n/a |

## Assumptions not carried over

* Resettable episodes, fixed action budgets, queries as the unit of evaluation.
* Licensed laws, D041/D061 settlement, refinement, latent inducers as the agent's core.
* "0 wrong answers" as a global objective.
* The I-series and N-series criteria and results as success criteria for this project.
