# SPL-ARCH V1

**Status:** THEORETICAL V1 — FROZEN  
**Implementation status:** not implemented  
**Epistemic status:** architectural proposal; universal originality not claimed

## 1. Purpose

SPL-ARCH V1 is a proposed trust architecture for adaptive realization of a computation.

Its central question is not whether a compiler can optimize, retarget, JIT-compile, schedule heterogeneous devices, synthesize hardware, or verify code. All of those areas already have substantial prior art.

The V1 question is narrower:

> Can producers of realizations and adaptive planners remain outside the semantic trusted computing base while every realization that actually executes is forced through a trusted admission boundary derived from one persistent semantic authority, with integrity between what was proved, what was admitted, and what executes?

SPL-ARCH V1 therefore describes an **architecture / trust protocol for adaptive heterogeneous realization**, not yet a programming language, IR, compiler, or runtime implementation.

## 2. Non-claims

V1 does **not** claim to have invented:

- proof-carrying code or proof-carrying hardware;
- adaptive compilation, JIT compilation, autotuning, PGO, or runtime scheduling;
- heterogeneous CPU/GPU/FPGA execution;
- runtime FPGA generation or hardware/software co-verification;
- semantic-preserving IRs, persistent compiler abstractions, formal rewrite validation, or ISA retargeting;
- a Smart Programming Language;
- practical performance, safety, or universal originality.

The candidate contribution is the composition and division of authority described below.

## 3. Fundamental objects

The frozen V1 model contains eight fundamental objects.

### 3.1 `K` — Semantic Contract

`K` defines what behavior counts as correct.

For SPL-0:

```text
SemanticOutcome<Y,E> =
    Return(Y)
  | SemanticError(E)
```

and:

```text
K : X × Env -> SemanticOutcome<Y,E>
```

`K` is persistent for one semantic definition of the computation. It does not define which checker, solver, backend, target, or performance strategy must be used.

### 3.2 `A` — Assurance / Admission Policy

`A` defines what evidence mechanisms the current environment accepts as sufficient justification for admission.

Conceptually:

```text
AssurancePolicy {
    accepted_mechanisms
    accepted_checkers
    minimum_assurance
    trusted_assumptions
    checker_versions
}
```

`K` answers **what is correct**. `A` answers **what justification is accepted as evidence of that correctness**.

In SPL-0 V1, `A` is frozen at the experimental freeze boundary. Dynamic changes to `A` are out of scope.

### 3.3 `S` — Physical Substrate

`S` represents the physical resources available during a `SubstrateEpoch`, for example CPU silicon, RAM, GPU silicon, FPGA fabric, and physical interconnect.

For the SPL-0 model, `S_epoch` is treated as constant during an invocation.

### 3.4 `Q` — Architectural Configuration

`Q` is the architectural configuration state realized over `S`.

Examples can include an FPGA bitstream, soft-core configuration, reconfigurable accelerator configuration, or another configuration that changes the computational primitives available over the substrate.

Define:

```text
ExecInterface(S,Q)
```

as the effective computational interface exposed by `S` under configuration `Q`.

### 3.5 `P` — Execution Plan

`P` is an executable realization over a particular `S/Q` environment. It may encode computation strategy, instruction organization, placement, data layout, transfer strategy, synchronization, specialization, or schedule.

### 3.6 `G_v` — Validity Guard

`G_v` restricts the invocation domain in which a candidate realization is semantically valid.

A validity guard is not a performance hint. A condition may influence selection without granting validity.

Frozen rule:

> Preference cannot authorize validity.

For SPL-0, validity guards must be stable for the complete invocation.

### 3.7 `VC` — Verification Condition

The untrusted producer does not choose what proposition must be proved.

The trusted side derives the obligation:

```text
VC = DeriveObligation(
    K,
    G_v,
    S_epoch,
    Q_bound,
    P_bound
)
```

`VC` must bind the exact semantic contract, guard, substrate epoch, architectural configuration, and execution realization to which the evidence applies.

### 3.8 `V` — Validation Evidence

`V` is evidence offered in satisfaction of `VC`.

