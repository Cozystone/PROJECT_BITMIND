# Architecture options (revised 2026-10-12)

The earlier choice (an explicit categorical/Dirichlet state-space model with expected-free-energy
action selection) is **demoted** to an interpretable baseline (candidate A). It is not the
cognitive core. Section 1 discloses what it presupposes; sections 2-3 compare three
candidates on the user's questions and specify the chosen one exactly.

## 1. What candidate A (explicit probability tables) presupposes

| element | given at birth | learned | never changes |
|---|---|---|---|
| variables and their categories | every latent variable (per-object attribute, hidden mode), every observation channel and its alphabet | — | the set of variables |
| observation (sensor) model | the form p(reading \| true value) as a confusion table with Dirichlet prior | the counts | the table's shape |
| transition and outcome models | p(outcome \| action, context key) as count tables; the **context key** (which observed features matter, and in what combination) | the counts | the keys |
| action effects | the fact that each action has an outcome distribution per key | the distribution | which features define a context |
| preference / maintenance range | an explicit log-preference over interoceptive buckets | — | the preference |
| what changes with experience | numbers in fixed tables | | |
| what cannot be represented | a temporal relation not expressible in the given keys (e.g. "the token two events ago at the neighbouring site"); a new latent variable | | |

It is a pre-designed world model whose numbers are updated. That is a fair baseline ("what a
hand-designed model of this world achieves") and a diagnostic tool, not a system that can
represent temporal relations it was not told about.

## 2. Three candidates on the same questions

A = explicit probability tables (above). B = small recurrent spiking network (LIF) with
online local plasticity. C = small continuous-time rate network (CT-RNN, with an optional
LTC-style input-dependent time constant) with online local plasticity.

| question | A | B | C |
|---|---|---|---|
| remembers order and intervals of irregular events? | only through its context keys (fixed look-back; intervals only if binned into the key) | yes in principle: membrane and synaptic time constants plus recurrence hold recent history; order is in the state trajectory | yes in principle: leaky-integrator state with several time constants; order and timing in the trajectory |
| what happens to internal state with no input? | nothing (tables are static) | decays to rest (τ_mem, τ_syn), possibly self-sustained activity through recurrence | decays toward the fixed point x = b-driven value at rate 1/τ; may oscillate through recurrence; bounded (LTC Theorem 2 argument) |
| state change vs lasting learning: how separated? | trivially: only counts change | state = (I, U); parameters = (W, V). Separate logs; a state reset test isolates W | state = x; parameters = (W_in, W_rec, W_out). Same separation; state reset test |
| what updates connections without targets or reward? | nothing but counts of what happened | needs a third factor; without targets the only self-generated signal is the error of the network's own prediction of the next input (rate-decoded) | the same: next-observation prediction error, computed where the prediction is read out |
| is the needed information local to the synapse? (Neftci Box 3) | n/a | presynaptic trace, postsynaptic state: yes; the error must be routed to hidden units (random feedback: yes; BPTT: no) | same; one eligibility trace per synapse (O(N²)), error routed by fixed random feedback |
| runaway or silence prevention | n/a | spike-rate homeostasis (threshold adaptation or synaptic scaling); weight bounds; risk of over-synchrony | activity homeostasis by slow synaptic scaling; weight bounds; bounded state by the semi-implicit step |
| cost on an ordinary PC | KB-MB; microseconds per update | N = 128 LIF with Δt = 1 tick: ~50k ops per tick; traces O(N²); surrogate derivative needed for the prediction readout | N = 64-128: ~20-40k multiply-adds per tick; one life of 10⁶ ticks in minutes; memory < 50 MB |
| can internal logs and ablations locate the cause of a change? | yes, trivially | yes with effort: spike rasters, membranes, traces, ΔW; many knobs (threshold, reset, surrogate, refractory) each needing ablation | yes: x(t), e(t), ε(t), ΔW per window; fewer knobs |
| violates project rules? | no | no, if plasticity is local and no BPTT | no, if plasticity is local and no BPTT. **LTC as published requires BPTT**; only its state equation is used |
| forced reuse of old BITMIND assets | none | none | none |

**Why not B first.** Spiking adds the non-differentiable readout problem (a surrogate
derivative is needed even for our local rule), a coding decision (rate vs timing) that our
first question does not need, and more regulation knobs (threshold, reset, refractory,
synchrony). None of these helps answer "does experience leave traces". B is **deferred**:
it becomes the next candidate only if C answers the first question and the next question
needs event-driven computation.

**Why not A as core.** It cannot represent relations outside its keys; it would make
"experience leaves traces" true by construction for the keyed relations and false by
construction for the others. It stays as the interpretable baseline that tells us what a
hand-designed model of the same world achieves.

**Choice: C**, in the smallest form, with the LTC time-constant term as a flag (default off).

## 3. Candidate C, exactly

### 3.1 Encoding (fixed; designed by us)

Observation vector o(t) ∈ ℝᵐ built from the events of `CONTINUOUS-WORLD-SPEC.md` section 3:
* token here: one-hot over 6 symbols, held for 1 tick when an event arrives, else zero;
* token nearby: two one-hot blocks of 6;
* position: one-hot over 8;
* interoception: one-hot over 16 buckets (held until the next reading);
* action echo: one-hot over 5.

m = 6 + 12 + 8 + 16 + 5 = 47. No channel is special-cased in the agent; the agent sees one
vector.

### 3.2 State equation (one step Δt = 1 tick, semi-implicit Euler as in Hasani Eq. 3)

Hidden state x ∈ ℝᴺ, N = 64 (first experiment), rate r = tanh(x).

    u(t) = W_rec r(t) + W_in o(t) + b
    g(t) = 0                              (flag LTC off: a plain CT-RNN)
    g(t) = σ(W_g r(t) + W_gi o(t) + b_g)   (flag LTC on; W_g fixed random, not learned)
    x(t+1) = ( x(t) + Δt · ( u(t) + g(t) ⊙ A ) ) / ( 1 + Δt · ( 1/τ + g(t) ) )

τ ∈ ℝᴺ fixed per unit, drawn log-uniform in [4, 400] ticks at birth (a spread of time
constants, so the state can hold information over several horizons). A is a fixed vector.
The denominator keeps the update stable at Δt = 1 (the LTC paper's reason for the fused step).

### 3.3 Prediction and error (the only teaching signal)

For each observation block k (a categorical channel), a readout predicts the next value:

    z_k(t) = W_out,k r(t) + c_k            logits
    p̂_k(t) = softmax(z_k(t))               prediction for the block's next event

When an event of block k actually arrives at t with value v:

    e_k(t) = onehot(v) − p̂_k(t)            prediction error (local to the readout)
    surprise_k(t) = −log p̂_k(t)[v]         logged; not used by learning beyond e_k

Between events a block produces no error (no update), so sparse channels do not drown the
others.

### 3.4 Plasticity (local; forward in time; no BPTT)

* Readout (delta rule): ΔW_out,k = η_out · e_k(t) · r(t)ᵀ. Local: postsynaptic error ×
  presynaptic rate.
* Hidden units get a learning signal by **fixed random feedback** (direct feedback alignment,
  Neftci Fig. 5c):

      L(t) = B e(t)          B ∈ ℝᴺˣᵐ fixed random at birth; e = concatenated block errors

  Ablation arm: B = W_outᵀ (weight transport).
* Eligibility traces, one per synapse, forward in time (SuperSpike form without BPTT):

      ε_ij(t+1) = λ_i ε_ij(t) + (1 − λ_i) · r′(x_i(t)) · pre_j(t)

  with pre_j = r_j for W_rec and o_j for W_in, λ_i = exp(−1/τ_i) (the trace decays with the
  unit's own time constant, so credit is assigned over the horizon the unit integrates).
* Three-factor update at every tick where some e ≠ 0:

      ΔW_rec,ij = η · L_i(t) · ε_ij(t),    ΔW_in,ij = η · L_i(t) · ε^in_ij(t)

* Weight bounds: |W| ≤ w_max, clipped.
* **Activity regulation (innate, slow; not learning):** every T_h = 500 ticks, for each
  unit, the incoming weights are scaled by (1 + η_h (r* − r̄_i)) where r̄_i is the unit's
  mean |r_i| over the window and r* = 0.2 a fixed target. This keeps units from saturating or
  going silent. It is logged as `homeostasis`, separately from plasticity, and is ablated in
  one arm.

Locality check (Neftci Box 3): each ΔW_ij uses W_ij, the presynaptic trace ε_ij (built from
pre_j and the postsynaptic r′(x_i)), and L_i, which is a single modulating signal delivered to
unit i. Nothing from another synapse or another unit's state is used.

### 3.5 What is given at birth, what is learned, what never changes

| element | given | learned | fixed forever |
|---|---|---|---|
| encoding of channels, m = 47 | yes | | yes |
| N, τ distribution, A, B, W_g (if LTC on) | yes (seeded) | | yes |
| W_in, W_rec, W_out, b, c | random init | **yes, continuously** | |
| η, η_out, η_h, r*, w_max, T_h | yes | | yes |
| the body's resource physics and rescue | in the world | | yes |
| any preference over interoception | **none** | | |

### 3.6 Action (first experiment)

Actions come from a fixed seeded random sampler outside the agent (`FIRST-EXPERIMENT.md`).
The agent's output is prediction only. Action selection from the agent's own predictions is
the next stage, after the first gate; two candidate rules are recorded there, neither is
implemented now.

## 4. Free-energy-principle terms, mapped honestly

| FEP term | in C | logged |
|---|---|---|
| beliefs about world and self | the state x(t) and the weights (no explicit posterior) | x(t) per tick (subsampled), W per window |
| uncertainty of beliefs | entropy of p̂_k(t) per block (predictive uncertainty only; no parameter uncertainty is represented) | per prediction |
| prediction of the next observation | p̂_k(t) | per block |
| mismatch | e_k(t), surprise_k(t) | per event |
| maintenance range | **none in the agent**; the body's physics only | world log |
| value of information-gathering action | **not computed** (actions are external in this stage) | — |
| risk and cost of action | **not computed** | — |
| variational free energy | **not computed** (no recognition density) | — |

## 5. Hard-coding risks specific to C, and their checks

* The encoding could smuggle structure (e.g. ordering symbols): symbols are one-hot and
  label-permutation must leave all results unchanged.
* The time-constant spread could be tuned to the world's intervals: τ is drawn log-uniform
  over a range fixed before the world's interval distribution was drawn; both are seeded
  independently and logged.
* The random action sampler could correlate with the world: its seed is independent and it
  never reads observations.
* The agent crate must not import the world crate (CI check) and must contain no channel
  names, no site indices, no action-specific branches.
