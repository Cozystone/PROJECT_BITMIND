# Observability: LifeTrace, the information barrier, replay and analysis tools

The instruments are part of the experiment, not an add-on. They must not change the learner.
Tracing on and tracing off must produce the same life, and world truth must never reach the
learner.

## 1. LifeTrace: data structure

Every record:

```
TraceRecord {
  id: u64,              // monotone over the whole life
  tick: u64,            // world time
  layer: Layer,         // see table
  parents: [u64; ≤4],   // causal links: the records this one was derived from
  payload: LayerPayload // fixed-schema fields per layer
}
```

| layer | payload | written by |
|---|---|---|
| `WorldEvent` | event type, the changed entity, actual result, the cause event id | world (truth side) |
| `RawObservation` | channel, value, source, dropped or noise flag. **The noise flag lives in the truth-side copy only**; the agent-side copy has no flag | world → interface |
| `Belief` | the variable or table touched; before and after (compact: changed entries only); entropy before and after; evidence mass | agent |
| `Body` | the energy and damage readings received; deltas | agent (the reading) and world (the truth) separately |
| `Candidate` | per candidate action: predicted outcome distribution, risk, ambiguity, information gain, cost, G, uncertainty | agent |
| `Choice` | the chosen action, its probability, γ, the RNG draw, the ids of the top unchosen candidates | agent |
| `Outcome` | the observations after the action; surprise per channel; realised cost, damage and resources | agent (surprise) and world (truth) |
| `Learning` | counts updated, change-detector statistic, discount events, context merges, posterior revisions, with their cause records | agent |
| `Recovery` | analyzer-derived: failure detected (by definition), class, linked later revisions and later decisions in the same class | analyzer only (never the agent) |

## 2. Storage and retention

* **Agent-side and truth-side traces are written to separate files**, so the learner never reads
  either. Format: length-prefixed binary records in 64 MB chunks, compressed per chunk, with an
  index per chunk (tick range, record id range).
* **Resident memory:** a bounded write buffer only (at most 8 MB). The agent keeps no trace
  history.
* **Retention:**
  * full detail for the whole life, by default, for the first gate's lives;
  * for long lives, full detail for the latest 2 x 10^5 ticks plus ±10,000 ticks around every
    analyzer-flagged event (viability exit, confident error, regime shift), and a 1% uniform
    random sample of decisions;
  * per-1,000-tick aggregates for everything.

  The retention policy is written into each run's manifest.
* **Cost estimate:**
  * about 0.4-1 KB per decision before compression; about 10^5 decisions per 10^6-tick life;
    about 50-100 MB per life uncompressed, 15-30 MB compressed;
  * 15 lives x (agent + 7 baselines and ablations), about 3-4 GB on disk;
  * write overhead target under 15% of run time (measured by the profiler).

## 3. Information barrier between the learner and the analyzer

* **Compile-time.**
  * `life-agent` depends only on `life-interface`.
  * `life-interface` contains only observation, action and result types, with no truth fields.
  * A CI check runs `cargo tree` and fails if `life-agent` reaches `life-world` or `life-lab`.
* **Runtime.**
  * The world sends the agent only `Observation` values. Truth goes to the truth trace and the
    analyzer.
  * The trace writer is a sink with no read API inside the agent process boundary.
* **Observer invariance test.** The same seed run with tracing fully on and fully off must
  give the identical sequence of action hashes and body states. This is a CI test.
* **Oracle hygiene.** Analyzer-derived labels (failure classes, recovery) are computed offline.
  They are never fed to policies, memory updates or baselines, except the fixed-rule baseline,
  whose rules are written from the world specification and documented as such.

## 4. Tools (minimum specification)

| tool | input → output | guarantees |
|---|---|---|
| `replay` | a run, a tick → the agent's and the world's state at that tick, reconstructed from the checkpoint before it plus deterministic re-execution | read-only; replays never write into the main life's files |
| `provenance` | a `Choice` record id → the tree back through candidates, the belief entries they used, the observations and world events behind those entries | every link is a stored parent id; no inference |
| `seen-vs-true` | a tick range → side by side: what the agent received, its beliefs, and the world's truth | the only tool that joins the two sides; analyzer-only |
| `diff` | two ticks, or "before and after failure X" → changed beliefs, uncertainty, candidate scores for the same context key | the context keys are the agent's own |
| `timeline` | a run → a compressed timeline: regime shifts, viability exits, confident errors, revisions, with counts between them | |
| `profile` | a run → resident memory, decision time, trace bytes per window | |

Checkpoints (full agent and world state plus RNG states, every 50,000 ticks) are used **only**
for replay and analysis. A branch from a checkpoint is never reported as the main life, and no
favourable branch is selected.

## 5. State narrator ("diary"): rules; not built in the first implementation

A narrator, if added later, renders template sentences **only from trace fields**, for example:

> "t=412,300-412,340: the observations (ids 88121, 88130) had surprise 6.2 bits under the outcome model for (shake, colour 2, wet); the posterior on bush 7's kind moved from sweet 0.81 to nettle 0.64 (id 88131); the next decision chose probe over shake (G components, id 88140)."

* Every sentence carries its trace ids, its tick range and the state deltas it summarises.
* Mental or affective words, first-person voice and any sentence without trace ids are
  forbidden (list in `SAFETY-AND-SCOPE.md`).
* The narrator is a reading aid for humans. It is not evidence of experience or awareness.

## 6. Dashboard (later)

Text, CSV and JSON first. The minimum later dashboard shows:
* the series of `MEASUREMENT-PLAN.md` section 3 on the continuous time axis;
* links from failure to revision to recurrence;
* counts of memory creation, revision, discount and merge;
* the baselines side by side;
* resource use.
