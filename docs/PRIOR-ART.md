# SPL-ARCH V1 — Prior-Art Status

**Status:** frozen research-status note for V1  
**Purpose:** document what the project does *not* claim, the strongest antecedents found during adversarial review, and the remaining provisional demarcation.

This file is not a claim of exhaustive literature coverage.

## 1. Epistemic rule

The V1 investigation uses the following interpretation:

```text
DOCUMENTED
    supported by inspected literature/project material

INFERENCE
    conclusion drawn from documented material

PROPOSAL
    SPL-ARCH architectural claim not yet implemented

UNKNOWN
    not established by the investigation
```

Absence of an equivalent in the inspected sources is not evidence that no equivalent has ever existed.

## 2. Ideas explicitly not claimed as novel

The adversarial process found substantial prior art for all of the following ideas considered in isolation:

```text
preserving high-level semantic information
multi-level / semantic IRs
adaptive compilation and JIT
PGO and runtime feedback
autotuning
algorithm/schedule separation
heterogeneous CPU/GPU execution
heterogeneous runtime scheduling
runtime generation of accelerator implementations
FPGA/HLS realization
custom or retargetable ISA work
formal rewrite validation
verified compilation
persistent compiler abstractions / e-graphs
proof-carrying code
proof-carrying hardware
software/hardware co-verification
future producers supplying realizations plus evidence
verification before loading reconfigurable hardware
```

SPL-ARCH V1 must not present any one of those as its invention.

## 3. Important antecedent families

The review considered, among others:

- LLVM and MLIR;
- PetaBricks;
- Delite;
- Halide;
- TVM / MetaSchedule;
- Truffle/Graal;
- CIRCT/Calyx;
- HPVM and HPVM2FPGA;
- ApproxHPVM and ApproxTuner;
- Exo / Exo2;
- persistent e-graph work;
- Hydride and MISAAL;
- CompCert and Alive2;
- Proof-Carrying Code (PCC);
- Proof-Carrying Hardware (PCH);
- Proof-Carrying Services (PCS);
- the SFB 901 On-The-Fly Computing research program at Paderborn.

The purpose of these comparisons was not to assemble a decorative bibliography. Each successive comparison removed overly broad originality claims from SPL-ARCH.

## 4. Main conclusions from earlier comparisons

### 4.1 Persistent semantics is not sufficient

MLIR and related compiler work already motivate delaying destructive lowering and retaining richer information across multiple abstraction levels.

Therefore:

```text
"preserve semantic information longer"
```

is not a sufficient demarcation for SPL-ARCH.

### 4.2 Adaptive planning is not sufficient

PetaBricks, Halide, TVM, Delite, Truffle/Graal, HPVM, ApproxTuner, and related systems occupy large parts of the space involving alternative realizations, profiling, scheduling, cost models, heterogeneous targets, and adaptive selection.

Therefore:

```text
semantics + target + measurements -> choose implementation
```

is not sufficient as a novelty claim.

### 4.3 Program-to-hardware is not sufficient

CIRCT/Calyx, HLS research, HPVM2FPGA, reconfigurable computing, and related work already establish routes from high-level representations toward hardware realization.

Therefore:

```text
program / IR -> specialized hardware
```

is not sufficient.

### 4.4 Formal semantics for retargeting is not sufficient

Hydride/MISAAL and related work occupy the space of semantics-driven retargeting and rewrite generation.

Therefore SPL-ARCH cannot claim novelty merely because formal semantics are used to derive target implementations.

### 4.5 Proof-carrying realization is not sufficient

Proof-Carrying Code already provides the producer/evidence/consumer-verification pattern for untrusted code producers.

Proof-Carrying Hardware extends a closely related model to dynamically reconfigurable hardware: a hardware module can be accompanied by evidence checked before configuration/execution.

Proof-Carrying Services further investigated software/hardware integration, including reconfigurable hardware and custom instructions.

Therefore the following composition alone is not novel:

```text
persistent specification
+
future producer
+
proof / certificate
+
verify before use
+
software and/or hardware realization
```

## 5. Strongest prior-art threat: SFB 901

The strongest threat found during the V1 adversarial review was the SFB 901 research program.

The relevant responsibilities identified were approximately:

```text
B4:
    proof-carrying certification / validation
    software and reconfigurable hardware
    co-verification

B2:
    configuration informed by execution / measured properties

C2:
    heterogeneous resource management
    runtime CPU/FPGA alternatives
    scheduling and migration

Proof-of-Concept:
    integration of contributions from several subprojects
```

This combination is extremely close to the broad SPL-ARCH intuition and forced the project to narrow its proposed demarcation substantially.

## 6. Result of the concentrated SFB 901 search

During the adversarial process, the inspected primary material established that:

- B4 performed proof-carrying/certification work for services and reconfigurable hardware;
- C2 generated/runtime-managed heterogeneous alternatives and scheduler/resource-management mechanisms;
- the program's Proof-of-Concept integrated components from multiple subprojects.

However, the investigation did **not establish** an explicit normative invariant equivalent to:

```text
Execute(realization)
    =>
MandatorySemanticAdmission(
    same persistent semantic authority,
    exact realization
)
```

with complete mediation over adaptive heterogeneous execution.

Likewise, the inspected material did not establish that every candidate participating in adaptive heterogeneous execution was necessarily tied to the same persistent semantic authority through per-realization proof admission and admission-to-execution integrity.

The correct V1 status is therefore:

```text
SFB 901 SUBSUMPTION:
    NOT ESTABLISHED

UNIVERSAL NOVELTY:
    NOT ESTABLISHED

SPL-ARCH DEMARCATION:
    PROVISIONALLY SURVIVES
```

This is deliberately weaker than saying that the link did not exist.

## 7. Frozen candidate demarcation

The V1 candidate demarcation is:

> SPL-ARCH is a trust protocol for adaptive realization in which no realization may be activated or executed without satisfying an obligation derived from a persistent semantic authority, while the system maintains identity between what was proved, admitted, activated, and actually executed, allowing realization producers and adaptive planners to remain outside the semantic trusted computing base.

The candidate distinction is therefore primarily a **composition and authority boundary**, not a new primitive mechanism.

## 8. What would refute the demarcation

The V1 originality line should be considered refuted if later research finds an earlier architecture that substantially combines:

```text
persistent semantic authority
trusted derivation of obligations
future / potentially untrusted realization producers
per-realization validation evidence
mandatory admission before execution
complete mediation
admission-to-execution identity/integrity
adaptive realization choice
untrusted or semantically untrusted planner
heterogeneous software/hardware realization
```

Equivalent responsibilities count even if the prior system uses entirely different terminology.

Renaming SPL-ARCH concepts is not a legitimate defense against prior art.

## 9. Research posture after V1 freeze

The theoretical search is no longer allowed to expand indefinitely before experimentation.

New literature can still:

- refute the provisional demarcation;
- motivate a later version;
- improve implementation choices;
- provide formal machinery for an existing V1 responsibility.

But optional conceptual embellishment is not a reason to reopen the V1 freeze.

The next evidence should come primarily from the experiments defined in [EXPERIMENTS-V1.md](./EXPERIMENTS-V1.md).
