# Architecture options

Three designs compared before any implementation. The criteria come from the research
question.

## 1. The options

**A. HDC/VSA reuse.**
* Situations are encoded as bound hypervectors; an item memory stores
  (situation ⊗ action → outcome) associations.
* Predictions come from nearest-neighbour similarity, and evidence provenance ledgers are reused
  from the archived research.
* Uncertainty would be read from similarity margins and neighbour counts.

**B. Explicit, small probabilistic state-space model.**
* A factored generative model with discrete variables:
  * latent per-object attributes (for example a bush's kind);
  * latent world state (weather);
  * body state from interoception.
* Beliefs are categorical posteriors and Dirichlet counts in fixed point.
* Transition and outcome models are count tables keyed by (action, observed context features).
* Actions are chosen by expected free energy over a short horizon (section 4).

**C. Hybrid.**
* B is the decision and belief core.
* HDC encodes contexts, so counts can be pooled across similar contexts, and serves as a compact
  episodic memory.

## 2. Comparison

| criterion | A: HDC reuse | **B: explicit state space** | C: hybrid |
|---|---|---|---|
| memory as life and memory grow | grows with stored items; needs eviction; similarity cost per query | bounded: one table per (action, context key), with fixed outcome alphabets; contexts capped and compacted with logged merges | B's cost plus an item memory |
| representing observation error and uncertainty | indirect: similarity is not probability; noise and confidence are hard to separate | direct: sensor-noise parameters are learned as counts; posteriors carry uncertainty; surprise is -log2 p(o) | as B for decisions |
| path from failure to belief revision | update or append items; no explicit "this prediction failed by this much" | explicit: a prediction error (surprise) updates the specific counts; change detection on surprise statistics triggers faster forgetting of the affected table | as B |
| risk of becoming a hard-coded goal tracker | moderate: needs hand-designed retrieval and decision rules | lower and **checkable**: the only goal-like element is a small, explicit preference prior over interoceptive observations; action selection is one formula, applied to every action | as B, plus similarity heuristics to audit |
| diagnosability | low to moderate: a hypervector is opaque without decoding | high: every belief is a readable table entry; every choice is a decomposition (risk, ambiguity, information gain, cost) | moderate |
| laptop-class feasibility | yes (popcount), but memory grows | yes: tables of KBs to MBs; planning over 6 actions with horizon ≤ 3 | yes, but heavier |
| forced reuse of old BITMIND assets | high (the temptation the instructions warn about) | none | some, but justified only after B shows a generalisation gap |

## 3. Choice: B. C is a pre-specified later ablation, not a plan

**Choice.** B is chosen because the research question is about **change over a lifetime**.
Detecting that change requires:
* beliefs whose uncertainty and revision can be read and audited;
* one action-evaluation rule whose components can be ablated.

B gives this with the least machinery and no inherited assumptions.

**Why A was rejected.** Its uncertainty is not probabilistic, so calibration (confidence vs
accuracy) cannot be audited directly. It also invites reuse of the archived engine "because it
exists", and its revision path after failure is implicit.

**Why C was not chosen now.** It adds a second representation before there is evidence that B
fails to generalise. Its trigger is pre-specified: C is considered only if B shows a measured
generalisation gap, such as slow learning in contexts that differ only in irrelevant features.
It would then be introduced as an ablation-tested module, with HDC justified by that
measurement.

## 4. How the free-energy principle becomes computation in B (operational, traceable)

| term | what it is in code | logged as |
|---|---|---|
| beliefs q | categorical posteriors over latent attributes (each bush's kind; the weather) and Dirichlet counts of outcome models p(outcome \| action, context) and sensor models p(reading \| true value) | belief snapshot per decision, and diffs per update |
| uncertainty | entropy of each posterior; predictive entropy; total Dirichlet concentration (evidence mass) | per belief, per prediction |
| prediction | the predictive distribution of the next observation for the chosen action (outcome model × sensor model under q) | per action, before its outcome |
| prediction error (surprise) | -log2 p(observed \| prediction), in fixed point (Q16 bits) | per observation; an exponential moving summary per channel |
| variational free energy F | complexity - accuracy for the update caused by an observation: KL(q_after ‖ q_before) - log p(o \| q_before), computed exactly over the small discrete variables touched | per update |
| preferences C | a fixed, explicit log-preference over interoceptive buckets: energy above a band, damage below a band. **The only goal-like element** | written once at birth; never changed by code |
| expected free energy G(a) | for each candidate action a (horizon h ≤ 3 through the learned models): risk + ambiguity - information gain + cost (see below) | the full decomposition for every candidate, every decision |
| action selection | softmax over -G with a fixed precision γ, sampled with the agent's seeded RNG (deterministic replay) | chosen action, the probabilities, the unchosen top candidates |

The terms of G(a):
* **risk** = KL(predicted interoceptive outcome ‖ C);
* **ambiguity** = expected sensor entropy;
* **information gain** = expected KL change of the Dirichlet parameters and latent posteriors;
* **cost** = predicted energy and time.

**Irreversibility.** For actions with a learned non-zero probability of damage, risk is
evaluated at an upper quantile of predicted damage under parameter uncertainty (a credible
bound computed from the Dirichlet counts), not at its mean. When the model is uncertain,
possible harm therefore counts more. This follows from the posterior, not from a hand-written
"avoid" rule.

**Revision after failure.**
* Counts are updated by every observation.
* A per-table change detector compares recent surprise with its long-run level (a CUSUM
  statistic over -log2 p). When it fires, that table's old counts are discounted by a fixed
  factor (bounded memory of the past regime), and the event is logged as a revision.
* There is no other forgetting.

**Words not used.** "Curiosity" (information gain is a term of G), "fear" (risk is a term of G),
"self-model" (the body model is a set of tables). See `SAFETY-AND-SCOPE.md`.

## 5. Bounded memory

* Context keys are tuples of observed features: (action, colour, wetness reading, day/night,
  terrain, pitch if probed). At most a few thousand exist.
* Outcome alphabets are fixed and small.
* Per-object posteriors: 12 bushes, small.

If the context count exceeds a cap, the least-used contexts are merged into their marginal
(feature-dropped) table. Every merge is logged.

Expected resident memory: under 50 MB. LifeTrace lives on disk (`OBSERVABILITY.md`).

## 6. Code layout (for the implementation phase)

| crate | role | may depend on |
|---|---|---|
| `life-interface` | `Observation`, `Action`, `ActionResult` types only | nothing |
| `life-world` | world state, rules, scheduler, truth log | interface |
| `life-agent` | beliefs, models, expected free energy, choice | interface only, **never** world or lab |
| `life-trace` | append-only trace writer (a one-way sink) | interface |
| `life-lab` | analyzer, replay, provenance, diff, profiler, metrics | everything, read-only |
| `life-run` | wires one life, baselines, ablations | all |
