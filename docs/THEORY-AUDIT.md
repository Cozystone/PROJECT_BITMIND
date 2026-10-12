# Theory audit: what the sources say, what we take, what we do not, what we add

2026-10-12. Each paper was read from its full text (PDF extracted to text), with the
exceptions listed under "not read" in each section. Quotes are kept short. Nothing below is
taken from abstracts alone.

Reading status:

| source | read | not read / not verified |
|---|---|---|
| Friston 2010, *The free-energy principle: a unified brain theory?*, Nat Rev Neurosci 11:127 | main text pp. 1-10, Boxes 1-2, Figures 1-4 captions, references | **online Supplementary Information S1-S5** (the formal argument for "surprise bounds entropy", the three free-energy forms, the infomax and Bellman links, the mountain-car priors). The claims that rest on them are reported here as claims, not as checked derivations |
| Neftci, Mostafa, Zenke 2019, *Surrogate Gradient Learning in Spiking Neural Networks*, arXiv:1901.09948v2 | all 25 pages (text, equations 1-15, Boxes 1-3, Figures 1-7) | the cited primary papers (SuperSpike, e-prop, DCLL) were not re-read; their properties are reported as this review states them |
| Hasani, Lechner, Amini, Rus, Grosu 2020, *Liquid Time-constant Networks*, arXiv:2006.04439v4 (AAAI 2021) | main text pp. 1-8 (Eqs. 1-9, Theorems 1-5, Tables 1-6), Appendix S1-S3 (proofs of Theorems 1-3), S5-S9 (experimental setup, hyperparameters) | Appendix S4 (proof of Theorem 4) beyond its opening, and the proof of Theorem 5: skimmed only; the trajectory-length bounds are not verified here |

## 1. Friston 2010

**1. Claims and premises (as the paper states them).**
* Premise: biological systems "maintain their states and form", so "the probability of these
  ... sensory states must have low entropy"; entropy is average surprise, so agents "must
  therefore minimize the long-term average of surprise".
* An agent cannot evaluate surprise directly; free energy is "an upper bound on surprise"
  that "can be evaluated because it is a function of two things to which the agent has access:
  its sensory states and a recognition density".
* Free energy = energy − entropy = surprise + divergence = complexity − accuracy (Box 1; the
  derivation is in S2, not read).
* Perception: minimize F by changing the recognition density (reduces the divergence term).
  Action: minimize F by changing sensory input (can only act on the accuracy term): "the brain
  will reconfigure its sensory epithelia to sample inputs that are predicted".
* Learning and plasticity: "a gradient descent on free energy ... is formally identical to
  Hebbian plasticity" (Box 2: synaptic efficacy updates μ_θ = −∂_θ εᵀξ, i.e. presynaptic
  prediction × postsynaptic precision-weighted prediction error; the paper itself notes "the
  learning rule in predictive coding is really a delta rule").
* Precision = synaptic gain of error units (attention).
* Value and reward are recast as priors: "valuable states are just the states that the agent
  expects to frequent"; policies are "prior expectations about the motion of sensory states";
  exploration can be induced by priors that make non-attractive states unstable (mountain car).
* Scope caveat from the author: if the principle's "main contribution is to integrate
  established theories, then the answer is probably 'not a lot'"; the real challenge "is to
  understand how it manifests in the brain".

**2. What is actually implemented in the paper.** Only two simulations are shown, both from
the predictive-coding scheme of Box 2 (Gaussian/Laplace assumption, hierarchical generative
model with known equations of motion g, f):
* birdsong categorization (a Lorenz attractor generator, inferred causal states v1, v2);
* a cued reaching movement and the mountain car, where action a follows ȧ = −∂_a εᵀξ and the
  behaviour comes from hand-specified priors (an "invisible elastic band"; a negative-friction
  cost prior).

In both, the generative model's form is given by the modeller. Learning of parameters is
stated as Hebbian gradient descent but is not demonstrated in this review.

**3. What this project takes.**
* The separation of four quantities, kept as separate variables and logs: beliefs (internal
  state), predictions of the next observation, prediction error (surprise of what arrived),
  and the agent's own maintenance range.
* The local learning rule structure: synaptic change = presynaptic activity × postsynaptic
  prediction error (delta/Hebbian), with the error available locally where the prediction is
  made. This is the only plasticity used in the chosen candidate.
* The claim that action can reduce surprise only by sampling predicted inputs: used later (not
  in the first experiment) as one candidate action rule, with its known failure mode (seeking
  uninformative, predictable states) recorded as a risk.

