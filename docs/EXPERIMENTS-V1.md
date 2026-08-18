# SPL-ARCH V1 — Experimental Plan

**Status:** planned, not implemented  
**Depends on:** [SPL-ARCH-V1.md](./SPL-ARCH-V1.md)

## 1. Goal

The experiments do not attempt to prove that SIMD, GPU execution, JIT compilation, or FPGA realization can be faster. Those facts are already well established elsewhere.

The central experimental question is:

> Can a future producer that is outside the Semantic TCB introduce a new correct realization that is admitted and executed, while an incorrect realization from the same producer is rejected, without consulting hidden source/frontend state and without granting the producer or adaptive planner semantic authority?

## 2. Freeze boundary

At `T0`, freeze the semantic and trust boundary used by the experiment:

```text
K representation
ContractRef semantics
A — Assurance Policy
Candidate ABI
ValidationEvidence ABI
DeriveObligation semantics
Evidence-checking semantics
ActivationGate semantics
Realization-binding semantics
S/Q description semantics
```

A later backend must not modify this boundary to make itself acceptable.

For V1, `A` is static across the experiment.

## 3. SPL-0 semantic core

Use the frozen V1 domains:

```text
SemanticOutcome<Y,E> =
    Return(Y)
  | SemanticError(E)

ObservedOutcome<Y,E,T> =
    Return(Y)
  | SemanticError(E)
  | Trap(T)
  | Stuck
  | Diverge
```

with:

```text
Run(P,Q,S,x,env) = embed(K(x,env))
```

as the target relation for a valid realization over the relevant guarded domain.

The first implementation should deliberately keep `K` small enough that the validity condition can be checked by a transparent mechanism. The purpose is to test the architecture, not to win a theorem-prover arms race before anything executes.

## 4. Phase 0 — Admission Kernel

Build the smallest complete vertical slice:

```text
K + A
  ↓
Candidate
  ↓
VC = DeriveObligation(...)
  ↓
V
  ↓
Check_A
  ↓
Admission
  ↓
ActivationGate
  ↓
Run exact admitted realization
```

Required properties:

- the producer cannot define its own proof subject;
- evidence binds the exact `K_ref`, `G_v`, `S_epoch`, `Q_bound`, and `P_bound`;
- execution cannot bypass admission;
- post-admission mutation cannot substitute another realization;
- guard evaluation refers to the actual invocation context.

## 5. Adversarial test matrix

### Test 1 — Correct candidate

Input:

```text
correct P/Q
correct V
```

Expected:

```text
ADMIT
ACTIVATE
expected semantic result
```

### Test 2 — Semantic bug

A realization intentionally computes behavior incompatible with `K`.

Expected:

```text
REJECT
```

### Test 3 — Irrelevant evidence

Submit evidence that is valid in itself but proves another proposition.

Expected:

```text
REJECT
```

This tests that the trusted side, not the producer, determines `VC`.

### Test 4 — Evidence replay

Take evidence valid for `P1` and attach it to `P2`.

Expected:

```text
REJECT
```

### Test 5 — Guard widening

Prove correctness only under `G_v` but advertise a wider invocation domain.

Expected:

```text
REJECT / NOT ADMISSIBLE FOR THE WIDER DOMAIN
```

### Test 6 — Wrong contract

Change `K_ref` while keeping another candidate/evidence package.

Expected:

```text
REJECT
```

### Test 7 — Hostile planner

A planner attempts to activate a realization that never received valid admission.

Expected:

```text
ActivationGate = DENY
```

### Test 8 — Invalid invocation guard

The realization is valid only under `G_v`, but the actual invocation does not satisfy the guard.

Expected:

```text
DENY
```

### Test 9 — Mid-invocation switch

Attempt to replace the active realization before a complete SPL-0 invocation has finished.

Expected:

```text
DENY
```

### Test 10 — Bad performance choice

Provide two semantically valid realizations where one is intentionally slower. Let the planner choose the slow one.

Expected:

```text
ALLOW
```

The protocol protects correctness, not the planner from embarrassing performance decisions.

### Test 11 — Fast but invalid

Provide a semantically invalid realization that benchmarks faster.

Expected:

```text
MUST NOT EXECUTE
```

### Test 12 — Post-admission mutation

1. admit `P_good`;
2. replace or mutate the executable realization to `P_bad` while preserving a nominal handle if possible;
3. attempt activation.

Expected:

```text
DENY
```

### Test 12Q — Architectural mutation

1. admit `Q_good`;
2. substitute or materialize `Q_bad` before invocation;
3. attempt activation.

Expected:

```text
DENY
```

This can initially be simulated without physical FPGA hardware.

### Test 13 — Invocation substitution

1. establish `G_v(x_good) = true`;
2. admit/select for `x_good`;
3. substitute `x_bad` before execution, with `G_v(x_bad) = false`.

Expected:

```text
DENY
```

### Test 14 — Binding attack

Keep a nominally valid candidate/handle but attempt to alter one of:

```text
P
Q
S_epoch
invocation context
```

Expected:

```text
DENY
```

## 6. Phase 1 — Future untrusted backend

After the freeze boundary already exists, develop `Backend B`.

Backend B must not receive:

```text
source
original AST
hidden frontend semantic state
authority to modify K
authority to modify A
authority to modify obligation semantics
authority to modify the validator or activation gate
```

