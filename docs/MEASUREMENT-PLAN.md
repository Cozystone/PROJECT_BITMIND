# Measurement plan

Everything here is fixed **before the first run**. Windows, definitions and thresholds are not
changed after results are seen. Any change requires a dated amendment written before the runs it
affects, and is reported.

## 1. Lives

* **Main experiment:** 15 independent lives (world seeds 1-15) of 10^6 ticks each. Each life has
  about 5 unannounced regime shifts.
* **Every baseline and ablation** lives the same 15 world seeds.
* The main life of an agent is never reset or rewound for evaluation. Replicate runs, from
  checkpoints or with other seeds, are separate lives and labelled as such.

## 2. Operational definitions

| term | definition (computed by the analyzer from LifeTrace and the world truth log) |
|---|---|
| **viability exit** | the body leaves the band (energy ≤ 2,000 or damage ≥ 6,000), or a collapse |
| **recovery time** | ticks from a viability exit until the body is back inside the band for 1,000 consecutive ticks |
| **confident error** | the agent's prediction gave the observed outcome a probability ≤ 0.1, while the predicted top outcome had ≥ 0.9 (surprise ≥ log2 10) |
| **failure class** | (action type, the agent's decision context key) for confident errors; (true damage source, action type, context key) for damage events. The context key is what the agent observed; the true source is analyzer-only |
| **recurrence** | a failure of a class that already occurred earlier in the same life |
| **re-adaptation time** after a shift | ticks from the shift until the rolling mean surprise (window 5,000 ticks) on the outcome and view channels returns to within 10% of its mean over the 50,000 ticks before the shift |
| **viability after a shift** | the fraction of the 20,000 ticks after the shift spent inside the band |
| **memory reuse** | a decision whose chosen action's prediction relied on counts last updated more than 100,000 ticks earlier and was correct |

Windows: each life is read in 10 consecutive windows of 10^5 ticks. The first 20,000 ticks are a
burn-in, reported but excluded from H1a slopes.

## 3. Series recorded for every life (CSV per life, plotted per window)

1. viability: energy, damage, time in band, collapses;
2. prediction error (surprise) per channel, and uncertainty (posterior entropy, predictive
   entropy);
3. recovery time after each viability exit;
4. recurrence rate of each failure class per window;
5. information per action: the realised KL of the belief update, expected against realised;
   prediction improvement (change in surprise on the next observations of the same context);
6. resource and risk cost per action: energy spent, damage taken, time;
7. calibration: predicted-probability deciles against observed frequency; confident errors
   listed individually;
8. memory reuse events;
9. adaptation around each regime shift: surprise and viability from -50,000 to +50,000 ticks;
10. resources: resident memory, decision time, trace bytes per window.

## 4. Baselines and controls (same seeds)

| name | what it is | what it controls for |
|---|---|---|
| random | uniform over legal actions | chance and world drift |
| fixed-rule | a hand-written heuristic: shake the nearest bush with fruit, eat, rest when damage is high, avoid brambles once entered; no learning | "a designed policy already does this" |
| probe-only (BITMIND-style) | the same models, but actions maximise information gain only (no risk or cost terms), like D060's disagreement probing | whether the risk-aware objective matters |
| learning off | the agent with models frozen at their priors | whether change comes from learning |
| no change detection | the agent without the surprise-triggered discounting | whether revision after failure matters for H1b |
| fresh-born | the agent born at a shift time into the post-shift world, with no prior experience | whether prior experience helps re-adaptation (H1b-ii) |
| label permutation | the agent in the permuted world | dependence on label values (a hard-coding check) |
| no-structure world | the agent where nothing is learnable | negative control: no improvement expected |

## 5. First research gate (decision rules fixed now)

**H1a (within-life recovery).** For each life, compute the Spearman slope over windows 1-10
(after burn-in) of:
* (i) median recovery time;
* (ii) recurrence rate of failure classes.

H1a is **observed** if, for (i) or (ii), all three hold:
* at least 12 of 15 lives have a negative slope (sign test, p < 0.02);
* the agent's slopes are more negative than the learning-off ablation's (Mann-Whitney, one-sided
  p < 0.05);
* the same does not hold in the no-structure world (fewer than 12 of 15 negative slopes).

**H1b (re-adaptation).** Over all regime shifts of all 15 lives, both must hold:
* **(i) viability:** the agent's viability after shifts exceeds random's (Mann-Whitney,
  p < 0.05);
* **(ii) re-adaptation time:** shorter than the fresh-born control's (prior experience helps)
  and shorter than the learning-off agent's (Mann-Whitney, p < 0.05 each).

**Gate verdict:**

| verdict | condition |
|---|---|
| **supported** | H1a and H1b both observed |
| **partial** | exactly one observed. Reported as partial; not a pass |
| **not supported** | neither observed. Recorded as "keeping it running does not by itself make it adapt better"; a new hypothesis is written; no features are added to rescue H1 |

Confident errors are audited separately in every verdict: calibration in the top decile, and
the count of confident errors per window.

## 6. Falsification plan: separating "growth" from chance, bugs and hard-coding

Before any trend is called a result, these are checked and reported:

1. **Chance:** sign tests across 15 independent lives; no claim from one life. Window-shuffle
   surrogate: recompute the slope with windows randomly permuted 1,000 times; the observed
   slope must be outside the surrogate's 95% range.
2. **Learning is the cause:** the effect must be absent or significantly smaller in the
   learning-off ablation.
3. **Hard-coding:**
   * **label permutation:** the effect must persist with every observation value relabelled;
   * **no-structure world:** no improvement appears;
   * **fixed-rule comparison:** if a 20-line heuristic equals the agent on viability, the claim
     is limited to model-based revision, not viability;
   * **code audit:** `life-agent` contains no world names, channel ids as constants, or
     action-specific branches beyond the action-type enum (a checklist and a grep test in CI).
4. **Bugs:**
   * a trace-consistency checker: every action has its candidate decomposition; every
     observation has a world event; no agent record references a truth field;
   * observer invariance: identical action hashes with tracing on and off;
   * determinism: same seed, same life, same hashes.
5. **Oracle leakage:** a build check that `life-agent` does not depend on `life-world` or
   `life-lab`, and a runtime check that no truth-only field type exists in the interface crate.
6. **Selection:** results are reported for all 15 lives and all baselines; no favourable subset
   and no chosen checkpoint branch.

## 7. Reporting interesting patterns

Every noteworthy pattern seen during runs is written up in this form (`reports/patterns/`):
1. **Observation:** the trace ids, time range and numbers.
2. **At least two interpretations.**
3. **Falsification:** what comparison or perturbation would distinguish them.
4. **Current status:** confirmed / promising / open.
5. **What may not be claimed from it.**
