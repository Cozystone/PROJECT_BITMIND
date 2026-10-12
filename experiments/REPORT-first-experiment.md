# Report: first experiment (bitmind-life-first-v0.1)

Formal run of 2026-10-12, seeds 101-110, from the frozen tag `bitmind-life-first-v0.1`
(`experiments/FREEZE.md`; freeze guard passed before the run). Primary statistics:
`results/formal/gate.json`; per seed: `results/formal/seed*/evaluate.json`; secondary:
`results/formal/secondary.json` (`scripts/aggregate_formal.py`). Traces and checkpoints
(5.3 GB) stay on disk and are not committed.

## 0. What was asked and what was done

Question (docs/APPENDIX-FIRST-EXPERIMENT.md): in a small recurrent network updated only by
observations that actually arrive, with no external task reward, does experience remain in the
connections and contribute to later prediction? Random actions are the control condition.
Nothing about action selection, self-maintenance, instinct or being alive is claimed.

Done, in the approved order: (1) minimal interface, ring world, candidate C, core traces;
(2) prediction time-order and information-barrier checks; (3) numerical stability and
approximate-update diagnostics; (4) readout-only baseline; (5) short development runs with
profiling and development-seed tuning (A10); (6) formal configuration, freeze, formal seeds
101-110, gate, this report.

**Gate verdict: supported** (P1, P2, P3 all pass in 10 of 10 seeds). **Hidden-plasticity
benefit: not shown.** Section 6 says what this does and does not mean; the control worlds show
that the passed gate measures connection change and surprise reduction relative to the birth
network, not the learning of the world's temporal structure.

## 1. Verification before any measurement

| check | result |
|---|---|
| prediction time order (`Eval.made_at < tick`, `last_obs <= made_at`) | 5 unit tests pass; `life-lab audit` on the formal `full`, `frozen`, `readout` lives (30 lives, 606k-717k evaluations each): 0 violations |
| no copy path | iid uniform tokens stay at chance (log2 6 = 2.585 bits) for every arm (test) |
| learnability | a deterministic 4-cycle is learned by `full` and `readout`, not by `frozen` (test) |
| determinism | same seed and inputs give identical state and weight hashes (test) |
| information barrier | `scripts/check_barrier.ps1`: `life-agent` depends on `life-interface` only; no world content in the agent crate |
| identical controls (A7) | input and action hashes equal across the seven main arms in 10/10 seeds; action hashes equal across the control worlds; every probe stream regenerated from each arm's own checkpoint was identical |
| approximate update (A2) | `life-lab gradcheck`, n = 8: W_rec = 0 with L = W_outᵀe → cosine 1.000, norm ratio 1.000 for W_rec, W_in, W_out+bias (the trace is the exact derivative); recurrent g_rec = 1 → cosine 0.978 (W_rec), 0.967 (W_in); random B → seed-dependent at birth (−0.42 to +0.77), −0.03 / −0.01 on a trained 64-unit checkpoint |
| numerics | 100 formal lives, 0 NaN/overflow guard trips; x_max over a life ≤ 3.0 (`full`), 3.4 (`fb_wt`), 4.2 (`frozen`) |

## 2. Development phase (seeds 1-3; excluded from evidence)

Recorded in docs/APPENDIX-FIRST-EXPERIMENT.md A10: held inputs saturated the network at a
single input scale (per-block-type birth gains, g_in 2.0 / g_in_held 0.5); homeostasis ran away
for units that cannot reach the RMS target (bounded cumulative gain h_max 4; weight bound inside
the homeostatic step); rates eta_hidden 0.002 (0.01 and 0.05 with random feedback saturate the
network), eta_out 0.02; under random actions ~76% of token-here events are forced 3-cycle
emissions after `touch` (world property, disclosed, not changed); P3 operationalised on
structure-unchanged events (forced cycles plus free emissions at untouched sites). The
expectation recorded before the formal run was: P1-P3 pass against the birth network;
hidden-plasticity benefit "not shown"; control worlds show the same surprise reduction. All three
expectations were met (section 3).

## 3. Formal result (seeds 101-110; medians, IQR, bootstrap 95% CI of the median)