Its mechanism can vary, for example a machine-checked proof, translation-validation result, verified derivation, runtime-checked invariant, proof-carrying artifact, or explicitly trusted attestation.

Different mechanisms may provide different levels of assurance. V1 does not pretend they are epistemically equivalent.

## 4. SPL-0 observation model

The final freeze patch removes the previously underspecified `Obs` projection.

The machine semantics use one explicit common outcome domain:

```text
ObservedOutcome<Y,E,T> =
    Return(Y)
  | SemanticError(E)
  | Trap(T)
  | Stuck
  | Diverge
```

The semantic contract embeds into that domain:

```text
embed : SemanticOutcome<Y,E> -> ObservedOutcome<Y,E,T>

embed(Return(y))          = Return(y)
embed(SemanticError(e))   = SemanticError(e)
```

`Trap`, `Stuck`, and `Diverge` are not in the image of `embed` for SPL-0.

Execution is modeled directly as:

```text
Run(P,Q,S,x,env) : ObservedOutcome<Y,E,T>
```

`Run` is a semantic function. `Diverge` represents semantic divergence; this notation does not require an actual interpreter routine to return after an infinite computation.

## 5. Initial validity relation

For SPL-0:

```text
Valid(P,K,Q,S) iff
    Realizable(Q,S)
    AND
    forall x,env in Dom(K):
        Run(P,Q,S,x,env) = embed(K(x,env))
```

Therefore, if `K(x,env) = Return(y)`, then `Diverge`, `Stuck`, `Trap(t)`, and `Return(y')` for `y' != y` are invalid outcomes.

Likewise, a semantic error is not interchangeable with an arbitrary machine trap:

```text
Trap(t) != SemanticError(e)
```

General effect traces, real-time semantics, concurrency, nondeterminism, and weak memory are outside the SPL-0 formal core.

Any state relevant to the semantic result of an invocation must be represented through `X`, `Env`, `K`, or another explicitly semantic representation rather than hidden mutable state.

## 6. Candidate and obligation binding

Conceptually a candidate is:

```text
Π = (K_ref, G_v, Q, P, V)
```

`K_ref` is a stable identity for the exact represented contract. V1 does not require semantic equivalence of two independently encoded contracts to imply identical references.

The important binding is not a pathname, symbolic name, or mutable handle. `P_bound` and `Q_bound` must denote the exact realization/configuration admitted, through immutability or an authenticated identity of immutable content.

## 7. Obligation soundness

The first central trusted responsibility is **Obligation Soundness**.

For a guarded candidate, acceptance of evidence for the derived obligation must justify the target semantic relation:

```text
Check_A(
    V,
    DeriveObligation(K,G_v,S,Q,P)
) = ACCEPT

=>

forall x,env:
    G_v(x,env)
    ->
    Run(P,Q,S,x,env) = embed(K(x,env))
```

under the assumptions explicitly belonging to the chosen assurance mechanism and TCB.

A derivation bug that omits a necessary condition is a TCB failure, not a permission for the backend to redefine correctness.

## 8. Evidence checking soundness

The second central trusted responsibility is **Evidence Checking Soundness**.

If:

```text
Check_A(V,VC) = ACCEPT
```

then the checker must establish that `V` satisfies `VC` under policy `A` and the declared trusted assumptions.

The producer may produce the evidence. The producer does not control the trusted statement that must be established.

## 9. Admission

A realization may be admitted only when the required semantic and realization conditions hold, including at minimum:

```text
ContractMatches
ValidityGuardHolds
Compatible(S,Q)
Check_A(V,VC) = ACCEPT
```

A preferred architecture can compute:

```text
EligibleSet_t = { Π | Admit_A(Π,state_t) }
```

and present only admitted realizations to the planner.

However, V1 does not make the ordering `Admission -> Selection` a fundamental correctness claim. A system could select first and admit before execution. The fundamental property is mandatory admission before activation/execution.

## 10. Complete mediation

No legitimate execution path may bypass semantic admission.

Frozen invariant:

```text
Execute_t(Π) -> Admitted_t(Π)
```

