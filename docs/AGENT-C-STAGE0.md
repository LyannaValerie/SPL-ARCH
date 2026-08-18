# Agent C Prompt — Stage 0 Validation Feasibility Spike

> This file is an execution prompt for the first implementation agent. It is intentionally limited to Stage 0 of [`SPL-0-IMPLEMENTATION-PLAN-V1.md`](./SPL-0-IMPLEMENTATION-PLAN-V1.md).

---

## ROLE

You are **Agent C — Implementer** for SPL-ARCH.

You are not being asked to implement SPL, the frontend, the runtime, the Plan VM, the planner, or the Admission Kernel.

Your only task is to execute **Stage 0 — Validation Feasibility Spike** and produce reproducible evidence that determines which validation pipeline, if any, is viable for the later SPL-0 trust path.

Treat the repository as an experimental research repository. Do not silently turn proposals into confirmed facts.

## REPOSITORY

Repository:

```text
LyannaValerie/SPL-ARCH
```

Before doing anything else:

1. inspect the repository root;
2. inspect the nearest `AGENTS.md` if one exists;
3. read these files in this order:
   - `docs/SPL-ARCH-V1.md`
   - `docs/SPL-0-IMPLEMENTATION-PLAN-V1.md`
   - `docs/EXPERIMENTS-V1.md`
   - `docs/PRIOR-ART.md`
4. inspect current Git status and branch;
5. do not overwrite unrelated user changes.

The frozen theory is authoritative. If this prompt appears to conflict with `docs/SPL-ARCH-V1.md`, stop and report the conflict rather than silently changing the theory.

## BRANCH

Create/use a dedicated implementation branch:

```text
feat/spl0-validation-spike
```

Do not implement Stage 1 on this branch unless explicitly authorized later.

## STAGE-0 QUESTION

Answer experimentally:

> Can representative SPL-0 `QF_BV` obligations produce reproducible evidence that is bound to the exact logical problem and independently checked, without silently trusting cvc5 as the semantic authority?

Two candidate pipelines must be evaluated:

```text
Pipeline A:
    cvc5 -> CPC -> Ethos

Pipeline B:
    cvc5 -> Alethe -> Carcara
```

If neither satisfies the gate, that is a valid Stage-0 result. Do not manipulate the experiment until one passes. The planned fallback is to put the solver explicitly inside the Runtime Semantic TCB in a later stage.

## PRIMARY-SOURCE REQUIREMENT

Before relying on tool-specific behavior, verify it from current official/project documentation or repository documentation.

In particular verify:

- the cvc5 version used;
- exact cvc5 proof-production flags supported by that version;
- the compatible Ethos version/signature for CPC;
- Carcara's actual CLI for checking Alethe proofs against the original SMT-LIB problem;
- any proof-completeness/safe-mode requirements;
- how trust/hole/incomplete proof steps are represented by the versions actually used.

Record links or source identifiers in the Stage-0 report.

Do not copy command lines from this prompt blindly if the installed/pinned tool version contradicts current primary documentation.

## KNOWN CURRENT STARTING POINTS TO VERIFY

Current project documentation has historically used commands in this family:

```text
cvc5 --dump-proofs --proof-format-mode=cpc ...
cvc5 --dump-proofs --proof-format-mode=alethe ...
```

Current cvc5 documentation describes CPC as its native/default proof format and Ethos as the CPC checker. cvc5's release documentation also exposes an official helper in the cvc5 source tree named approximately:

```text
./contrib/get-ethos-checker
```

Carcara's project README currently documents proof checking in the form:

```text
carcara check PROOF_FILE ORIGINAL_PROBLEM.smt2
```

These are research starting points, not permission to skip version verification.

## TOOL PINNING

The result must be reproducible.

Record at minimum:

```text
OS / architecture
cvc5 version or exact commit SHA
Ethos version or exact commit SHA
Carcara version or exact commit SHA
Rust/Cargo version if Carcara is built locally
compiler/build-tool versions when relevant
all non-default cvc5 flags used
```

Prefer released versions when they satisfy the experiment. If development commits are required, pin exact commit SHAs and explain why a release was insufficient.

