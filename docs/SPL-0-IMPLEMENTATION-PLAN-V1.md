# SPL-0 Implementation Plan V1

**Status:** IMPLEMENTATION PLAN V1 — FROZEN  
**Theoretical dependency:** [`SPL-ARCH-V1.md`](./SPL-ARCH-V1.md)  
**Experimental dependency:** [`EXPERIMENTS-V1.md`](./EXPERIMENTS-V1.md)  
**Implementation status:** not started

## 1. Purpose

This document freezes the first implementation plan for SPL-ARCH V1 after two consecutive structurally stable adversarial planning rounds and a final implementation-readiness audit.

The implementation goal is deliberately narrow:

> Demonstrate that a future, untrusted realization producer can introduce a new correct realization of one persistent semantic contract while an incorrect realization from the same trust domain is rejected, without granting the producer or adaptive planner semantic authority.

The first complete implementation is called **SPL-0**.

SPL-0 is not intended to be general-purpose, self-hosting, production-safe, hardware-reconfigurable, or high-performance. It exists to test the frozen trust architecture honestly before larger language or hardware claims are attempted.

## 2. End-to-end target

A complete SPL-0 execution must eventually follow this path:

```text
program.spl
    ↓
Construction TCB
    ↓
standalone K artifact
    ↓
candidate producer
    ↓
P + Gv + Q requirements
    ↓
trusted DeriveObligation
    ↓
VCArtifact
    ↓
untrusted proof production
    ↓
ValidationEvidence
    ↓
trusted evidence checking
    ↓
Admission
    ↓
adaptive planner
    ↓
ActivationGate
    ↓
exact admitted Plan VM execution
    ↓
ObservedOutcome
```

The source/frontend must no longer be required after the standalone `K` artifact is created.

## 3. SPL-0 semantic scope

### 3.1 Initial types

```text
Bool
u8
u32
i32
```

### 3.2 Initial semantic operations

```text
constants
inputs
if
comparisons
wrapping add/sub/mul
checked add/sub/mul
Return
SemanticError
```

### 3.3 Explicitly out of scope for first release

```text
general loops
recursion
mutable heap
general observable effects
concurrency
weak memory
vectors / Core B
native machine code
GPU
RISC-V extensions
FPGA / real architectural adaptation
```

These exclusions prevent termination, aliasing, state, concurrency and target-semantics problems from obscuring the trust experiment.

## 4. Primary implementation stack

The primary implementation language is **Rust**.

Runtime-Semantic-TCB crates should use:

```rust
#![forbid(unsafe_code)]
```

unless a later explicit audit authorizes an exception.

External proof producers/checkers should initially be invoked as external processes rather than pulled into the runtime via unsafe FFI when a process boundary can express the required interface.

## 5. Three trust boundaries

### 5.1 Construction TCB

Responsible for:

```text
SPL source -> K
```

Includes conceptually:

```text
lexer/parser
typechecker
semantic analysis
K construction
K canonical serialization
```

A bug here may make the constructed semantic contract differ from the source program. SPL-0 therefore trusts the frontend during construction.

After K is materialized, this TCB is not required for future-backend admission or execution.

### 5.2 Runtime Semantic TCB

Responsible for semantic authorization of execution.

Includes conceptually:

```text
K decoder
P decoder
Candidate decoder
canonical artifact handling
identity verification
K semantics
P semantics / Plan VM
K symbolic encoder
P symbolic encoder
DeriveObligation
ValidationEvidence parser
independent proof checker and rule/signature implementation
Assurance Policy enforcement
Gv evaluator
S/Q model
Admission binding
ActivationGate
```

### 5.3 Isolation TCB

Responsible for maintaining the process boundary against hostile producers/planners.

Includes mechanisms actually relied upon from:

```text
OS process isolation
process creation
IPC
filesystem protections, if relied upon
```

Backend and planner remain outside both semantic TCBs even if they reuse public source code or libraries. Trust is about authority and failure consequences, not code ancestry.