Debug paths, fallbacks, warm starts, cached handles, direct backend calls, recovery paths, and other implementation routes must still satisfy the same invariant.

A path that executes without it is a protocol violation.

## 11. Activation Gate

Selection does not imply permission to execute.

An `ActivationGate` must mediate execution and verify the still-relevant admission and realization bindings before invocation.

At minimum it must enforce the active contract relationship, bound realization identity, bound architectural configuration, substrate epoch, and invocation guard/context.

## 12. Admission-to-Execution Integrity

The third central trusted responsibility is **Activation Integrity**, expressed by the stronger V1 invariant of Admission-to-Execution Integrity:

> What was proved, what was admitted, what was activated, and what actually executes must be the same semantically relevant realization.

For invocation `i`:

```text
ActualP(i) = P_bound
ActualQ(i) = Q_bound
ActualS(i) = S_epoch
```

and the guard must have been evaluated for the same semantically relevant invocation context that is actually executed.

Conceptually:

```text
GuardContext(i) = ExecutionContext(i)
```

for the facts on which `G_v` depends.

This closes check-to-use attacks such as proving `P_good` and executing `P_bad`, proving one bitstream and configuring another, or validating `x_good` and substituting `x_bad` before execution.

## 13. Invocation boundaries

SPL-0 permits adaptive switching only between semantically complete invocations:

```text
invoke K(x1) using Π_A
complete

switch

invoke K(x2) using Π_B
complete
```

Mid-invocation migration is not part of V1.

A future model may require transition evidence such as `W_ij`, but such a mechanism is explicitly post-V1.

## 14. Adaptive selection

After semantic permission has been established, runtime evidence `E_t` may influence preference among usable realizations.

Examples include latency, throughput, input size, data location, transfer cost, warmup, energy estimates, and historical performance.

The planner may be wrong about performance. That is acceptable.

The planner may not create semantic permission.

## 15. Semantic TCB

The candidate Semantic Trusted Computing Base contains the responsibilities necessary to maintain the protocol, conceptually including:

- semantic-contract interpretation;
- verification-condition derivation;
- evidence checking and enforcement of `A`;
- validity-guard evaluation;
- activation gating;
- realization identity/binding;
- trusted modeling of `S`, `Q`, and `ExecInterface` where relevant.

The exact implementation boundary may cross processes or binaries. TCB here is an authority concept, not a requirement for one executable.

The V1 goal is:

```text
Backend ∉ SemanticTCB
Planner ∉ SemanticTCB
```

A backend may generate incorrect code or evidence; a planner may choose badly or maliciously. Neither should be able to make an inadmissible realization execute.

## 16. Execution adaptation versus architectural adaptation

V1 distinguishes two forms of change.

If:

```text
ExecInterface(S,Q_t) = ExecInterface(S,Q_t+1)
```

while the execution realization changes, the change is **execution adaptation**.

If:

```text
ExecInterface(S,Q_t) != ExecInterface(S,Q_t+1)
```

then V1 calls it **architectural adaptation**.

Using an instruction that was already available does not itself count as architectural adaptation. Materializing or reconfiguring a capability so that the architectural interface changes can.

The stronger architectural hypothesis is post-core: the same semantic authority may eventually govern realizations across different `ExecInterface` configurations.

## 17. Frozen chain

The consolidated V1 trust chain is:

```text
Semantic Contract K
        +
Assurance Policy A
        ↓
Candidate proposed
        ↓
VC = DeriveObligation(
    K,
    G_v,
    S_epoch,
    Q_bound,
    P_bound
)
        ↓
Validation Evidence V
        ↓
Check_A(V,VC)
        ↓
Admission
        ↓
Adaptive decision
        ↓
ActivationGate
        ↓
Admission-to-Execution Integrity
        ↓
Execute exact admitted realization
```

## 18. Candidate demarcation

The surviving candidate demarcation is deliberately narrow:

> SPL-ARCH proposes a trust protocol for adaptive realization in which no realization may be activated or executed without satisfying an obligation derived from a persistent semantic authority, while the system preserves identity between the object proved, admitted, activated, and actually executed, allowing realization producers and adaptive planners to remain outside the semantic TCB.

