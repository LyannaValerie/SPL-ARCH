# Stage 1 — Semantic Vertical Slice

**Status:** `STAGE 1 COMPLETE`
**Scope:** [`SPL-0-IMPLEMENTATION-PLAN-V1.md`](./SPL-0-IMPLEMENTATION-PLAN-V1.md), Stage 1 only
**Code:** [`crates/spl-core`](../crates/spl-core), [`crates/spl-plan`](../crates/spl-plan)

Stage 1 answers one question:

> Can the same computation be represented as a semantic contract `K` and as operationally different plans `P`, executed independently, and can we concretely exhibit a valid realization, a valid specialization and a semantically wrong realization — with deterministic, identifiable artifacts — before any trust mechanism exists?

Yes. What follows is what was built and what it does and does not show.

**Stage 1 contains no trust.** There is no verification condition, no evidence, no assurance policy, no admission record, no activation gate, no guard evaluation, no planner and no SMT of any kind. Nothing in this slice authorizes a realization. The disagreement of the wrong plan is *observed by a test*, not acted on by the system.

## The two representations

### `K` — semantic contract (`spl-core`)

A typed semantic tree in two levels, because Core A distinguishes values from outcomes:

* `KExpr` denotes a value. Evaluating it may **raise** a semantic error, which propagates outward. That is the whole of checked arithmetic in `K`: there is no overflow flag anywhere in the contract, and nothing branches on one.
* `KTerm` denotes a `SemanticOutcome` — `Return(e)`, `SemanticError(kind)`, or an outcome-level conditional.

`KContract` adds the signature. `type_check` is total and syntax-directed and rejects: unknown inputs, non-`Bool` conditions, disagreeing conditional branches, mismatched operands, operations undefined for a type (arithmetic or ordering on `Bool`), and a body whose type is not the declared result.

The evaluator is a direct recursive interpreter over that tree and consumes nothing else. It returns `SemanticOutcome`; the outer `Result` carries only refusals to run — wrong input arity or type — so a semantic error is never confused with a caller mistake.

### `P` — Plan IR (`spl-plan`)

A register machine: basic blocks over single-assignment typed registers, explicit branches, explicit terminators. Block 0 is the entry.

The instruction that carries the design is `Checked`:

```text
(value, overflow) = checked_add lhs, rhs
```

Two results. The overflow is an ordinary `Bool` register and the plan must branch on it itself. A plan that ignores the flag is structurally valid and semantically wrong — which is the difference between a contract and a realization, made concrete.

`Terminator::Error { kind }` ends an execution with a semantic error, so raising is a control-flow act in `P` and a propagation in `K`.

### Why they stay distinct

| | `K` | `P` |
| --- | --- | --- |
| shape | expression tree | basic blocks over registers |
| overflow | raised and propagated implicitly | a `Bool` register the plan must branch on |
| control flow | nested conditionals | explicit branch/jump terminators, acyclic |
| Rust types | `KExpr`, `KTerm`, `KContract` | `Instruction`, `Terminator`, `Block`, `Plan` |
| executed by | `spl_core::eval` | `spl_plan::vm::PlanVm` |
| artifact tag | `1` | `2` |

`spl-plan` depends on `spl-core` for the Core A value and outcome domains, the primitive semantics and the artifact encoding — shared vocabulary — and for nothing else. Two tests keep that honest by scanning the crates' own sources (with comments stripped) and failing if `spl-plan` ever names `KExpr`, `KTerm`, `KContract`, `spl_core::k`, `spl_core::eval` or `spl_core::demo`, or if `spl-core` ever names the plan representation.

What *is* shared, deliberately, is the primitive layer: one definition of what `u8` checked addition means. The plan calls this common-mode semantic risk and it is real — a bug there is a bug in the contract and in every realization at once. Two independent primitive implementations would only move the problem, because the conformance suite would then be comparing one implementation under test against another, which the plan forbids as an oracle. The mitigation is an oracle that shares no code with the implementation: the conformance tests compute expectations in `i128` and reduce into range explicitly.

## Structural validation

`validate(plan) -> ValidatedPlan` is about form only. `ValidatedPlan` is constructible only by the validator and is the only thing the VM accepts, so "the VM never runs an unvalidated plan" is enforced by the type system rather than by convention.

It rejects: missing entry block, out-of-range block targets, unreachable blocks, cyclic control flow, registers assigned twice, registers read where they are not defined on *every* path, registers beyond the addressable range, unknown input indices, mismatched operands, operations undefined for a type, a branch condition that is not `Bool`, and a returned register whose type is not the declared result.

Availability is a dataflow pass: registers are single-assignment and the graph is acyclic, so the set available on entry to a block is the intersection over its predecessors and one pass in topological order suffices. The typing pass also walks blocks in topological order, so validation does not depend on the order blocks happen to be stored in — there is a test for that.

## The Plan VM

Interprets the Plan IR directly and returns `ObservedOutcome`. It never builds a `K` expression, never calls the `K` evaluator, and has no notion of a correct answer to compare against.

Termination is structural: cyclic control flow is rejected, so a validated plan always reaches a `Return` or an `Error`, and `Diverge` is unreachable for Core A. A step limit exists as defence in depth and is expressible as `Trap(StepLimitExceeded)`; a test shows a starved configuration reaching it, and another test shows the demo never does. `Stuck` is reachable only from defensive paths a validated plan cannot take.

## `S` and `Q`

`Q = PlanVmConfiguration` binds the facts a plan actually depends on: Plan IR revision, primitive-set revision, the supported types, the arithmetic modes, and the step limit. The VM refuses to run a plan needing something `Q` does not offer — and the refusal is a `VmError`, never an `ObservedOutcome`, because a substrate that cannot host a plan has not executed it.

