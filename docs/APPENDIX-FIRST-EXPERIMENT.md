# Appendix to the first experiment: approved amendments and the concrete gate

2026-10-12. Approved scope (and nothing beyond it): minimal interface, ring world, candidate C,
core traces, the prediction time-order and information-barrier checks, numerical and
approximate-update diagnostics, a readout-only baseline, short development runs with profiling,
and the formal configuration. **Not in scope:** action selection, candidate B, a larger network,
new features. Random actions are the first experiment's control condition; nothing about active
selection, self-maintenance, instinct or being alive is claimed from it.

The question of this round, exactly:

> In a small recurrent network updated only by observations that actually arrive, with no external task reward, does experience remain in the connections and contribute to later prediction?

## A1. Prediction and observation are separated in time

Each stored prediction carries `made_at` (the tick it was computed), `last_obs_tick` (the last
observation incorporated before it), the target block, and its scope: **the value of the next
event of that block, whenever it arrives**. The arrival time itself is not predicted in this
round; the arrival of an event is information the agent receives, and the value prediction is
conditional on arrival. These two are kept distinct in the records (`Eval.made_at` vs
`Eval.tick`).

Order inside one tick t, enforced by `Agent::tick` and not reorderable by callers:

1. **evaluate**: for every event (block k, value v) arriving at t, the error
   e_k = onehot(v) − p̂_k uses only the prediction stored with `made_at` < t;
2. **learn**: the plasticity update uses e, the rates r(t−1) that produced that prediction, and
   the eligibility traces up to x(t−1);
3. **incorporate**: the observation o(t) enters the state, x(t) is computed, traces advance;
4. **predict**: p̂_k(t) for every block is computed from r(t) and stored with `made_at` = t.

A copy path (reading o(t) and then predicting o(t)) is impossible in this order: the prediction
evaluated against the event at t was fixed before o(t) existed. Tests:
`crates/life-agent/tests/ordering.rs` (made_at < tick for every evaluation; an iid stream stays
at chance; a deterministic cycle is learned), and `life-lab audit` checks every `Eval` record of
a run.

**Missing and delayed observations.** A dropped reading produces no event, so nothing is
evaluated or updated for that block at that tick (the stored prediction stays and will be
evaluated against the next event that does arrive). A delayed reading (neighbour tokens arrive
one tick after emission) is an event at its arrival tick; the evaluated prediction is the one
stored before that arrival tick. The agent never sees emission times.

## A2. Eligibility traces are an approximation hypothesis, not a gradient

The state step is the normalised leaky integrator x(t) = λ ⊙ x(t−1) + (1 − λ) ⊙ u(t) with
λ_i = τ_i / (τ_i + Δt), Δt = 1 (fixed point u, independent of τ; this replaces the Hasani form in
`ARCHITECTURE-OPTIONS.md` 3.2, whose fixed point τ·u saturates long-τ units). The trace is the
direct-path derivative ∂x_i/∂W_ij under the same step:

    ε^rec_ij(t) = λ_i ε^rec_ij(t−1) + (1 − λ_i) r_j(t−1)        (pre = the rate that entered u(t))
    ε^in_ij(t)  = λ_i ε^in_ij(t−1)  + (1 − λ_i) o_j(t)          (pre = the input that entered u(t))

and the update at an evaluation tick is ΔW_ij = η · L_i · r′(x_i(t−1)) · ε_ij(t−1), with
L = B e (fixed random B) or L = W_outᵀ e (ablation). This **ignores** the recurrent path through
other units and replaces the true error projection by a random one. It is written and reported
as an approximation. `life-lab gradcheck` compares, on an 8-unit network over short sequences,
the accumulated update with a central finite-difference gradient of the summed loss (cosine
similarity and norm ratio, separately for W_rec, W_in, W_out, and for B random vs W_outᵀ). The
check is analysis only; the learner never uses it. Whether the approximate update is useful is
judged only by prediction improvement and ablation.

Weight clipping (|W_rec|, |W_in| ≤ 3; |W_out| ≤ 6) is counted per tick and reported per window
with ‖ΔW‖. A configuration whose clip count keeps growing is reported as runaway hidden by
clipping, not as stable.

## A3. Arms (matched initial network, world, action stream)

| arm | W_in, W_rec | W_out | homeostasis |
|---|---|---|---|
| `full` | learn | learn | on |
| `readout` | fixed random | learn | on |
| `frozen` | fixed | fixed | off |
| `homeo` | homeostasis only | fixed | on |
| `freeze_at` | learn until t₀ = 2·10⁵, then everything off | | |
| `fb_wt` | as full, L = W_outᵀ e | | |
| `ltc` | as full, input-dependent time constant on | | |
| `full` in the shuffled, iid and label-permuted worlds | | | |