This is a **candidate architectural composition**, not a claim of universal novelty.

## 19. Prior-art status

The adversarial investigation identified strong antecedents for the individual mechanisms, including MLIR/LLVM, HPVM and HPVM2FPGA, ApproxHPVM/ApproxTuner, PetaBricks, Halide, TVM, Delite, Truffle/Graal, Exo/Exo2, persistent e-graphs, Hydride/MISAAL, CompCert/Alive2, CIRCT/Calyx, Proof-Carrying Code, Proof-Carrying Hardware, Proof-Carrying Services, and the SFB 901 research program.

The strongest threat found was SFB 901, especially the combination of:

- B4: proof-carrying software/hardware certification and co-verification;
- B2: feedback-driven configuration;
- C2: runtime CPU/FPGA alternatives, heterogeneous scheduling, and migration;
- the integrated Proof-of-Concept.

In the primary material examined during the adversarial process, an explicit invariant equivalent to mandatory semantic admission with complete mediation over adaptive heterogeneous execution was **not established**.

This supports only:

```text
PRIOR-ART SUBSUMPTION: NOT ESTABLISHED
DEMARCATION: PROVISIONALLY SURVIVES
```

It does not prove that no equivalent system has ever existed.

See [PRIOR-ART.md](./PRIOR-ART.md) for the frozen research-status note.

## 20. Falsification conditions

The V1 hypothesis should be weakened or rejected if experiments or later research establish, among other things, that:

- an earlier system substantially subsumes the central architecture;
- future backends must be trusted for semantic correctness;
- the planner must possess semantic authority;
- `K` is insufficient to derive obligations for future backends without returning to hidden frontend/source information;
- validation must be semantically redesigned for each backend;
- complete mediation or admission-to-execution integrity cannot be enforced;
- the system cannot distinguish valid from invalid realizations produced by an untrusted producer;
- the cost of maintaining the persistent authority eliminates the architectural value;
- architectural adaptation necessarily requires redefining `K` in a way that destroys the proposed continuity.

## 21. Evidence that would strengthen the hypothesis

Evidence in favor includes:

- a future untrusted backend produces a new correct realization that is admitted;
- the same backend produces an incorrect realization that is rejected;
- backend and planner remain outside the Semantic TCB;
- obligation generation works without source/frontend recovery;
- multiple admissible realizations coexist;
- runtime evidence changes selection without granting validity;
- post-admission mutation is detected;
- a hostile planner cannot bypass activation;
- later, the same `K` governs realizations across distinct `ExecInterface` configurations.

## 22. Explicit V1 scope limits

Out of scope for the frozen V1 formal core:

- general concurrency and weak memory;
- nondeterministic contracts;
- general effect traces;
- real-time contract semantics;
- mid-invocation migration and transition proofs;
- dynamic mutation of `A`;
- dynamic mutation of the semantic contract;
- universal program-equivalence checking;
- universal automated proof generation;
- full FPGA synthesis;
- a general-purpose SPL parser or surface language;
- AI-based semantic correctness decisions.

AI may later propose candidates, architectures, or cost estimates, but the V1 trust model does not grant AI semantic authority merely because it is AI.

## 23. Experimental transition

The theoretical formalization loop is frozen at V1.

The next phase is empirical:

1. build the SPL-0 Admission Kernel experiment;
2. test a future untrusted backend against a frozen semantic/admission boundary;
3. introduce adaptive choice among multiple admitted realizations;
4. only after those succeed, investigate simulated and then physical architectural adaptation.

The experimental contract and adversarial test plan are specified in [EXPERIMENTS-V1.md](./EXPERIMENTS-V1.md).

---

## Frozen V1 statement

> **SPL-ARCH V1 proposes and investigates a trust protocol for adaptive realization in which producers and planners may remain outside the semantic trusted computing base, while no realization may execute without satisfying an obligation derived against a persistent semantic authority and while the system maintains identity between the realization proved, admitted, activated, and actually executed.**