This is exercised, not decorative: a configuration without checked arithmetic refuses `p_reference` and still hosts `p_specialized`, because `Q` restricts realizations and not the contract.

`S = SubstrateIdentity` is an identity for one prototype substrate. **No architectural adaptation is claimed, exercised or simulated.** Nothing reconfigures anything.

## Artifacts

`KArtifact`, `PlanArtifact` and `QArtifact` all encode as `[schema_tag, schema_version, body]` in a strict canonical CBOR profile, with identity `sha256:<hex>` over the exact canonical bytes. The library evaluation, the profile, the canonical decode rule and the limitations are recorded separately in [`ARTIFACT-ENCODING-V1.md`](./ARTIFACT-ENCODING-V1.md).

The result that matters here: bytes that decode but are not canonical are rejected, so one artifact has one byte string and one identity.

## The experimental program

```text
K(x):  checked_sub(checked_add(x, 255), 255)      over u8
```

Its meaning, stated independently of the code:

```text
x == 0  ->  Return(0)
x >  0  ->  SemanticError(Overflow)
```

That statement is the oracle. The tests do not obtain it by running `K` or any plan.

Three hand-written realizations:

* `p_reference` — computes, inspects the overflow flag, branches. Four blocks, six registers, two checked instructions, no comparison.
* `p_specialized` — never does the arithmetic; decides from `x > 0`. Three blocks, four registers, no checked instruction, one comparison.
* `p_bad` — `Return(x)`. One block, well typed, executable, and wrong.

## Results

| check | result |
| --- | --- |
| `K` matches the independent oracle | all 256 inputs |
| `Run(p_reference, x) == embed(K(x))` | all 256 inputs |
| `Run(p_specialized, x) == embed(K(x))` | all 256 inputs |
| `p_bad` structurally valid | yes |
| `p_bad` disagrees with `K` | 255 of 256 inputs; agrees only at `x = 0` |
| any demo run traps, gets stuck or diverges | never |
| `u8` primitive conformance | exhaustive, 65 536 operand pairs per operation and mode |
| `u32` / `i32` boundary conformance | full boundary corpus, all ordered pairs |
| `K` and `P` artifact bytes | differ; neither decodes as the other |

At `x = 1` the contract raises `SemanticError(Overflow)` and `p_bad` returns `Return(1)`. Nothing structural distinguishes it from the two correct plans — which is the point of the whole slice.

### Mutation sanity checks

Deliberate mutations, with their expected effect:

| mutation | detected |
| --- | --- |
| contract: checked becomes wrapping | yes — disagrees at every `x > 0` |
| realization: both operations become wrapping, no flag inspected | yes — disagrees at every `x > 0` |
| realization: guard `x > 0` becomes `x >= 0` | yes — disagrees exactly at `x = 0` |
| realization: the error arm returns `Return(0)` instead of raising | yes — same payload, different tag |
| realization: the *first* overflow check removed | **no, and correctly so** |

The last row is kept on purpose. Removing the first check changes the plan's structure but not its meaning: for every `x` above zero the wrapped sum is `x - 1`, and subtracting 255 from a value below 255 underflows, so the second check raises exactly where the first would have. It is an equivalent mutant, and it is recorded so that "the mutation survived" is not read as "the tests are weak".

The outcome-tag mutation is worth its own note: the mutant returns `Return(0)` where the contract raises. The payload `0` is exactly what the contract returns at `x = 0`, so only the *tag* distinguishes the outcomes. A comparison over payloads would have missed it.

## What Stage 1 does not show

1. **Nothing is verified.** These are tests. The `u8` demo is exhaustive over its 256 inputs, which is a complete check *of that contract at that width* — it is not a proof about `K`, the Plan VM, or any other program. The `u32`/`i32` results are boundary samples, not exhaustive.
2. **Correctness of the shared primitive layer is assumed, not established.** The `i128` oracle is independent of the implementation, but the Core A semantics themselves are the thing being defined here, not derived from anywhere.
3. **`P` is not a realistic backend.** It is a small interpreted IR with no loops, no memory, no effects and no machine below it. It exercises the K/P distinction, not any performance or hardware claim.
4. **`S`/`Q` binding is exercised at the smallest possible scale.** A configuration refusing a plan is real; it is not evidence about architectural adaptation.
5. **The equivalence demonstrated is by execution over an enumerable input domain.** Stage 2 is where an obligation is derived symbolically and evidence is checked; nothing in Stage 1 anticipates that, and no VC, encoder or admission structure was created for it.

## Definition of Done

```text
[x] Rust workspace builds
[x] no unsafe code in Stage-1 crates
[x] Core-A K is representable and executable
[x] Plan IR is representable and structurally validated
[x] Plan VM executes independently from K evaluator
[x] P_reference and P_specialized are structurally different
[x] both agree with K for all 256 inputs of the u8 demo
[x] P_bad is well-typed and executable
[x] P_bad disagrees with K for at least one input
[x] u8 primitive conformance/exhaustive tests pass
[x] i32/u32 boundary tests pass
[x] minimal S/Q exist
[x] K/P/Q artifacts serialize deterministically
[x] non-canonical artifact bytes are rejected
[x] exact artifact content IDs are stable
[x] K and P artifact encodings differ
[x] cargo test --workspace passes
[x] cargo clippy --workspace --all-targets -- -D warnings passes
[x] cargo fmt --all -- --check passes
```

No Stage-2 work was performed. No `spl0-t0` tag, no Backend B, no planner, no admission kernel, no frontend, no SMT.