## 6. Initial workspace shape

Keep the workspace small until responsibilities justify further split:

```text
crates/
    spl-core/
    spl-plan/
    spl-trust/
    spl-runtime/

bins/
    spl/
    spl-backend-reference/
    spl-planner-reference/
```

A dedicated frontend or protocol crate may be split later for code organization. Such a split is not itself an architectural change.

## 7. K and P must remain genuinely distinct

The implementation must satisfy:

```text
K representation != P representation
K evaluator      != P interpreter
K SMT encoder    != P SMT encoder
```

Shared primitive type definitions or semantic constants are allowed, but admission must never reduce to structural equality, known-good hashes, or checking that `P` equals one blessed lowering of `K`.

### 7.1 K representation

K is initially a typed semantic tree capable of expressing Core A. Conceptually:

```text
KExpr =
    Input
    Const
    If
    Eq
    Lt
    WrappingAdd
    WrappingSub
    WrappingMul
    CheckedAdd
    CheckedSub
    CheckedMul
    Return
    SemanticError
```

### 7.2 P representation

P is a separate typed register/SSA-like Plan IR with explicit control flow and overflow handling. It has no general loops in Core A.

Example shape:

```text
entry:
    r0 = input 0
    r1 = const INT_MAX
    (r2, ov) = checked_add r0, r1
    branch ov -> overflow, continue

overflow:
    error Overflow

continue:
    ...
```

The same K must be able to admit multiple structurally different P artifacts, and a well-typed executable P must be able to disagree semantically with K.

## 8. Initial execution substrate

SPL-0 uses a small trusted **Plan VM**.

The Plan VM belongs to the Runtime Semantic TCB and interprets the frozen Plan IR semantics. SPL-0 does not initially attempt to admit arbitrary x86-64, LLVM IR or Wasm implementations.

For the first release:

```text
Q = PlanVmConfiguration
```

Q can bind such facts as:

```text
Plan IR version
primitive-set version
supported integer widths
arithmetic modes
```

This exercises Q binding without claiming real architectural adaptation.

## 9. Artifact model

Artifacts must be:

```text
versioned
strictly decoded
canonically serialized
content-identifiable
bounded when parsing untrusted bytes
```

Deterministic CBOR is the leading serialization candidate, but no Rust CBOR library is frozen by this plan. The chosen implementation must specify one canonical encoding and reject ambiguous alternate encodings.

The binding invariant is:

```text
bytes identified
==
bytes semantically decoded
==
bytes eventually executed
```

`ContractRef`, `PlanRef`, `QRef`, `VCRef` and related IDs identify exact artifact content, not semantic equivalence classes.

## 10. VCArtifact and ValidationEvidence

Conceptual VC artifact:

```text
VCArtifact {
    schema_version
    contract_ref
    guard_ref
    substrate_epoch
    q_ref
    p_ref
    encoding_profile
    logic
    canonical_problem_bytes
}
```

Conceptual identity:

```text
VCRef = Digest(canonical VCArtifact bytes)
```

Conceptual validation evidence:

```text
ValidationEvidence {
    evidence_format
    checker_profile
    vc_ref
    proof_bytes
}
```

Before semantic proof checking, the SPL layer must require:

```text
ValidationEvidence.vc_ref == computed VCRef
```

The independent checker must additionally establish that the proof is valid for the logical problem corresponding to that exact VC.

## 11. DeriveObligation target

For candidate `(K, Gv, S, Q, P)`, the initial symbolic obligation searches for a counterexample to preservation:

```text
Gv(x, env)
AND
Run_P(x, env) != embed(K(x, env))
```

Core A is intentionally selected so representative obligations can initially fit `QF_BV`.

Admission policy for solver results:

```text
SAT                   -> REJECT
UNKNOWN               -> NOT ADMITTED
TIMEOUT               -> NOT ADMITTED
ERROR                 -> NOT ADMITTED
UNSAT without accepted evidence -> NOT ADMITTED
UNSAT + independently accepted evidence -> MAY ADMIT
```