Do not write global machine state into the repository.

If tools need to be built locally, prefer a documented local/cache path or a reproducible setup script. Do not vendor large third-party source trees or compiled binaries into the SPL-ARCH repository.

## DIRECTORY LAYOUT

Create a bounded Stage-0 area, preferably:

```text
experiments/
  validation-spike/
    README.md
    cases/
    scripts/
    results/
```

A reasonable structure is:

```text
experiments/validation-spike/
├── README.md
├── cases/
│   ├── 01_wrapping_equiv_valid.smt2
│   ├── 02_checked_intermediate_overflow_invalid.smt2
│   ├── 03_guarded_specialization_valid.smt2
│   ├── 04_guard_widening_invalid.smt2
│   ├── 05_outcome_tag_mismatch_invalid.smt2
│   └── ...
├── scripts/
│   ├── run-all.sh
│   ├── run-cpc-ethos.sh
│   ├── run-alethe-carcara.sh
│   └── mutate-binding.sh
└── results/
    ├── toolchain.txt
    ├── summary.md
    └── raw/   # only small textual outputs worth preserving
```

Adjust names if necessary, but keep the spike isolated and obvious.

## DO NOT CREATE YET

Do not create SPL architecture code merely to look productive.

Do not implement:

```text
SPL parser/frontend
K Rust AST
P Plan IR
Plan VM
spl-core
spl-plan
spl-trust
spl-runtime
AdmissionRecord
ActivationGate
planner
future backend
CBOR artifact system
```

Stage 0 uses hand-written SMT-LIB fixtures deliberately.

## LOGIC

Use quantifier-free bit-vectors where possible:

```text
(set-logic QF_BV)
```

The fixtures are prototypes of future obligations, not yet output of `DeriveObligation`.

Prefer small bit widths (`8`, sometimes `32`) when they express the semantic property clearly and make counterexamples easy to inspect.

## OUTCOME REPRESENTATION

At least one case must distinguish semantic outcome tags rather than comparing only raw integer values.

Use an explicit finite/tag representation sufficient to distinguish conceptual outcomes such as:

```text
Return(value)
SemanticError(Overflow)
```

You do not need to encode the entire future `ObservedOutcome` domain in Stage 0 unless it is useful. The point is to prove that `Return(x)` and `SemanticError(Overflow)` cannot be conflated by the future validation encoding.

Document the exact encoding chosen.

## CHECKED ARITHMETIC

At least one fixture must model checked arithmetic with an explicit overflow condition.

Do not model checked addition merely as wrapping `bvadd` and then claim it covers overflow semantics.

For signed arithmetic, carefully encode signed overflow. Validate boundary examples separately so the fixture itself is not based on a mistaken overflow rule.

## REQUIRED CASES

Build at least the following conceptual cases.

### Case 1 — wrapping equivalence — expected VALID

Construct two genuinely different but equivalent bit-vector expressions, then assert that they differ.

Expected solver result:

```text
UNSAT
```

A proof should be produced and independently accepted.

### Case 2 — checked intermediate-overflow rewrite — expected INVALID

Model a K-like computation where an intermediate checked operation can produce `SemanticError(Overflow)`, but an invalid candidate collapses the arithmetic and ignores that intermediate overflow.

The conceptual family is:

```text
t = checked_add(x, MAX)
if overflow(t):
    SemanticError(Overflow)
else:
    checked_add(t, -MAX)
```

versus an invalid candidate resembling:

```text
Return(x)
```

There must exist an input such as a positive boundary-adjacent value where the outcomes differ.

Expected result for the counterexample formula:

```text
SAT
```

Record the model/counterexample if conveniently available.

### Case 3 — guarded specialization — expected VALID

Create a specialization that is valid only under an explicit guard `Gv`.

Formulate the future-VC shape conceptually as:

```text
Gv(x) AND P(x) != K(x)
```

Expected result:

```text
UNSAT
```

Proof must be independently accepted.

### Case 4 — guard widening — expected INVALID

Take the specialization from Case 3 and weaken/remove the guard so a counterexample exists.

Expected result:

```text
SAT
```

### Case 5 — outcome-tag mismatch — expected INVALID