| measure (A8) | statistic | result | rule | verdict |
|---|---|---|---|---|
| P1 change | D_hist(2·10⁵, 10⁶) of `full` − `frozen` | 0.270 bits, IQR 0.251-0.278, CI 0.250-0.279; > 0 in 10/10; ≥ 0.05 in 10/10 | median ≥ 0.05, > 0 in ≥ 9/10 | PASS |
| P2 usefulness | S_frozen − S_full, block 0, mean over t ∈ {2,4,6,8,10}·10⁵ on E_t | 0.604 bits, IQR 0.494-0.759, CI 0.487-0.800; 10/10 | median ≥ 0.10, > 0 in ≥ 9/10 | PASS |
| P3 persistence | S_frozen − S_full(10⁶) on E_{2·10⁵}, structure-unchanged events (n ≈ 466 per seed) | 0.958 bits, IQR 0.842-1.134, CI 0.838-1.154; ≥ 0.05 in 10/10 | ≥ 0.05 in ≥ 8/10 | PASS |
| hidden plasticity | S_readout − S_full on P2's streams | −0.005 bits, IQR −0.033-0.020, CI −0.035-0.024; > 0 in 4/10; ≥ 0.05 in 0/10 | ≥ 0.05 in ≥ 8/10 | not shown |

Valid seeds 10/10; invalid lives 0; P3 insufficient 0. Per-arm medians over seeds:

| arm | D_hist(2·10⁵, 10⁶) | S_here on E_t (P2) | S_here, free events only | S_here(10⁶) on E_{2·10⁵}, structure-unchanged | same copy at 2·10⁵ |
|---|---|---|---|---|---|
| `frozen` (birth network) | 0 | 2.714 | 2.747 | 2.743 | 2.743 |
| `homeo` | 0.0003 | 2.661 | 2.678 | 2.667 | 2.666 |
| `readout` | 0.208 | 2.082 | 3.227 | 1.747 | 1.790 |
| `full` | 0.270 | 2.096 | 3.230 | 1.732 | 1.778 |
| `freeze_at` (2·10⁵) | 0 | 2.103 | 3.181 | 1.778 | 1.778 |
| `fb_wt` (weight transport) | 0.414 | 1.942 | 3.332 | 1.460 | 1.685 |
| `ltc` | 0.372 | 2.096 | 3.269 | 1.711 | 1.848 |
| marginal floor (analyzer) | | 1.991 | | | |
| generative reference (analyzer) | | 0.732 (free events: 1.934) | | | |

Control worlds (`full` agent, same seeds): D_hist 0.311 (shuffled), 0.235 (iid), 0.266 (label
permuted) against 0.270 in the structured world; S_birth − S_full 0.615 (shuffled), 0.633 (iid),
0.661 (permuted) against 0.604 (structured). Structure-specific gain (structured minus control):
−0.004 (IQR −0.023 to 0.006), −0.018 (−0.043 to −0.009), −0.014 (−0.115 to 0.040).

## 4. The six categories, kept apart

1. **Transient state change.** Present in every arm (r_rms 0.42 in `full`, 0.54 in `frozen`;
   x_max ≤ 4.2) and excluded from every probe by construction: analysis copies are reset and
   warmed up, so D_hist(`frozen`) = 0 exactly although the frozen network's predictions vary with
   its state during life. No measure in this report counts state as memory.
2. **Homeostatic change.** In the `full` arm the homeostatic weight change per 100k ticks
   (‖ΔW‖ 5.6) is five times the error-driven hidden change (1.0), yet the `homeo` arm, which has
   only this change, shows D_hist 0.0003 bits and lowers surprise relative to `frozen` by 0.05
   bits only (2.661 vs 2.714). Bounded gain reached by 641 unit-applications per 100k ticks in
   `full` (of 12,800) and 7,707 in `ltc`. Homeostatic change is reported, not counted as
   memory.
3. **Experience-based connection change.** D_hist(`full`) 0.270 bits (P1). The readout weights
   alone give 0.208 (`readout`), so about three quarters of the full arm's history dependence sits
   in W_out; `freeze_at` gives exactly 0 (nothing changes after 2·10⁵), which confirms that D_hist
   measures weight change in (2·10⁵, 10⁶] only. The change is of the same size in worlds without
   temporal structure (0.235-0.311).
