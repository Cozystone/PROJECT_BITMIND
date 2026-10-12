# Measurement plan (revised 2026-10-12)

Everything here is fixed before the first run. The evaluation values are computed by the
analyzer from the traces and the world's truth log; **none is fed back to the learner**.
When prediction learning is used, the only target is the observation the agent actually
received; hidden world state is never a target.

## 1. Lives, clones and controls

* **Main life:** one agent per world seed, 10⁶ ticks, memory never reset. 10 independent
  world seeds (lives) for the first experiment.
* **Clones** are separate individuals, named as such. Each clone of a life receives the
  identical seeded action stream and the identical world stream (where the world is the same),
  so differences come from the agent, not from different experiences.
* The end of the observation period is a configuration choice; it is not called death.

| arm | what differs | question it answers |
|---|---|---|
| C (main) | — | the hypothesis |
| C, plasticity off | η = η_out = 0 from birth; homeostasis on | is the effect dynamics only? |
| C, plasticity off after t₀ | learning until t₀ = 2 × 10⁵, then frozen | do traces persist and keep acting? |
| C, homeostasis off | η_h = 0 | is regulation needed, and is it doing the learning? |
| C, Wᵀ feedback | B = W_outᵀ | does random feedback limit learning? |
| C, LTC flag on | input-dependent τ | H-LTC-1 (expected: no difference) |
| A (tables) | the explicit baseline | what a hand-designed model of this world achieves |
| random predictor | predicts each block's running marginal | floor |
| **shuffled world** | C in the order-shuffled world | memory tests must fail |
| **iid world** | C in the iid world | same |
| **label permutation** | C in the relabelled world | results must be unchanged |

## 2. The six "experience leaves traces" measures (definitions)

All probes are **offline**: the analyzer takes a checkpoint of the agent (x and W), runs the
probe on a copy, and discards the copy. The main life never sees a probe.

1. **History dependence.** Take two checkpoints of the same life at ticks t₁ < t₂ that are
   followed by the same 200-tick input window (the analyzer searches the truth log for
   repeated windows; the world is built so that short windows recur). Feed the window to both
   copies with **the fast state reset to zero** (x ← 0) so only W differs. Measure
   D_hist = mean KL(p̂ at t₂ ‖ p̂ at t₁) over the window. Compare with the plasticity-off arm
   (expected ≈ 0).
2. **Persistence.** For a trace created by a distinctive event sequence S at tick t (an
   analyzer-detected novel sub-grammar), measure the prediction of S's continuation at
   t + Δ for Δ ∈ {10³, 10⁴, 10⁵} with state reset. Persistence = the fraction of Δ at which
   the prediction of S's continuation beats the marginal predictor by ≥ 0.5 bits.
   **Dynamics horizon:** H_dyn = the lag at which the autocorrelation of x falls below 0.05
   (measured on the plasticity-off arm). A trace counts as lasting only if it survives both the
   state reset and Δ ≥ 10 · H_dyn.
3. **Consequence.** Freeze W at t and at t − 10⁴; run both on the same held-out future
   stream (from the truth log, next 5,000 ticks); Δsurprise = mean surprise difference. Positive
   means the later weights predict the future better.
4. **Order.** For sequences ABC and ACB that both occur at a site, after state reset present
   each and measure the predicted next-symbol distributions; D_order = KL between them.
   Compare with the shuffled world (expected ≈ 0).
5. **Revision.** At each unannounced structural change, measure the surprise on the changed
   sites from −20,000 to +50,000 ticks; T_revise = ticks until the rolling mean (window 2,000)
   returns within 10% of its pre-change level. Compare the plasticity-off arm (no return
   expected) and a **fresh-born** clone started at the change time.
6. **Reuse (savings).** When a grammar reappears after an absence, compare T_revise on its
   reappearance with T_revise on its first appearance; savings = 1 − T_second / T_first.

Also recorded, per window of 10⁵ ticks:
* surprise and predictive entropy per block;
* calibration: predicted-probability deciles vs observed frequency; count of confident
  errors (p̂[v] ≤ 0.1 with max p̂ ≥ 0.9);
* ‖ΔW‖ per window, split into plasticity and homeostasis;
* maintenance failures, external rescues, resumptions (separately);
* numerical health: max |x|, NaN guard trips, solver step;
* resources: resident memory, ticks per second, trace bytes.

## 3. Chain tests (event → response → ΔW → later response)

For each analyzer-flagged event (a novel sub-grammar first appearance, a structural change):
* the `provenance` tool must link the event's observations to the errors e(t), to the
  eligibility traces that were non-zero, to the ΔW entries of that window, and to the first
  later prediction that differs from the plasticity-off clone;
* **blocking test:** a clone with plasticity disabled only during [t_event, t_event + 500] must
  lose the later difference. If the difference remains, the chain is not the cause.

## 4. Gate for the first experiment (fixed now)

Over the 10 lives, candidate C is **supported** on the first question if all hold:
* measure 1 (history, with state reset) is positive and exceeds the plasticity-off arm in
  ≥ 8 of 10 lives (sign test p < 0.06), and ≈ 0 in the shuffled and iid worlds;
* measure 2: lasting traces (Δ ≥ 10 · H_dyn, state reset) exist in ≥ 8 of 10 lives;
* measure 3: Δsurprise > 0 in ≥ 8 of 10 lives;
* measure 4: D_order exceeds the shuffled world in ≥ 8 of 10 lives;
* measure 5: T_revise finite in ≥ 8 of 10 lives and shorter than the plasticity-off arm's
  (which should not revise);
* the label-permutation arm reproduces all of the above within noise;
* the blocking test removes the effect in ≥ 8 of 10 flagged events.

Measure 6 (reuse) is reported, not gated, in this stage.

**Partial:** some but not all hold. Reported as partial, with the failing measures named.
**Not supported:** fewer than three measures hold. Candidate C is recorded as not supported for
the question; the next step is a new hypothesis, not a larger network or a richer world.

## 5. Distinguishing growth-like trends from chance, bugs and hard-coding

1. Chance: 10 independent lives; sign tests; window-shuffle surrogates (1,000 permutations).
2. Dynamics vs plasticity: plasticity-off arm; state-reset probes; H_dyn threshold.
3. Regulation vs learning: homeostasis-off arm, and homeostasis-only arm (η = 0, η_h > 0).
4. Hard-coding: label permutation; iid and shuffled worlds; the agent crate's dependency and
   content audit (no world names, channel ids, site indices).
5. Bugs: trace-consistency checker; observer invariance (tracing on/off gives identical
   state hashes); determinism (same seeds, same hashes); NaN/overflow guards.
6. Selection: all lives, all arms reported; no favourable checkpoint branches.

## 6. Reporting patterns

Every noteworthy pattern: observation (trace ids, ticks, numbers); at least two
interpretations; the comparison or perturbation that would separate them; status (confirmed /
promising / open); what may not be claimed.