Construct a problem where the numeric payload can coincide but the semantic tag differs, e.g. conceptual:

```text
Return(0)
!=
SemanticError(Overflow)
```

Expected result:

```text
SAT
```

or an equivalent fixture that demonstrates the tag distinction unambiguously.

### Additional cases

Add enough cases to reach roughly 5–10 total if doing so exercises proof/problem binding, bit widths, signed comparisons, or checked/wrapping semantics without turning Stage 0 into a language implementation.

## IMPORTANT DISTINCTION: SAT VS UNSAT

Only UNSAT/valid-equivalence cases are expected to have proof evidence for admission.

SAT cases are negative validation examples. Their role is to demonstrate that the proposed equivalence is false and therefore would not be admitted.

Do not fabricate a proof expectation for a SAT case.

## PIPELINE A — CPC / ETHOS

For each expected-UNSAT case:

1. generate a CPC proof with the pinned cvc5;
2. verify it with the matching pinned Ethos/signature;
3. capture checker exit status/output;
4. determine whether the proof contains any trust step;
5. require the checker result to indicate a complete/correct proof rather than an incomplete proof;
6. investigate how the proof's input assumptions are bound to the exact original SMT-LIB problem.

The binding question is mandatory.

CPC proofs are described as closed refutations whose assumptions come from the input, but Stage 0 must demonstrate how SPL can prevent this failure:

```text
proof is internally correct
but for a different logical problem
```

If Ethos does not directly consume the original SMT problem, document precisely how the CPC assumptions are matched to the canonical input. A small future trusted binder is acceptable conceptually, but do not pretend the binding exists if it does not.

If safe exact binding cannot be demonstrated at Stage 0, mark CPC/Ethos as failing the Stage-0 gate even if Ethos says `correct` for a standalone certificate.

## PIPELINE B — ALETHE / CARCARA

For each expected-UNSAT case:

1. generate Alethe proof output with cvc5;
2. preserve the exact SMT-LIB input used;
3. run Carcara against both the proof and original SMT-LIB problem;
4. capture exit status/output;
5. inspect the proof for `hole`, unsupported trusted steps, or other unchecked escape hatches supported by the actual tool versions;
6. require a clean acceptable proof according to the Stage-0 assurance rule.

Because Carcara accepts proof + original problem, it is especially important to run the mutation/misbinding tests below rather than merely assuming binding works.

## TRUST / HOLE POLICY

Stage-0 independent admission evidence is unacceptable if acceptance relies on an unchecked rule that proves an arbitrary proposition.

For each pipeline, explicitly identify:

```text
what the checker considers complete
what it considers incomplete/trusted
what syntactic proof steps indicate a hole/trust escape
whether the checker rejects or merely warns
```

Do not rely only on `grep` if the checker has a stronger semantic completeness result. Use both where useful.

If a proof is accepted by a checker despite an unchecked hole under the selected policy, the pipeline fails this gate unless the unchecked component is explicitly moved into the Runtime Semantic TCB. Do not do that silently in Stage 0.

## PROOF / PROBLEM MISBINDING TESTS

These tests are mandatory for every candidate pipeline that can produce an accepted good proof.

Start with:

```text
problem_good + proof_good -> ACCEPT
```

Then preserve the proof and modify the logical problem.

At minimum test:

```text
same proof + changed constant
same proof + changed guard
same proof + changed equality/comparison
```

Expected:

```text
REJECT
```

For Alethe/Carcara, run the checker directly with the modified original problem.

For CPC/Ethos, test the strongest available input-binding mechanism and document whether an additional SPL-side binder would be needed later.

Do not fake `K_ref`/`P_ref` hashing in Stage 0. There is no artifact layer yet. The Stage-0 equivalent is exact logical-problem mutation.

## PROOF MUTATION TEST

Take at least one independently accepted proof and create a controlled textual mutation that should invalidate the proof, such as changing a justified conclusion, premise reference, term, or rule argument without making the parser failure itself the only result.

Expected:

```text
REJECT / non-zero checker result
```

Record exactly what was mutated.

## CVC5 SAFETY SETTINGS

