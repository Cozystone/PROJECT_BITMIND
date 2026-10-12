# Research question (revised 2026-10-12)

## What this system is not for

It is not a foraging, survival, optimisation or task agent. No external task, correct answer
or success reward is part of its learning. Useful behaviour may appear later; none is put in
as a learning target now.

Four things are kept apart, in code and in reports:

| kind | examples | status |
|---|---|---|
| external task | predicting a label, collecting fruit, maximising a survival score | **forbidden** as a core learning objective |
| conditions of existence | a resource that activity consumes, physical limits, a range the body must stay in | allowed, **disclosed as designed by us** (`CONTINUOUS-WORLD-SPEC.md` section 4) |
| learning principle | update from the error of the agent's own next-observation prediction; local plasticity; activity regulation | necessary; **not evidence of being alive or aware** |
| researcher's metrics | measures of change, memory and adaptation | computed by the analyzer; **never given to the learner** |

"No external purpose" does not mean "no update rule". And a survival reward renamed
"homeostasis" is still a reward: the only teaching signal allowed is the observation that
actually arrives next.

## The one question of this stage

> Does experience leave lasting traces in the system, and do those traces change what it predicts and does later?

Operationally (definitions and windows in `MEASUREMENT-PLAN.md`):
1. **history dependence:** the same present input produces a different response because of
   different past events;
2. **persistence:** that difference survives long after the events, and survives a reset of
   the fast state (so it lives in the connections, not only in the activity);
3. **consequence:** the connection changes alter later predictions (and, in a later stage,
   actions);
4. **order:** differently ordered event sequences are distinguished;
5. **revision:** when a relation in the environment changes, the old trace is updated;
6. **reuse:** part of the experience is reused under new conditions (savings on re-learning).

Reacting to input, or showing complex oscillations, is not learning. A clone with plasticity
off has the same state dynamics; whatever it also shows is dynamics, not memory.

## Hypothesis H1 (operational, falsifiable)

A small continuous-time recurrent network whose only plasticity is a local three-factor rule
driven by its own next-observation prediction error (`ARCHITECTURE-OPTIONS.md`, candidate C)
shows 1-6 above in a continuous world with temporal structure, beyond three controls:
* the same network with plasticity off (same seed, same action stream);
* the same network in a world with the same marginal statistics but no temporal structure;
* the same network in a world whose event order is shuffled.

## What refutes H1

Within the fixed budget (`FIRST-EXPERIMENT.md`), if lasting experience-dependent change beyond
the controls is not reproduced in the pre-registered majority of independent lives, candidate C
is recorded as **not supported**. The world is not made more complex and the network is not
made larger to postpone that conclusion. A new hypothesis is written instead.

## Not the goal, not claimed

Consciousness, feelings, curiosity, a self, sleep, autonomy, general intelligence, real-world
readiness. Internal variables are named by what they are. The end of an observation period is
not called death or a lifetime.
