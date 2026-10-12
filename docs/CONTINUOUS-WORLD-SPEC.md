# Continuous worlds (revised 2026-10-12)

The earlier 16 × 16 "Patchland" with bushes, fruit and brambles is **demoted**: it is one
possible later environment, not the agent's purpose and not the only form a world may take.
It carried task-like meanings (food, hazard, shelter). The first experiment uses the smaller
world below, which has temporal structure and an action interface but no built-in meanings.

## 1. Requirements for any world in this project

* One clock that never resets; observations arrive as events; actions take time and consume
  the body's resource.
* Temporal regularities exist in the sensory stream (so there is something to learn).
* A small action interface can influence future events (so action matters).
* **No meanings are given to the agent:** nothing in the agent says "this token is food",
  "this action restores", "this place is safe". If such regularities exist in the world, the
  agent can only find them through its own predictions.
* World truth (hidden generators, rule-change times) is visible to the analyzer only.

## 2. Ring world (first experiment)

**Space.** A ring of K = 8 sites. The agent occupies one site.

**Emitters.** Each site has an emitter that produces tokens from an alphabet of 6 symbols.
Tokens are events, not a per-tick vector: each emitter has its own inter-event interval
(drawn from a site-specific distribution, mean 8-40 ticks, drawn per seed). The next token
depends on the previous two tokens of that site (a second-order Markov grammar, per site) and,
for 2 of the 8 sites, on the current token of a neighbour (a cross-site relation).

**Hidden slow variable.** A two-state hidden "mode" (mean dwell 3,000 ticks) that changes the
grammar of 3 sites. It is not observed; only its effects on tokens are.

**Rare unannounced structural change.** At pre-drawn times (one per 150,000-250,000 ticks),
the grammar of 2 sites is re-drawn and one cross-site relation is moved.

**Perturbation by action.** `touch` at the current site resets that emitter's phase and,
for 100 ticks, forces its grammar to a fixed short cycle (so acting creates a predictable
consequence to learn).

## 3. Observations (what the agent receives)

Events `{tick, channel, value}`:

| channel | value | imperfection |
|---|---|---|
| token here | the symbol emitted at the current site | 4% read as a random other symbol |
| token nearby | symbols emitted at the two neighbouring sites, 1 tick late | 8% misread; 10% dropped |
| position | current site index (nominal) | exact |
| interoception | body resource in 16 buckets, every 20 ticks and after each action | exact bucket |
| action echo | which action just completed | exact |

No channel carries the hidden mode, the grammar, or any "good/bad" label.

## 4. The body: a maintenance condition, disclosed as designed by us

| component | what it is | who designed it |
|---|---|---|
| resource R ∈ [0, 1000] | −1 per tick; `move` −6, `touch` −10, `look` −1, `rest` +8 per tick of rest (cap 1000) | the world's physics (us) |
| interoceptive signal | R in 16 buckets, read by the agent as an observation like any other | the sensor (us) |
| innate regulation | the network's activity homeostasis (`ARCHITECTURE-OPTIONS.md` section 3.4): keeps unit activity near a target rate so learning neither runs away nor dies out. It is **not** about R | the agent's design (us) |
| human-added preference or cost | **none in the first experiment.** No term rewards high R or penalises low R. R is only predicted, like every other channel | — |

**Incapacity and rescue.** When R reaches 0 the body cannot act for 2,000 ticks; it then
restarts with R = 500. This restart is an **external rescue** by the world, not self-recovery.
The analyzer records three separate events: `maintenance_failure` (R = 0), `external_rescue`
(the reset), `resumption` (first action after it). None of them is counted as recovery by the
agent. There is no "survival instinct" in the agent and none is claimed.

## 5. Actions

| action | duration (ticks) | resource | effect on the world |
|---|---|---|---|
| `look` | 2 | −1 | an extra independent read of the neighbour channels |
| `move_left`, `move_right` | 6 | −6 | changes the site |
| `touch` | 10 | −10 | resets the local emitter's phase; forces a short cycle for 100 ticks |
| `rest` | 20 | +8 per tick | nothing |

In the first experiment the actions are chosen by a **fixed random sampler** from a seeded
stream (`FIRST-EXPERIMENT.md`): the question of this stage is whether experience leaves
traces, not how actions are chosen. Action selection from the agent's own model is a later
stage.

## 6. Control worlds (same seeds, same action streams)

| world | difference | purpose |
|---|---|---|
| **shuffled order** | each site's token sequence is a random permutation of the original one (same symbol frequencies, no temporal grammar) | the memory tests should fail here |
| **iid** | tokens drawn independently from each site's marginal distribution | same |
| **label permutation** | every symbol relabelled through a fixed bijection | a hard-coding check: results must be unchanged |

## 7. Patchland (deferred, optional)

The 16 × 16 object world of the earlier draft is kept in the repository history as an optional
later environment. If used, its meanings (fruit, damage, shelter) stay in the world only; the
agent still receives no labels.