4. **Persistent memory.** What was in the connections at 2·10⁵ is still there at 10⁶ and has not
   been overwritten by 800k ticks of further learning and 3-5 structural changes: on E_{2·10⁵}'s
   structure-unchanged events the 10⁶ copy (1.732) is slightly better than the 2·10⁵ copy
   (1.778); retention ratio median 1.09; the `freeze_at` arm (weights frozen at 2·10⁵) scores
   1.778, identical to its 2·10⁵ copy as it must. What persists is almost entirely the forced-cycle
   component (forced-only: `full` 1.651 vs `frozen` 2.725); the free-event component had 0-44
   events per seed (two seeds had no untouched site).
5. **Later prediction improvement.** Relative to the birth network, yes: 0.604 bits on block 0
   (P2), in every seed. Relative to anything that knows the world, no: the `full` arm (2.096) is
   0.1 bits **worse** than a per-site marginal-frequency predictor computed from the agent's own
   observations (1.991), 1.36 bits above the generative reference (0.732), and on free (grammar)
   events it is **worse than at birth** (3.23 vs 2.75: the learned expectations are cycle
   continuations, which are wrong when the grammar resumes). The same improvement appears in
   the iid, shuffled and label-permuted worlds (structure-specific gain ≈ 0), so the improvement
   is not evidence that the world's temporal structure was learned.
6. **Hidden-plasticity contribution versus the readout-only baseline.** Not shown: `readout` −
   `full` median −0.005 bits, positive in 4 of 10 seeds, never ≥ 0.05. With the exact learning
   signal (`fb_wt`, L = W_outᵀe) hidden plasticity does contribute: 0.106-0.170 bits better than
   `full` in 10 of 10 seeds (median 0.15; D_hist 0.414; P3 1.460), but its hidden weights are at
   the bound 50k times per 100k ticks in the last window and the count grows through the second
   half of life in 10 of 10 seeds: runaway contained by clipping, "not stable" by A2's
   definition. The `ltc` arm is equal or worse than `full` (median −0.03). The random-feedback
   update is unaligned with the gradient (gradcheck ≈ 0 on a trained network), which is the
   proximate reason the hidden rule adds nothing in the `full` arm.

## 5. Secondary measures (reported, not gated)

* **S1 order probe (10⁶, 160 contexts per arm).** D_order: `frozen` 0.005, `homeo` 0.003,
  `freeze_at` 0.088, `full` 0.168, `readout` 0.190, `ltc` 0.458, `fb_wt` 1.268; so
  D_order(W(10⁶)) − D_order(W(0)) = 0.163 for `full`. KL to the grammar's truth: `frozen` 1.29,
  `full` 2.38, `readout` 2.41, `fb_wt` 3.12. The learning arms are farther from the grammar than
  the birth network: after A,B,C they expect cycle continuations, not the grammar's next symbol.
* **S2 re-adaptation (44 structural changes with ≥ 50k ticks remaining).** Mean block-0 surprise
  at changed sites over [t_c + 30k, t_c + 50k]: `continue` 1.98, `frozen` clone 2.08, `fresh`
  clone 2.12 (medians). `continue` below both clones in 44/44 cases; `fresh` below `frozen` in
  19/44. Converged (|slope| < 0.01 bits/10k): 4 (`continue`), 3 (`fresh`), 2 (`frozen`) of 44;
  all 40+ unconverged cases are listed in `secondary.json`. The 5k windows hold ≈ 300 events,
  which makes the slope criterion rarely satisfiable; this is reported, not relaxed.
* **S3 homeostasis.** Mean rms reaches the target 0.40 within ~30k ticks in every learning arm
  (last-window mean 0.41-0.42); `frozen` sits at 0.54 without regulation. Saturated
  unit-applications: `full` 641, `fb_wt` 468, `ltc` 7,707 per 100k (of 12,800).