If `readout` and `full` are within the minimum effect of each other, the additional benefit of
hidden plasticity is reported as **not shown**. The network is not enlarged to rescue it.

## A4. Homeostasis, defined

Regulated quantity: the **RMS** of r_i over the window (tanh can be negative; the mean would
cancel). Target r*_rms = 0.4. Reason: at |r| ≈ 0.4 the slope r′ = 1 − r² ≈ 0.84, so units stay
responsive (not saturated, |r| → 1, r′ → 0) and not silent (r ≈ 0 carries nothing to the
readout). Every T_h = 500 ticks, the incoming weights of unit i (row i of W_in and W_rec) are
scaled by s_i = clamp(1 + η_h (r* − rms_i), 0.8, 1.25), η_h = 0.05. Logged per application:
rms_i before, s_i, the Frobenius norm of the induced weight change (`Homeo.dw2`), and the next
window's rms_i (the "after" value). A development diagnostic shows whether the scaling moves rms
toward the target. Plasticity ‖ΔW‖ and homeostatic ‖ΔW‖ are separate records; a response change
caused by homeostasis is never counted as memory.

## A5. Probes: "changed" versus "useful"

* **Change (history dependence):** two copies of the agent that differ only in W (checkpoints
  t₁, t₂, or arm vs arm) get a **state reset (x, traces, held inputs ← 0)**, the **same 200-tick
  warm-up stream**, then the same 1,000-tick probe stream; D_hist = mean KL(p̂₂ ‖ p̂₁) over the
  probe's events. State resets happen only on analysis copies.
* **Usefulness:** on **pre-generated, fixed evaluation streams** E_t (a continuation of the world
  forked at checkpoint t with its own seeds, never fed to any main life), mean surprise per event
  of each copy, with learning disabled during evaluation. "Useful" = lower surprise than the
  frozen arm's copy on the same stream.
* Outcomes reported separately: trace present and useful; trace present and not useful; no
  trace.

## A6. Order and re-adaptation, corrected

A fixed random recurrent network also responds differently to ABC and ACB. The "shuffled world
gives D_order ≈ 0" criterion is **removed**. Reported instead: D_order(W(t)) − D_order(W(0)),
D_order(full) − D_order(frozen), structured vs shuffled vs iid worlds, and whether the agent's
post-ABC and post-ACB predictions are closer to the true next-token distributions of the world's
grammar (KL to truth; analyzer only).