**4. What this project does not take.**
* The phrase "variational free energy" for our objective. The paper's F requires a
  recognition density q and a generative model p with a divergence term. Our chosen candidate
  has a point prediction and a squared/log error, no posterior over causes; its objective is a
  prediction error, not a variational bound. We do not call it free energy.
* Physical/thermodynamic energy: F is "an information theoretic quantity ... as opposed to a
  thermodynamic quantity". The body's energy variable in our world is a resource budget, a
  different thing, and the two words are kept apart in code (`surprise` vs `body.energy`).
* "Expected free energy" (future-looking G with risk and ambiguity terms): not in this
  paper; it belongs to later active-inference work. Not used in the first experiment.
* The claim that the principle entails or explains survival, value, consciousness or
  cognition. It is treated as a principle about systems that already persist, not as a
  guarantee that a system built on it will persist.
* Priors-as-goals (the elastic band, the negative-friction cost). These are hand-written
  objectives in disguise; the user's rules forbid them as core learning targets.

**5. Untested hypotheses we add (ours, not the paper's).**
* H-FEP-1: a small recurrent network trained only by local next-observation prediction error
  accumulates experience-dependent changes that persist beyond its state dynamics and change
  later predictions (the "experience leaves traces" question). The paper gives the rule, not
  this result in a lifelong setting.
* H-FEP-2 (deferred): choosing actions by predicted surprise alone leads to predictable-state
  seeking in our world; a risk-free test is planned, not assumed.

## 2. Neftci, Mostafa, Zenke 2019

**1. Claims and premises.**
* Spiking networks of LIF neurons are formally RNNs: Eqs. 4-5 in discrete time,
  I[n+1] = αI[n] + Σ W S[n] + Σ V S[n]; U[n+1] = βU[n] + I[n] − S[n]; S = Θ(U − ϑ).
* Training them needs spatial and temporal credit assignment; the spike nonlinearity Θ has a
  derivative that is "zero everywhere except at U = ϑ, where it is ill defined".
* Surrogate gradients (SG) replace Θ′ by a smooth function (piecewise linear, fast sigmoid,
  exponential); "the success of the method is not crucially dependent on the details of the
  surrogate".
* The review is explicitly about **supervised** training: "a cost or loss function which is
  minimized when the network's response corresponds to the desired behavior".

**2. Actual training methods described.**
* BPTT on the unrolled network (most SG work; memory O(NT)).
* Forward methods (RTRL; Eq. 8; memory O(N³) naive, O(N²) in SuperSpike, O(N) in DCLL).
* SuperSpike: three-factor rule ΔW_ij ∝ e_i · ε ∗ (σ′(U_k) ∂U_k/∂W_ij) with eligibility traces
  simulated forward in time; the reset term is dropped "which empirically leads to better
  results"; spatial credit via random feedback (feedback alignment / direct feedback
  alignment), not weight transport.
* DCLL: local layer-wise losses with fixed random projections to a local classifier; scales
  O(N); feedforward only; refractory dynamics omitted.
* Box 3 defines locality: variables available at a synapse are {W_BA, S_A(t), U_A(t)}; a
  presynaptic spike T seconds ago and another neuron's membrane are non-local.
* Stated limitations: sigmoidal surrogates bring vanishing gradients; SGs introduce a bias;
  SuperSpike "does not scale favorably for large multilayer networks"; probabilistic models
  need injected noise that "often poses a challenge for optimization".

**3. What this project takes.**
* The locality criterion of Box 3 as the test for every update rule we write: the update of
  W_ij may use only W_ij, the presynaptic activity trace at j, the postsynaptic state at i,
  and one broadcast or locally computed modulating signal.
* Eligibility traces as the forward-in-time (no BPTT) mechanism for temporal credit, in the
  O(N²) form (one trace per synapse), which is affordable at our scale (N ≤ 128).
* Random fixed feedback (direct feedback alignment) as the default way to route the
  prediction error to hidden units, with weight-transport (Wᵀ) as an ablation.
* The warning that the choice of surrogate is not what makes or breaks learning, and that
  dropping the reset term is an empirical convenience: both recorded as design freedoms that
  must be ablated, not assumed.

**4. What this project does not take.**
* BPTT and adjoint methods (forbidden by the project rules, and the review's own locality
  argument).
* Supervised targets. There is no label and no target spike train. The only teaching signal
  is the agent's own next observation when it arrives.
* Spiking itself, for the first candidate. The review shows that spikes add a credit-assignment
  problem (Θ′) and a coding choice (rate vs timing) without, for our question, a demonstrated
  benefit. A spiking variant (candidate B in `ARCHITECTURE-OPTIONS.md`) is deferred.
