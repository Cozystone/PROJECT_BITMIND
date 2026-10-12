# Safety and scope

## 1. In scope (first stage)

* One small simulated world (`CONTINUOUS-WORLD-SPEC.md`).
* One agent with explicit beliefs, uncertainty, prediction error, an explicit preference prior
  and expected-free-energy action selection (`ARCHITECTURE-OPTIONS.md`).
* Instruments and analysis (`OBSERVABILITY.md`, `MEASUREMENT-PLAN.md`).

## 2. Out of scope (not in the initial environment, not added without a new design review)

* Language, natural-language interfaces, LLM/SLM/Transformer models, pretrained embeddings,
  external model APIs, used as runtime components, teachers, planners or oracles.
* OS control, file creation, deletion or modification outside the run's own output directory,
  web actions, network access, real mouse or keyboard.
* Any human or user data.
* Real sensors.

## 3. Engineering limits

* At most one memory-heavy process at a time. A per-process cap of 16 GB, enforced by a
  watchdog that logs and terminates; a resource failure is reported separately.
* No black-box learned component hides the core. Probabilities and counts are explicit and
  readable.

## 4. Claims discipline

**The only claim this stage may make, if the gate is passed:**

> In a limited synthetic world, a persistent agent with explicit beliefs and uncertainty recovered faster from failures and/or re-adapted faster after unannounced change, as its own experience accumulated, than the stated baselines and ablations.

**Expressions never used in reports, docs or dashboards** (Claude's banned list):
* "the agent felt / feared / wanted / was curious / was anxious / enjoyed / suffered";
* "the agent realised / understood / had an insight / thought / decided to" (as mental
  claims). "Selected action X with G = …" is used instead;
* "self-aware", "conscious", "sentient", "alive" (as a property claim), "has a self",
  "inner life", "experience" (as phenomenal experience);
* "learned like a child / like an animal / like a human";
* "general intelligence", "AGI", "autonomous agent" (as a capability claim), "real-world ready",
  "Jarvis-like";
* "grew" or "developed" for any trend not passing the falsification plan;
* "proves" for any synthetic-world result;
* any narrative sentence without trace ids.

Internal variables are named functionally: energy, damage, prediction error (surprise),
uncertainty (entropy), information gain, risk, cost, preference prior.

## 4a. Maintenance conditions are not instincts (added 2026-10-12)

The simulated body's resource, the interoceptive signal, the agent's activity regulation and
any human-added cost are four different things (`CONTINUOUS-WORLD-SPEC.md` section 4). None of
them is reported as a "survival instinct", "drive" or "will to live". When the world resets an
incapacitated body, the record says `external_rescue`; it is never counted as self-recovery.

Reacting to input, or complex internal oscillation, is reported as state dynamics, not as
learning, memory or thought. A change is called a memory trace only when it passes the
state-reset and plasticity-off controls of `MEASUREMENT-PLAN.md`.

## 5. Claude's roles in this project

1. **Systems engineer:** builds the world, agent, instruments and runners to the specification.
2. **Experiment designer:** fixes what is measured, and how a result would be judged, before
   running.
3. **Falsification assistant:** for every interesting behaviour, looks first for simpler
   explanations (bugs, chance, hard-coding, leakage, selection). Reports it in the pattern
   format of `MEASUREMENT-PLAN.md` section 7 with status confirmed / promising / open, and
   never packages it as a success.

## 6. Post-hoc changes

When results are disappointing, thresholds, world rules and measurement windows are not changed
afterwards. A changed design is a new pre-registered experiment, and the old result stays
reported.