Re-adaptation is not "surprise returns to its old level" (a new regime can be harder or more
random). At each structural change t_c, three individuals see the same post-change stream for
50,000 ticks: the main agent (continue), a **fresh-born** clone (new random W, same architecture)
and a **frozen** clone (the main agent's W at t_c, all weight change off). Reported: surprise on
block-0 events at changed sites per 5,000-tick window, the mean over [t_c + 30k, t_c + 50k], the
slope over the last 20k, a converged flag (|slope| < 0.01 bits per 10k), and every case that did
not converge within the horizon.

## A7. Identical conditions for controls

Independent RNG streams: world timing, world symbols, sensor noise, action sampler, network
initialisation, probe generation. Logging consumes no randomness. The world's randomness depends
only on its own streams and the action sequence, so arms with the same world seed and action seed
receive the identical input stream. **This is verified, not assumed:** every run writes a running
hash of (tick, block, value) over all received events and a hash of the action sequence every
1,000 ticks; `life-lab compare` requires equality across arms before any comparison is reported.

## A8. The gate, concretely (fixed before any formal run)

Development seeds **1-3** are for correctness, stability (learning rates, clipping, homeostasis)
and cost; their results are excluded from evidence. Formal seeds **101-110**, run after the code,
world, measure definitions and budget are frozen (tag + guard).

Primary measures, all three required (conjunctive; secondary measures never change the verdict):

| measure | primary statistic | minimum meaningful effect | control | reporting |
|---|---|---|---|---|
| **P1 change** | D_hist(t₁ = 2·10⁵, t₂ = 10⁶) of `full` minus that of `frozen` | ≥ 0.05 bits, and full > frozen in ≥ 9 of 10 seeds | `frozen` (same stream); shuffled and iid worlds reported | per seed, median, IQR, bootstrap 95% CI (1,000 resamples) |
| **P2 usefulness** | S_frozen − S_full in bits/event on block 0 (token here), mean over checkpoints t ∈ {2,4,6,8,10}·10⁵ on E_t | ≥ 0.10 bits, and full < frozen in ≥ 9 of 10 seeds | `frozen`; `readout` reported alongside | same |
| **P3 persistence** | S_frozen − S_full(10⁶) on E_{2·10⁵}, events at sites unchanged since 2·10⁵ (truth) | ≥ 0.05 bits in ≥ 8 of 10 seeds | `frozen`; `freeze_at` reported alongside (interference vs persistence) | same |

Separate verdict, **hidden plasticity benefit**: S_readout − S_full ≥ 0.05 bits in ≥ 8 of 10
seeds on P2's streams → "shown"; otherwise "not shown".

Secondary (reported, not gated): S1 order (A6), S2 re-adaptation (A6), S3 homeostasis
diagnostics, S4 numerics and clipping, S5 compute, memory and trace cost, S6 gradcheck
alignment.

Failures: a life with a NaN/overflow guard trip is **numerically invalid**: counted, reported,
excluded from the primary statistics; more than 2 invalid lives out of 10 → the configuration is
"unstable, not supported". Unconverged re-adaptation cases are reported, never dropped. Probes
with fewer than 30 events are reported as insufficient, not as zero.

Verdict: **supported** if P1, P2, P3 all pass; **partial** if one or two pass (named); **not
supported** otherwise. Code changes that improve results are a new experiment under a new tag;
only changes inside a frozen life count as experience.

## A9. Not implemented in this round

Candidate A (probability tables; the marginal predictor stands in as the floor), candidate B,
any action rule, Patchland, the narrator, a dashboard, trace compression.

## A10. Development phase (seeds 1-3): findings, amendments, frozen configuration

Written after the development runs and before any formal seed. Everything in this section was
measured on seeds 1-3 (200,000 to 1,000,000 ticks) and is excluded from the evidence. Changes
made here are development-phase tuning of initial scales, rates, clipping and homeostasis, as
A8 allows; no gate threshold, world rule, probe window or arm definition from A1-A9 was changed.

**Implemented and verified before tuning.** Workspace `crates/life-{interface,world,agent,trace,
run,lab}`; `cargo test` (5 tests in `crates/life-agent/tests/ordering.rs`: made_at < tick for
every evaluation, iid stream stays at chance for every arm, a deterministic cycle is learned
unless frozen, determinism of hashes, reset keeps weights); `scripts/check_barrier.ps1` (the
agent crate depends on `life-interface` only and contains no world content); `life-lab audit`
on a 100k life: 60,344 evaluations, 0 time-order violations; `life-lab compare`: identical input
and action hashes across `full`, `readout`, `frozen` on the same seed; `life-lab gradcheck`
(n = 8, 80 ticks, 35 evaluations, δ = 0.01): with W_rec = 0 and L = W_outᵀe the accumulated
dry-run update equals the finite-difference gradient (cosine 1.000, norm ratio 1.000 for W_rec,
W_in, W_out+bias); with recurrence (g_rec = 1) cosine 0.978 (W_rec), 0.967 (W_in); with random
feedback B the cosine is seed-dependent (−0.42 to +0.77 over 10 seed/stream pairs) and on a
64-unit checkpoint after 100k ticks of learning it is −0.03 (W_rec) and −0.01 (W_in), while the
readout group stays at 0.95. The random-feedback hidden update is therefore not aligned with the
gradient in this system; whether it helps is judged by prediction only (A2).

**Finding 1, held-input saturation.** With one birth scale for all input columns, the held
blocks (position, interoception) are sustained inputs whose fixed point saturates units: the
frozen arm's r_rms was 0.67 at g_in = 1 and 0.81 at g_in = 2 (x_max 5.5 and 10.5), and
homeostasis then scaled 18 of 64 units down to the gain bound. Amendment: W_in columns are drawn
with std `g_in` = 2.0 for pulse blocks (token here/left/right, action echo) and `g_in_held` = 0.5
for held blocks. Both are birth constants and appear in the birth table (3.5 of
`ARCHITECTURE-OPTIONS.md`). The pulse gain itself changed block-0 surprise by < 0.01 bits
between 1.0 and 4.0; 2.0 was chosen as the middle value with the frozen arm unsaturated
(r_rms 0.55).

**Finding 2, homeostatic runaway.** A unit that cannot reach the RMS target (a slow unit under
pulse input, or a fast unit under sustained input) was scaled every 500 ticks without bound until
its weights hit the clip (2 of 64 units at 100k ticks, 21% of unit-applications saturated by
1,000k in the unbounded version). Amendment to A4: the cumulative homeostatic gain of each unit's
incoming weights is bounded to [1/`h_max`, `h_max`] with `h_max` = 4; applications at the bound
are logged (`Homeo.saturated`), and the weight bound |w| ≤ `w_max_hidden` is enforced inside the
homeostatic step too (`Homeo.clips`). With Finding 1's gains the bound is rarely reached
(0-178 unit-applications per 100k ticks, out of 12,800).

**Finding 3, rates.** Readout rate 0.05 was worse than 0.02 by 0.10 bits on the world (more
churn: ‖ΔW_out‖ 33 vs 13 per 100k ticks) and 0.01 was equal to 0.02 on block 0 but slower on the
position block; `eta_out` = 0.02 is kept. Hidden rate 0.01 or 0.05 with random feedback drives the
network into saturation (r_rms 0.90, 0.99) and makes the full arm worse than the readout-only arm
(2.55 and 2.71 bits versus 2.08); `eta_hidden` = 0.002 is kept. Weight bound `w_max_hidden` = 6
(3σ of the pulse weights), `w_max_out` = 6.

**Finding 4, forced cycles dominate the current site (world property, not changed).** Under
random actions `touch` is chosen one time in five and forces the local emitter into its 3-symbol
cycle for 100 ticks at a 5-tick interval, so 75-77% of token-here events are forced-cycle
emissions and only a quarter follow the site grammar. The world rules were fixed before any
result and stay as specified; the property is disclosed, and the analyzer splits block-0
surprise into forced and free events (`life-lab summary`, `evaluate` field `p2_s_free`). On
development seeds the learning arms reach 1.7-1.9 bits on forced events (the cycle is
deterministic, so this is far from what a predictor that knew site and phase could reach) and
2.8-3.3 bits on free events, which is above chance (2.585): the network's cycle expectations are
wrong when the grammar resumes.

**Finding 5, what the development seeds show (200k ticks, last 100k window, mean of seeds 1-3,
block 0).** frozen 2.87 bits; readout 2.08; full 2.09; full with weight transport (`fb_wt`)
forced-event surprise 1.61 versus 1.72 for random feedback, at the cost of heavy clipping
(22k-185k clip events per 100k ticks against 1k-13k). The `ltc` arm runs (no numerical failure)
with heavy clipping (60k-70k per 100k ticks) and 2.15 bits. The `homeo` arm changes predictions
relative to frozen (2.71 versus 2.87) without any error-driven learning. Expectation recorded
before the formal runs: with random feedback, `full` and `readout` will be within the minimum
effect of each other, so the hidden-plasticity verdict is expected to be "not shown"; P1-P3
compare against the birth network and are expected to pass on the strength of readout
plasticity alone. This expectation does not change the gate; it is written down so that the
formal result can be read against it.

**Probe specifications fixed here (not in A5 in this detail).** Every analysis copy is reset
(x, r, traces, held inputs, stored predictions ← 0) and warmed up on the first 200 ticks of the
stream, which are not scored. The history probe scores the next 1,000 ticks (A5). Usefulness
streams E_t score the next 5,000 ticks so that the per-site subset of P3 has at least 30 events.
P3's "unchanged since 2·10⁵" sites are those not redrawn and not the source or target of a
cross-relation move by any structural change in (2·10⁵, 10⁶]. Streams are generated from the
checkpointed world with analyzer seeds derived from (world seed, t, tag); `evaluate` regenerates
each stream from every arm's own checkpoint and requires equality (A7 at the checkpoint level).
Two analyzer-only reference lines are reported next to the arms: the marginal floor (per-site
Laplace frequencies of the life's own observations up to t; candidate A's stand-in, A9) and the
generative reference (the world's own conditional: grammar row for free emissions, the cycle
symbol for forced ones, both through the misread model). Side analyses, not gated: free-event
surprise; retention (S_frozen − S_full@10⁶) / (S_frozen − S_full@2·10⁵) on E_{2·10⁵}'s unchanged
sites; D_hist and S_birth − S_full in the shuffled, iid and label-permuted worlds; the order probe
at 10⁶ for every arm with the birth network as D_order(W(0)).

**P3 events, operationalised after the first end-to-end development run.** With 4-5 structural
changes per life, each redrawing two grammars and moving one cross relation, every site had been
touched between 2·10⁵ and 10⁶ on development seed 1, leaving no "unchanged site" and no P3
event. The truth log also shows that each site's forced 3-cycle (Finding 4) is never changed by
a structural change. P3 is therefore evaluated on the block-0 events of E_{2·10⁵} whose
generative structure is unchanged: forced-cycle emissions at any site, plus free emissions at
sites neither redrawn nor part of a cross-relation move in (2·10⁵, 10⁶]. The two components are
reported separately with their counts; the threshold (0.05 bits in ≥ 8 of 10 seeds) is unchanged.
On development seed 1 the free component had 0 events and the forced component 414.

**Recorded development picture, seed 1, full pipeline (10 lives, 8 branches, evaluate).**
Streams identical across arms; audits clean (606,732 evaluations, 0 violations per life). P1
D_hist(2·10⁵, 10⁶): full 0.33, readout 0.27, fb_wt 0.54, ltc 0.54, homeo 0.0002, frozen 0,
freeze_at 0. P2 (mean of 5 checkpoints, block 0): frozen 2.87, homeo 2.70, full 2.03, readout
2.04, freeze_at 2.08, ltc 2.11, fb_wt 1.91; marginal floor 1.92; generative reference 0.72. P3
(forced-cycle events, n = 414): frozen 2.99, full 1.68, readout 1.63, fb_wt 1.36, freeze_at 1.74.
Control worlds (same seed): S_birth − S_full was 0.81 (iid), 0.78 (shuffled), 1.04 (label
permuted) against 0.85 in the structured world, and D_hist 0.34-0.37 against 0.33. **Reading
recorded before the formal runs:** the surprise reduction relative to the birth network is of
the same size in a world without temporal structure, so passing P1-P3 against `frozen` would show
that experience changes the connections and lowers later surprise, not that temporal structure
was learned; the structure-specific gain (structured minus control, a side statistic of
`life-lab gate`) is the quantity to read for that, and on seed 1 it is ≈ 0. The order probe shows
the learning arms farther from the grammar's truth than the birth network (KL 2.2 vs 1.3 bits),
consistent with cycle expectations dominating. No re-adaptation case converged under the
|slope| < 0.01 bits/10k criterion with ≈ 300 events per 5k window; `continue` was below `fresh`
and `frozen` in the [30k, 50k] mean at 3 of 4 changes.

**Gate statistics, operationalised.** P1 and P2: median over valid seeds ≥ minimum effect and
positive in ≥ 9 of 10 seeds. P3 and the hidden-plasticity verdict: per-seed effect ≥ minimum
effect in ≥ 8 of 10 seeds (as written in A8). Bootstrap 95% CI of the median, 1,000 resamples,
fixed seed. A seed whose `full`, `frozen` or `readout` life is numerically invalid, or whose arms
did not receive identical streams, is excluded and listed; more than 2 numerically invalid lives
→ "unstable, not supported". A P3 subset with fewer than 30 events is "insufficient" and counts
as not passing for that seed. Constants live in `crates/life-lab/src/gate.rs`.

**Frozen configuration** (`experiments/formal-config.json`, hash recorded in
`experiments/FREEZE.md` at the freeze commit): n = 64, τ ∈ [4, 400] log-uniform, g_rec = 1.0,
g_in = 2.0, g_in_held = 0.5, eta_hidden = 0.002, eta_out = 0.02, eta_h = 0.05, rms target 0.4,
T_h = 500, h_max = 4, w_max_hidden = 6, w_max_out = 6, feedback = random B (full), LTC off; world
as in `CONTINUOUS-WORLD-SPEC.md` (misread 4%/8%, drop 10%, mode dwell 3,000, structural change
every 150k-250k, forced cycle 100 ticks at interval 5, resource 1,000, rescue after 2,000 ticks at
500). Per seed: arms `full`, `frozen`, `readout`, `homeo`, `freeze_at` (t₀ = 2·10⁵), `fb_wt`, `ltc`,
and `full` in the shuffled, iid and label-permuted worlds (10 lives × 10⁶ ticks); fresh-born and
frozen branches of 50,000 ticks at every structural change at or before 950,000; checkpoints
every 50,000 ticks from tick 0; state records every 100 ticks. Seeds: development 1-3, formal
101-110; derived seeds: noise = 7919·s + 3, agent = 104729·s + 11, actions = 15485863·s + 5,
label permutation = s + 5000.

**Cost (measured, development).** 130k-170k ticks/s single-threaded; one 10⁶-tick life ≈ 7 s,
trace ≈ 55 MB, 21 checkpoints ≈ 3 MB; per seed ≈ 10 lives + 8-10 branches + evaluation ≈ 3
minutes; the formal run ≈ 30 minutes and ≈ 6 GB of traces at two seeds in parallel.