* **S4 numerics and clipping.** 0 guard trips. Hidden-weight clip events in the last 100k window
  (median over seeds): `full` 242 (20k per life; growing in the second half in 1/10 seeds),
  `fb_wt` 50,430 (446k per life; growing in 10/10), `ltc` 12,452 (284k; growing in 0/10),
  `readout`/`frozen`/`homeo` 0. Readout clips 0 everywhere. Homeostatic clips per life: `full`
  1,562, `fb_wt` 34,651, `ltc` 15,638.
* **S5 cost.** 100 lives, median 7.6 s per 10⁶-tick life (132k ticks/s, single thread), traces
  5.05 GB, checkpoints 0.26 GB, 88 branches, 10 evaluations; the whole formal run took about 20
  minutes with four seeds in parallel.
* **S6 gradcheck.** Section 1; the alignment of the random-feedback hidden update with the
  gradient did not develop during life (−0.03 on a 100k checkpoint).

## 6. What this does and does not show

Shown, in every one of 10 seeds, with no external reward or label: the connections of a
64-unit recurrent network change through the observations that arrive (P1); the change persists
through 800k further ticks and 3-5 structural changes without being overwritten (P3, retention
≥ 1); and it lowers later surprise relative to the network at birth (P2). In the literal sense of
the pre-registered gate, experience remains in the connections and contributes to later
prediction: **supported**.

Not shown, and the control worlds say so: that the world's **temporal structure** was learned.
The surprise reduction is of the same size in a world with iid symbols, a block-shuffled world
and a label-permuted world; the agent is worse than a per-site marginal predictor and worse than
its own birth network on grammar events; the deterministic forced cycles that dominate the input
are predicted at 1.65 bits where the generative reference is ≈ 0. What remains in the connections
is, as far as these measures can tell, per-input and per-site frequency statistics plus the
de-saturation of a random readout, carried mostly by the readout weights. The error-driven
hidden rule with random feedback adds nothing measurable (**not shown**); with weight transport
it adds 0.15 bits but runs into the weight bound.

Not claimed: action selection, self-maintenance, instinct, agency, a self, memory in any
psychological sense, consciousness, general intelligence, real-world readiness. Random actions
were the control condition throughout.

Two properties of the setup limit what the first experiment could resolve, and both were fixed
before results and left unchanged: under random actions `touch` makes the current site's stream
a forced 3-cycle three quarters of the time, so the grammar is rarely observed at the current
site; and a linear readout on an additively coded state cannot represent the site × previous
token conjunction that predicting a per-site cycle needs, while the hidden rule that could shape
such features has no usable learning signal under random feedback.

## 7. Candidates for the next experiment (not executed; each is a new tag after approval)

1. **Learning signal for hidden plasticity.** The bottleneck is alignment, not the trace
   (gradcheck: trace exact at W_rec = 0, 0.97 with recurrence, ≈ 0 with random B). Candidates
   that stay local: feedback weights that learn toward W_outᵀ with a local rule (Kolen-Pollack
   type), or sign-concordant feedback. The `fb_wt` arm bounds what this could gain (≈ 0.15 bits)
   unless the clipping runaway is also understood.
2. **Exposure to the grammar.** The random sampler's touch frequency, or the forced duration,
   decides how much grammar the agent ever sees at its site. Changing either is a world/sampler
   change and therefore a new experiment, to be pre-registered with the forced/free split as a
   primary measure.
3. **Conjunctive features.** Whether any arm can predict the per-site cycles (1.65 bits now,
   ≈ 0 achievable) is a sharp, cheap test of representation; it needs no new world.
4. **Churn.** D_hist over the last 100k ticks equals D_hist over 800k (0.66 vs 0.67 bits in
   development): the readout never settles at a constant rate. A measure of "settled" change
   versus churn would separate accumulated experience from ongoing fluctuation better than
   D_hist does.

## 8. Files

`results/formal/gate.json`, `gate.txt`, `secondary.json`, `COMMIT`; per seed `evaluate.json`,
`evaluate.txt`, `compare_main.txt`, `compare_variants.txt`, `audit.txt`, `changes.txt`,
`summary_{full,frozen,readout}.txt`, `readapt_<t_c>.json`; development: `docs/APPENDIX-FIRST-
EXPERIMENT.md` A10. Code frozen at tag `bitmind-life-first-v0.1`.