Correctness wins over availability.

## 12. ValidationEvidence technology remains Stage-0 gated

No proof pipeline is frozen before Stage 0.

Candidates:

```text
cvc5 -> CPC -> Ethos
cvc5 -> Alethe -> Carcara
```

Fallback:

```text
translation validation with cvc5 explicitly inside Runtime Semantic TCB
```

If the fallback is used, SPL-0 must not claim independent proof-carrying admission for that implementation revision.

Pipeline-selection priority:

```text
1. no unchecked/trusted/hole step
2. exact proof/VC binding
3. reproducible pinned checker
4. smallest/auditable checker TCB
5. widest Core-A coverage
6. proof size / checking cost
```

Any preprocessing between the trusted VC and checked proof must be proof-producing, explicitly trusted, or prohibited.

## 13. Common-mode semantic risk

Independent K/P representations remove trivial tautology but do not eliminate a common semantic bug shared by K evaluator, P interpreter and symbolic encoders.

This is a Runtime Semantic TCB correctness risk, not an architectural exception.

Required mitigation:

```text
concrete K evaluator
concrete P interpreter
symbolic K encoder
symbolic P encoder
primitive conformance fixtures
exhaustive u8 cases where feasible
i32/u32 boundary corpus
semantic mutation tests
```

Expected-result fixtures must not merely call another component under test.

## 14. Central demonstration program

Use checked arithmetic to create an observable intermediate-overflow distinction.

Conceptual K:

```text
t = checked_add(x, INT_MAX)

if overflow(t):
    SemanticError(Overflow)
else:
    checked_add(t, -INT_MAX)
```

Reference valid P:

```text
perform both checked operations
```

Specialized valid P:

```text
if x > 0:
    SemanticError(Overflow)
else:
    Return(x)
```

Tempting invalid P:

```text
Return(x)
```

For `x = 1`, K produces `SemanticError(Overflow)` while the invalid P returns `1`. This is a well-formed semantic error, not merely malformed input.

## 15. Stage graph

```text
Stage 0  Validation Feasibility Spike
Stage 1  Semantic Vertical Slice
Stage 2  Trust Vertical Slice
Stage 3  Minimal SPL Frontend
Stage 4  Reference End-to-End
Stage 5  Official T0 Freeze
Stage 6  Future Untrusted Backend B
Stage 7  Adaptive Planner
Stage 8  SPL-0 Release
```

The graph is intentionally acyclic. Do not move to the next stage until the current Definition of Done is satisfied and reviewed.

---

# Stage 0 — Validation Feasibility Spike

## Goal

Determine whether representative SPL-0 `QF_BV` obligations can produce reproducible evidence that is bound to the exact problem and independently checked without silently trusting cvc5.

## Cases

Build approximately 5–10 manual SMT-LIB cases including at least:

```text
wrapping equivalence                  VALID
checked intermediate-overflow rewrite INVALID
guarded specialization                VALID
guard widening                        INVALID
outcome-tag mismatch                  INVALID
changed K / proof replay              REJECT
changed P / proof replay              REJECT
changed guard / proof replay          REJECT
```

## Pipelines

```text
Pipeline A: cvc5 -> CPC -> Ethos
Pipeline B: cvc5 -> Alethe -> Carcara
```

## Definition of Done

A pipeline may be selected only if:

```text
expected UNSAT cases produce evidence
the independent checker accepts good evidence
tampered proof is rejected
modified original problem rejects old proof
proof/problem binding is understood and demonstrated
no disallowed trust/hole step is required for ACCEPT
tool/checker versions are reproducibly pinned
checker rule/signature dependencies are recorded
```

If neither pipeline passes, document the failures and select the solver-in-TCB fallback explicitly.

No frontend, Plan VM, runtime, planner or SPL syntax is to be implemented in Stage 0.

---

# Stage 1 — Semantic Vertical Slice

Implement:

```text
K Core A
SemanticOutcome
ObservedOutcome
embed
K evaluator
Plan IR
Plan VM
P interpreter
minimal S/Q
canonical serialization
content identity
```

## Definition of Done

```text
K executes concretely
P executes concretely
same semantic task has >=2 structurally distinct valid P
well-typed P_bad produces a semantic mismatch
K bytes differ from P bytes
K evaluator and P interpreter are separate
primitive conformance passes
u8 exhaustive tests pass where applicable
integer boundary corpus passes
```

---

# Stage 2 — Trust Vertical Slice

Add:

```text
K symbolic encoder
P symbolic encoder
DeriveObligation
VCArtifact / VCRef
ValidationEvidence
selected proof-producer integration
trusted checker
Assurance Policy
AdmissionRecord
ActivationGate
```

## Mandatory integration gate

Stage 0 uses manual VCs. Stage 2 is not complete until a **real VC generated by `DeriveObligation`** passes the selected evidence pipeline.

## Adversarial tests

At minimum:

```text
semantic bug
irrelevant evidence
proof replay
wrong K
wrong P
wrong Q
guard widening
post-admission P mutation
post-admission Q mutation
invocation substitution
binding attack
malformed artifact
changed VC
```

Conceptual admission record:

```text
AdmissionRecord {
    contract_ref
    guard_ref
    s_epoch
    q_ref
    p_ref
    vc_ref
    evidence_ref
    assurance_policy_ref
}
```

ActivationGate must revalidate exact admission/binding before Plan VM execution.

---

# Stage 3 — Minimal SPL Frontend

Implement only enough surface language for Core A:

```text
source .spl
lexer/parser
typechecker
semantic construction
K artifact emission
```

## Definition of Done

A real `.spl` program must produce the exact K artifact shape intended for T0. After that artifact exists, removing the source/frontend from the runtime path must not prevent backend generation, admission or execution.

---

# Stage 4 — Reference End-to-End

Run the complete reference path before freezing anything:

```text
program.spl
    -> frontend
    -> K
    -> reference backend
    -> P
    -> DeriveObligation
    -> V
    -> Admission
    -> ActivationGate
    -> Plan VM
```

## Definition of Done

```text
valid reference realization admitted
invalid reference realization rejected
complete structured event trail exists
source no longer required after K artifact creation
Stage-2 binding attacks remain passing
```

---

# Stage 5 — Official T0 Freeze

Freeze and record:

```text
K schema + semantics
P schema + semantics
Candidate ABI
canonical serialization rules
identity algorithms
Core A semantics
frontend subset
VC semantics
SMT encoding profile
V format
checker + rule/signature/version
Assurance Policy
Plan VM version
ActivationGate semantics
S/Q schema
reference fixtures
adversarial corpus
commit SHA
```

Create an unambiguous Git marker such as:

```text
tag: spl0-t0
```

**Backend B must not exist before this marker.**

The proof-producer implementation itself need not be the only future proof producer. The frozen contract is the accepted evidence format/checker/semantics/policy.

---

# Stage 6 — Future Untrusted Backend B

Create Backend B only after T0.

Inputs allowed:

```text
K artifact
public frozen schema/ABI documentation
Q capabilities
```

Outputs:

```text
P
Gv
Q requirements
optionally V
```

Backend B cannot create trusted admission state or invoke privileged activation capability.

It must be:

```text
Future
Untrusted
Artifact-driven
```

## K_demo test

Backend B must consume the pre-T0 central contract and produce:

```text
generic valid realization
nontrivial specialized valid realization
well-formed semantic-invalid realization
guard-widened realization
```

The valid artifacts must be admitted; invalid artifacts must be rejected without changing the T0 interfaces.

## Anti-hardcode K_holdout

After the implementation of Backend B is frozen in a commit, create a new Core-A K instance. Record:

```text
Backend B commit SHA
K_holdout source
K_holdout artifact
generation parameters, if any
results
```

Backend B must decode the previously unseen K structure and produce a generic valid P. It need not discover a sophisticated optimization.

