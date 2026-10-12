# The minimal continuous world ("Patchland", v0 specification)

A small, transparent world. Every rule is listed here, and every one is in the world crate.
None is in the agent.

## 1. Time

* One world clock in integer **ticks**. It never resets during a life.
* The simulation is a discrete-event scheduler; a tick is a computational unit, not a unit of
  the agent's life. Actions have durations in ticks. The agent decides again when its current
  action completes or is interrupted (by damage, or by collapse).
* Observations are delivered as events whenever something enters the agent's senses or a sensor
  is read. Interoceptive readings (energy, damage) are delivered every `T_INTERO = 50` ticks and
  at every action completion.
* There is no episode, no "end of task", and no reset. Checkpoints exist only for analysis and
  replay (`OBSERVABILITY.md`) and never feed back into the main life.

## 2. External state (world-owned; visible to the analyzer only)

| element | definition |
|---|---|
| grid | 16 x 16 cells, bounded. Terrain per cell: `ground`, `bramble`, `shelter` |
| bushes | 12 bushes at cells. Each has a hidden **kind** (`sweet`, `bitter`, `nettle`), a visible **colour** (4 values), a fruit count (0-3) |
| fruit regrowth | each bush regrows one fruit with a probability per tick that depends on season (below) |
| weather | hidden `dry`/`wet`, a two-state Markov chain (mean dwell 2,000 ticks) |
| day/night | a cycle of 4,000 ticks (2,000 day, 2,000 night) |
| season | slow sinusoid of the regrowth rate, period 40,000 ticks |
| regime | the colour → kind assignment and the bramble layout. Regime changes are **rare and unannounced** (section 6) |

**Causal rules (fixed within a regime):**
* Shaking a `sweet` bush with fruit drops a fruit (eating: +900 energy).
* Shaking a `bitter` bush with fruit drops a fruit (eating: +250 energy, +150 damage).
* Shaking a `nettle` bush: +400 damage dry, +900 damage wet, and drops fruit (+400 energy) only
  half the time.
* Entering a `bramble` cell: +200 damage dry, +500 damage wet.
* Resting in `shelter`: damage recovers 6 per tick (else 1 per tick); weather has no effect
  there.
* Within a regime, colour predicts kind statistically: each colour maps to a dominant kind for
  75% of its bushes. The rest are exceptions, so colour is informative but not certain.

## 3. The agent's body (world-owned physics, read by the agent only through interoception)

| variable | range | dynamics |
|---|---|---|
| energy E | 0-10,000 | -1 per tick; plus action costs (section 5); plus food |
| damage D | 0-10,000 | plus hazards; minus recovery per tick (above) |
| collapse | event | if E = 0 or D = 10,000: the body is incapacitated for 5,000 ticks (no actions; energy set to 2,000, damage to 7,000). Life continues; each collapse is recorded as a viability failure |

Viability band (used only by the analyzer to define failures; the agent has only its own
preference prior, `ARCHITECTURE-OPTIONS.md` section 4):
* E > 2,000;
* D < 6,000.

## 4. Observations (exactly what the agent receives)

Each observation is an event `{tick, channel, value, source}`. No observation contains a hidden
variable directly.

| channel | content | imperfection |
|---|---|---|
| view | cells within radius 2 (day) or 1 (night): terrain, bush present, bush colour, fruit present | colour read wrong 5%; fruit-present wrong 5%; whole view dropped 2% |
| wetness | a binary reading of the weather | wrong 10% |
| clock | day/night bit | exact |
| interoception | energy and damage in 32 buckets | exact bucket |
| pain | on damage: amount bucket (4 levels) | exact |
| taste | on eating: `sweet` or `bitter` | exact |
| rustle | on `probe`: a pitch (3 values) correlated with kind (80% the kind's pitch) | noisy by design |
| self | own position | exact |

## 5. Actions (small set, no macros)

| action | duration | energy cost | effect |
|---|---|---|---|
| `look(cell)` | 5 | 2 | a fresh, independent view reading of one visible cell |
| `move(dir)` | 10 (20 at night) | 8 | one cell, if not outside the grid |
| `probe(bush)` | 15 | 5 | **reversible experiment**: a rustle reading; no damage, no fruit |
| `shake(bush)` | 20 | 15 | the causal rule of the bush's kind (may give fruit, may cause damage) |
| `eat` | 10 | 2 | eats a dropped fruit at the agent's cell, if any (taste observed) |
| `rest` | 50 | 0 | nothing; damage recovers at the cell's rate |

No action is "go to food" or "avoid hazard". Every such behaviour must come from the agent's
model and action evaluation.

## 6. Change over time

| change | frequency | known to the agent? |
|---|---|---|
| day/night, weather, season, fruit regrowth | recurring | no rule is given; only observations |
| **regime shift** | unannounced; at pre-drawn times (one per 150,000-250,000 ticks, from the seed) | no |

At a regime shift:
* 50% of the colour → dominant-kind assignments are re-drawn (bushes "re-rooted");
* the bramble layout is re-drawn.

Every shift time is written to the analyzer's world log only.

A life lasts 10^6 ticks by default (about 5 regime shifts). This is a configuration choice for
experiments; the agent cannot detect it.

## 7. Seeds and variants

* A world seed fixes the initial layout, colours, kinds, weather sequence and shift schedule.
  Independent lives use different seeds.
* **Label permutation variant:** the same world with every observation value relabelled through
  a fixed bijection (colours, pitches, channel ids). This is a falsification control.
* **No-structure control world:** kinds independent of colour and rustle, and hazards uniform.
  Nothing is learnable beyond base rates, so no improvement is expected.