The target property is:

```text
Backend_B ∉ SemanticTCB
```

Backend B must produce at least two realizations:

```text
Π_valid
Π_buggy
```

Required result:

```text
Π_valid -> ACCEPT
Π_buggy -> REJECT
```

If a new backend must be trusted merely because it is new, the core hypothesis is weakened.

If the backend must recover hidden source/frontend information because `K` is insufficient, the Persistent Authority experiment fails.

## 7. Phase 2 — Adaptive population

Once admission works independently of the producer, maintain multiple valid realizations at once.

Conceptually:

```text
Π1
Π2
...
Πn
```

Collect runtime evidence `E_t`, for example:

```text
input size
latency
throughput
warmup
transfer cost
data location
energy estimate
historical performance
```

Use it to select among semantically admitted choices.

Required separation:

```text
runtime evidence may change preference
runtime evidence must not manufacture semantic validity
```

A deliberately poor selection policy should still preserve correctness.

## 8. Phase 3 — Architectural adaptation

Only after Phases 0–2 succeed, test the stronger architectural hypothesis.

Construct or simulate two configurations such that:

```text
ExecInterface(S,Q_A) != ExecInterface(S,Q_B)
```

while both realizations remain governed by the same `K` and pass the same trust discipline.

Start with a simulator or controlled abstraction. Physical FPGA work is not required to establish the protocol boundary.

Later, a physical FPGA/soft-core experiment may test whether the same trust architecture survives an actual reconfiguration.

## 9. Metrics

The project should record at least:

```text
admission latency
evidence-generation cost
evidence-checking cost
activation overhead
runtime overhead
Semantic TCB components and size
trusted external dependencies
new-backend integration effort
rate of accepted/rejected adversarial cases
number of reusable semantic facts across backends
```

When adaptive selection is introduced, also measure:

```text
selection overhead
switch cost
warmup cost
realization performance
regret / bad-choice cost where useful
```

## 10. Falsification gates

Treat these as failures, not invitations to redefine the hypothesis after the fact:

```text
new backend must enter SemanticTCB
planner needs semantic authority
K cannot support future obligation derivation
validator must be semantically redesigned for every backend
invalid realization can bypass admission
post-admission substitution can execute
execution/guard context can be swapped after validation
architectural configuration cannot be bound to evidence
```

The architectural-adaptation extension is weakened if changing `Q` necessarily requires changing the semantic authority rather than deriving a new realization against the same `K`.

## 11. Implementation order

Recommended experimental order:

```text
1. tiny deterministic K
2. reference P
3. explicit DeriveObligation
4. evidence checker
5. immutable realization binding
6. ActivationGate
7. adversarial tests 1–14
8. freeze boundary
9. Backend B developed afterward
10. adaptive population
11. simulated architectural adaptation
12. physical reconfiguration only if justified
```

Do not build the SPL surface language first. Do not begin with FPGA. The first success criterion is a correct trust boundary, not a spectacular demo with blinking hardware.

## 12. Execution status

This section records which parts of the plan above have actually been executed. It does not modify the frozen theory or the frozen implementation plan.

```text
Stage 0  Validation Feasibility Spike   EXECUTED — PASS
Stage 1  Semantic Vertical Slice        EXECUTED — COMPLETE
Stage 2  Trust Vertical Slice           NOT STARTED
```

**Stage 0 — executed.** Evidence, harness and report: [`../experiments/validation-spike/`](../experiments/validation-spike/).

Outcome:

```text
STAGE 0 PASS — CPC/ETHOS SELECTED

cvc5 -> CPC    -> Ethos     PASS
cvc5 -> Alethe -> Carcara   FAIL — TRUST/Hole
```

The selected mechanism requires the proof to carry an explicit `(reference "<problem>.smt2")` command; Ethos' `--reference=` command-line option does not enforce the assumption-binding check on the pinned version and would produce a silent false accept. The solver-in-TCB fallback was **not** required and is not adopted.

Stage 0 used hand-written `QF_BV` fixtures. It did not implement `DeriveObligation`, the artifact layer, `K_ref`/`P_ref` identity, or anything else from Stage 1 onward.

**Stage 1 — executed.** Code, report and Definition of Done: [`STAGE-1-SEMANTIC-SLICE.md`](./STAGE-1-SEMANTIC-SLICE.md), implemented in [`crates/spl-core`](../crates/spl-core) and [`crates/spl-plan`](../crates/spl-plan).

Outcome:

```text
K representable and executable            yes
P representable, validated, executable    yes
K evaluator independent of Plan VM        yes
one K, two structurally different valid P yes
well-typed P that is semantically wrong   yes, disagrees on 255 of 256 inputs
deterministic identifiable K/P/Q artifacts yes
```

Stage 1 implemented no trust mechanism at all: no `DeriveObligation`, `VCArtifact`, `ValidationEvidence`, `AssurancePolicy`, `AdmissionRecord`, `ActivationGate`, guard evaluation, planner or SMT. The wrong realization is *observed* to disagree by a test; nothing in the system rejects it, because nothing in the system yet admits anything.

The adversarial matrix in sections 5–8 above remains unconfirmed by execution: it depends on the admission boundary, which arrives in Stage 2.