This exists to rule out `ContractRef -> prewritten P` lookup-table explanations.

## Capability audit

Do not ban public semantic-library reuse by ancestry. Instead prove that Backend B lacks privileged capability to:

```text
admit
activate
mutate trusted runtime state
replace VC
modify A
bypass checker
require hidden frontend state for correctness
```

---

# Stage 7 — Adaptive Planner

Run the planner as an external untrusted process.

Runtime sends candidate references and non-authoritative preference observations. Planner returns only a selection such as:

```text
Select(candidate_ref)
```

Initial preference evidence may include:

```text
input size
invocation count
historical elapsed time
```

No ML/RL/autotuner complexity is required.

## Required population

```text
valid slow candidate
valid fast candidate
invalid fastest candidate
```

Expected behavior:

```text
slow valid selected -> ALLOW
fast valid selected -> ALLOW
invalid candidate selected -> DENY
```

## Definition of Done

```text
>=2 admitted realizations coexist
planner is outside Runtime Semantic TCB
selection can change using preference evidence
slow valid candidate can execute
rejected candidate never executes
hostile planner cannot forge trusted handles
stale/invalid selections are denied
```

---

# Stage 8 — SPL-0 Release

Only here may the project claim a **first real SPL implementation**.

The release must demonstrate:

```text
real source language exists
source produces standalone K
K survives source/frontend disappearance
K/P representations are genuinely distinct
reference backend works
validation/admission pipeline works
T0 is recorded
future Backend B was created after T0
Backend B consumes frozen K artifacts
valid Backend B realization is admitted
invalid Backend B realization is rejected
K_holdout defeats hardcoding explanation
planner is outside Runtime Semantic TCB
multiple admitted realizations coexist
invalid realization cannot execute
ActivationGate preserves exact binding
adversarial tests pass
TCB is documented
entire claim is reproducible from repository
```

## Structured instrumentation

At minimum log structured events for:

```text
candidate_proposed
obligation_derived
evidence_checked
candidate_admitted
candidate_rejected
planner_selected
activation_allowed
activation_denied
execution_started
execution_completed
```

Logs support experiment auditability; they are not themselves semantic proof.

## Mutation testing

Deliberately mutate semantic/identity components such as:

```text
K encoder
P encoder
P interpreter
overflow rule
Outcome tag
VC bytes
proof
P artifact
Q artifact
invocation
```

Tests must expose the resulting semantic or binding failure.

## Fuzzing

Once decoders stabilize, fuzz untrusted parsing surfaces:

```text
K decoder
P decoder
Candidate decoder
ValidationEvidence parser
```

Malformed bytes must not compromise the trusted runtime.

## TCB documentation

Maintain `docs/TCB-V1.md` once implementation begins, recording:

```text
component
why it is trusted
failure consequence
external dependencies
unsafe-code status
proof-checker/rule dependencies
```

## Anti-demo rules

SPL-0 does not count as successful if any of the following explains the result:

```text
validator accepts known-good hashes
P is serialized K
Backend B uses a ContractRef lookup table
future backend requires hidden frontend state
invalid cases are only malformed data
planner cannot actually choose alternatives
proof checker trusts producer assertions
solver UNSAT alone authorizes execution while claimed outside TCB
```

## What SPL-0 success does not prove

Even a successful release does not establish:

```text
universal originality
formal verification of the entire TCB
security against a compromised OS
performance superiority
general-program scalability
native-code validation
physical architectural adaptation
```

It establishes only the frozen experimental claims that were actually tested.

## Failure policy

After T0, a frozen-interface failure is experimental evidence. Do not silently move T0 to make Backend B work. A corrected design must become a new implementation revision.

Before T0, Stage-0 through Stage-4 findings may refine implementation details as long as they do not silently alter SPL-ARCH Theoretical V1.

## First action

The first implementation task is **Stage 0 — Validation Feasibility Spike**.

Its execution prompt is maintained separately in [`AGENT-C-STAGE0.md`](./AGENT-C-STAGE0.md).
