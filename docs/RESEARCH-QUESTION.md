# Research question

## Question

> Does this system, living one continuous life, recover after failures and adapt better as experience accumulates?

**Living one continuous life** means:
* one world clock that never resets;
* one agent whose memory is never wiped;
* observations arriving as events;
* actions that cost time, energy and risk.

The agent never learns when an evaluation starts or ends. Outside observers read sliding time
windows.

## Hypothesis H1 (operational)

An agent that keeps a model of the world and of its own body, maintains beliefs with explicit
uncertainty, compares predictions with what it observes, revises the model where the two
disagree, and chooses actions by the expected consequences for its viability and for the
uncertainty of its beliefs will show, within one life:

* **H1a (recovery).** After failures, the time to recover shrinks, or the same class of failure
  recurs less often, later in life than earlier.
* **H1b (re-adaptation).** After an unannounced partial change of the world's structure,
  prediction and viability recover faster than under a random baseline. They also recover
  faster than under the same architecture with learning disabled, measured from the moment
  of change.

"Failure", "recovery", "class of failure" and the windows are defined operationally in
`MEASUREMENT-PLAN.md` before any run. They are not changed afterwards.

## What would refute H1

H1 is not supported if, across the pre-registered number of independent lives, **neither** H1a
**nor** H1b appears in the agent's own life (relative to its early life and to the baselines,
as defined in `MEASUREMENT-PLAN.md`).

In that case the result is recorded as: "keeping this system running does not by itself make
it adapt better". **Features are not added to rescue the hypothesis**; a new hypothesis is
written instead.

H1 is also not supported by a result that:
* appears only in one life or one world seed;
* appears equally in the learning-disabled ablation;
* disappears under label permutation;
* is explained by a trivial bug or a fixed behavioural loop (falsification plan,
  `MEASUREMENT-PLAN.md` section 6).

## What is deliberately not the goal

* A correctness rate on queries. There is no global "0 wrong" objective. Confident beliefs are
  audited separately: confidence must match accuracy (calibration), and confident errors are
  counted.
* Language, perception of real sensors, OS control, web access, human data.
* Any claim about consciousness, feelings, curiosity, a self, sleep, autonomy or general
  intelligence. Internal variables are named by what they are (energy, damage, prediction
  error), not by mental terms.

## First research gate (summary; exact rules in MEASUREMENT-PLAN.md)

After the first implementation, success is not declared by a single score. Both of these must
hold across independent lives:
1. within one life, recovery time after failures falls, or recurrence of the same failure class
   falls;
2. after a partial structural change of the world, prior experience leads to faster
   re-adaptation than the random baseline.

If neither appears, H1 is recorded as not supported, and the next step is a new hypothesis, not
more features.