* Any claim that STDP-like or homeostatic rules alone constitute learning of meaning. The
  review classifies "entirely biologically inspired local learning rules" as a separate
  category it does not evaluate.

**5. Untested hypotheses we add.**
* H-SG-1: a three-factor rule whose third factor is a self-generated next-observation error
  (not a supervised target) is sufficient for experience-dependent, lasting change in a small
  continuous-time network, under the locality criterion. The review covers supervised third
  factors only.
* H-SG-2: random feedback suffices for credit assignment in a 1-2 layer prediction network of
  our size (feedback alignment results in the review are for classification).

## 3. Hasani et al. 2020, Liquid Time-constant Networks

**1. Claims and premises.**
* Model: dx/dt = −[1/τ + f(x, I, t, θ)] x + f(x, I, t, θ) A (Eq. 1); f is a neural network;
  the effective time constant τ_sys = τ / (1 + τ f) is input- and state-dependent ("liquid").
* Theorem 1: τ_i/(1 + τ_i W_i) ≤ τ_sys,i ≤ τ_i. Theorem 2: states are bounded,
  min(0, A_min) ≤ x_i ≤ max(0, A_max). Theorem 3: universal approximation of finite
  trajectories of autonomous ODEs (proof follows Funahashi & Nakamura 1993; **no inputs** in
  the proof).
* Theorems 4-5: lower bounds on trajectory-length growth (expressivity). Not verified here.
* The ODE is stiff; an explicit/implicit fused Euler step (Eq. 3) is used; performance "is
  majorly influenced when off-the-shelf explicit Euler methods are used".
* Empirical: LTC outperforms LSTM/CT-RNN/Neural ODE on 4 of 7 supervised time-series tasks
  and on person-activity and half-cheetah fitting.
* Stated limitations: vanishing gradients ("they would not be the obvious choice for learning
  long-term dependencies in their current format"); elevated time and memory cost; solver
  sensitivity.

**2. Actual implementation and learning method.** Supervised, offline, gradient-based:
"trained by backpropagation through time" (Algorithm 2) with Adam, BPTT length 32,
minibatch 16, 200 epochs, 32 hidden units, sequences cut to 32 steps, best-validation
checkpoint selection (Appendix S5-S6). The liquid time constant is a property of the
**state dynamics**; the paper does not propose any online or local learning rule, and
nothing in it changes parameters during inference.

**3. What this project takes.**
* The continuous-time leaky-integrator formulation with a fused (semi-implicit) Euler step
  for stability at fixed step size; the state-boundedness argument as a safety property.
* The option of an input-dependent time constant as a **flag** on the state equation (default
  off), to be tested as an ablation of state dynamics, not as a learning mechanism.
* The distinction, now enforced in our logs, between state change (x(t) responding to input)
  and parameter change (W). LTC's expressivity results are about the former under
  BPTT-trained parameters; they say nothing about online learning.

**4. What this project does not take.**
* BPTT, Adam, epochs, minibatches, validation-checkpoint selection, train/test splits: the
  entire training protocol. Our agent has one life and no training set.
* The trajectory-length expressivity measure as evidence of learning. A long latent
  trajectory shows rich state dynamics, which the user's rules explicitly refuse to count as
  learning.
* The claim of superiority on benchmarks: irrelevant to our question and obtained with
  forbidden methods.
* The biological grounding ("loosely related" to non-spiking neuron models of small species):
  not used as justification for anything.

**5. Untested hypotheses we add.**
* H-LTC-1: with parameters learned only by a local three-factor rule (no BPTT), an
  input-dependent time constant gives no measurable advantage over a fixed-τ CT-RNN on the
  "experience leaves traces" tests. (Stated as a null expectation; the flag exists to test it.)
* H-LTC-2: the state dynamics' intrinsic memory horizon (autocorrelation decay of x) is short
  compared with the persistence we are looking for, so any lasting effect must be in W; this
  is what the state-reset test in `MEASUREMENT-PLAN.md` relies on.

## 4. Terms kept distinct in code and reports

| term | meaning here | not to be confused with |
|---|---|---|
| surprise | −log p̂(o(t)) of the arrived observation under the agent's own prediction | thermodynamic energy; body energy |
| prediction error | o(t) − ô(t) per channel (the signal that drives plasticity) | reward prediction error (there is no reward) |
| variational free energy | **not computed**; the candidate has no recognition density | our objective |
| expected free energy | **not computed** in the first experiment | action rules (deferred) |
| body energy | a resource variable of the simulated body, decremented by actions, replenished by rest | any information quantity |