Investigate and record whether the pinned cvc5 supports and benefits from settings such as:

```text
--safe-mode=safe
--proof-mode=full-proof-strict
```

and whether these interact correctly with the selected proof format and `QF_BV` cases.

Do not assume that safe mode alone proves certificate completeness. Independent checker output and trust/hole inspection remain authoritative for this experiment.

Also inspect the actual default/value of any option equivalent to `proof-allow-trust`. If a no-trust setting exists and works for the fixture, exercise it and record the result. Do not assume the option name or behavior without verifying the pinned version.

## AUTOMATION

Create scripts so the full Stage-0 matrix can be rerun with one top-level command, ideally something like:

```bash
./experiments/validation-spike/scripts/run-all.sh
```

The script should:

1. fail loudly on missing tools;
2. print/record tool versions;
3. run each fixture through cvc5;
4. compare SAT/UNSAT with expected result;
5. for UNSAT cases, attempt each proof pipeline;
6. run independent checker;
7. run proof mutation and problem-misbinding tests where applicable;
8. emit a concise machine-readable or TSV/CSV/JSON summary plus human-readable Markdown summary;
9. return non-zero if the harness itself malfunctioned.

A pipeline failing its scientific gate is not the same as the harness malfunctioning. Preserve that distinction.

## RESULT MATRIX

The final report should contain a matrix conceptually like:

```text
case | expected | cvc5 | CPC proof | Ethos | trust-free | CPC binding | Alethe proof | Carcara | hole-free | Alethe binding
```

Use clearer formatting if desired.

For each pipeline conclude exactly one:

```text
PASS
FAIL — TRUST/Hole
FAIL — CHECKER REJECTS
FAIL — PROBLEM BINDING NOT ESTABLISHED
FAIL — CORE-A COVERAGE
FAIL — TOOLING/REPRODUCIBILITY
NOT TESTABLE — with evidence
```

## PIPELINE SELECTION

Choose a preferred Stage-0 winner only after data exists.

Priority order:

```text
1. no unchecked/trusted/hole step
2. exact proof/problem binding
3. reproducible pinned checker
4. smaller/auditable checker TCB
5. wider Core-A coverage
6. proof size/checking cost
```

Do not choose by aesthetics or familiarity.

Possible final outcomes:

### Outcome A

```text
SELECT CPC / Ethos
```

only if complete trust-free proof and exact-input binding are established.

### Outcome B

```text
SELECT Alethe / Carcara
```

only if complete acceptable proof and exact-input binding are established.

### Outcome C

```text
NO INDEPENDENT PIPELINE PASSES
FALLBACK REQUIRED: SOLVER INSIDE RUNTIME SEMANTIC TCB
```

This is a legitimate and scientifically useful result.

## DO NOT CHANGE THEORY

Do not alter:

```text
K/A/S/Q/P/Gv/VC/V responsibilities
complete mediation
Admission-to-Execution Integrity
T0 placement
future-backend experiment
planner trust boundary
```

because a proof tool is inconvenient.

Stage 0 selects an implementation mechanism; it does not renegotiate SPL-ARCH V1.

## DO NOT START T0

Stage 0 is far before official T0.

Do not create the `spl0-t0` tag.

Do not create Backend B.

Do not claim interfaces are frozen beyond the Stage-0 experimental artifacts.

## DOCUMENTATION TO CREATE

At minimum commit:

```text
experiments/validation-spike/README.md
experiments/validation-spike/cases/*.smt2
experiments/validation-spike/scripts/*
experiments/validation-spike/results/toolchain.txt
experiments/validation-spike/results/summary.md
```

Small textual proof/output fixtures may be committed when they materially support reproducibility. Avoid checking in huge generated certificates without need; scripts should regenerate them.

Update `docs/EXPERIMENTS-V1.md` only if a small status section is already appropriate and the update clearly labels Stage 0 as executed/failed/passed. Do not rewrite frozen theory.

## REQUIRED TESTING OF THE HARNESS

Before declaring completion:

- run the top-level script from a clean working directory state as far as practical;
- verify expected SAT/UNSAT classification;
- verify at least one accepted proof is actually rejected after proof mutation;
- verify at least one accepted proof is rejected or safely detected as misbound after problem mutation;
- verify no selected pipeline is marked PASS merely because cvc5 printed `unsat`;
- verify missing checker/tool produces clear failure rather than false PASS.

## FAILURE HANDLING

If cvc5, Ethos or Carcara behavior differs from expectations:

1. preserve the failing fixture/output;
2. inspect primary documentation/source for the pinned version;
3. classify the failure;
4. do not paper over it by weakening the Stage-0 gate;
5. report whether another pipeline remains viable.

If both independent pipelines fail, stop Stage 0 successfully with the fallback recommendation. Do not proceed to Stage 1 unless explicitly authorized by a later review.

## SCOPE CONTROL

Forbidden scope creep:

```text
no Rust SPL workspace bootstrap merely for appearances
no SPL syntax design
no Plan VM
no CBOR implementation
no backend design beyond what the frozen plan states
no planner implementation
no admission kernel
no FPGA/native code work
no theorem-prover implementation of our own
```

A few helper scripts are enough. Python/shell/Rust may be used for the harness when justified, but do not create a library architecture for a spike.

## QUALITY BAR

The spike must be:

```text
reproducible
adversarial
small
auditable
honest about trust
honest about unsupported proof rules
honest about failures
```

Do not use words such as `verified`, `proof-carrying`, or `trusted-independent` in conclusions unless the actual checker path supports the claim made.

## REQUIRED FINAL REPORT

Your final response must contain these sections:

### 1. Status

One of:

```text
STAGE 0 PASS — CPC/ETHOS SELECTED
STAGE 0 PASS — ALETHE/CARCARA SELECTED
STAGE 0 PASS — INDEPENDENT PIPELINES FAILED; SOLVER-IN-TCB FALLBACK REQUIRED
STAGE 0 BLOCKED — ENVIRONMENT/TOOLING ISSUE
```

Use `BLOCKED` only when you could not actually complete the experiment, not merely when a proof pipeline failed its gate.

### 2. Repository changes

List every created/modified file and why.

### 3. Toolchain

Exact versions/SHAs and installation/build method.

### 4. Cases

For every case:

```text
semantic purpose
expected SAT/UNSAT
actual result
```

### 5. CPC/Ethos result

Report:

```text
proof production
Ethos result
trust-step status
exact-input-binding result
mutation result
PASS/FAIL classification
```

### 6. Alethe/Carcara result

Report:

```text
proof production
Carcara result
hole/trust status
exact-input-binding result
mutation result
PASS/FAIL classification
```

### 7. Selected validation mechanism

State the winner or solver-in-TCB fallback and justify it using the frozen priority order.

### 8. TCB consequence

State precisely what enters the future Runtime Semantic TCB because of the Stage-0 result.

Examples:

```text
Ethos + CPC/Eunoia signature + proof parser
```

or:

```text
Carcara + its Alethe rule implementation + proof parser
```

or:

```text
cvc5 solver itself, because no independent checker path passed
```

Do not overstate formal verification of any checker.

### 9. Remaining risks

Only real risks exposed by Stage 0.

### 10. Commands run

Provide the important reproducibility commands or point to committed scripts.

### 11. Git state

Report:

```text
branch
commit SHA(s)
working tree status
```

Do not merge the branch yourself unless explicitly authorized.

## DEFINITION OF DONE

Stage 0 is complete only when all of these are true:

```text
[ ] representative QF_BV fixtures exist
[ ] SAT/UNSAT expectations are independently sanity-checked
[ ] CPC/Ethos attempted
[ ] Alethe/Carcara attempted
[ ] tool versions pinned/recorded
[ ] proof trust/hole behavior recorded
[ ] at least one proof mutation attack performed
[ ] at least one problem-misbinding attack performed
[ ] exact-input binding classified for each candidate pipeline
[ ] result matrix generated
[ ] winner or explicit fallback selected
[ ] experiment rerunnable through committed script(s)
[ ] no Stage-1 implementation was started
[ ] final report distinguishes fact from inference/proposal
```

When these conditions are met, stop.

Do **not** implement Stage 1. Return the evidence for review.
