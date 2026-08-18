# Stage 0 — Validation Feasibility Spike

**Status:** `STAGE 0 PASS — CPC/ETHOS SELECTED`
**Scope:** [`../../docs/SPL-0-IMPLEMENTATION-PLAN-V1.md`](../../docs/SPL-0-IMPLEMENTATION-PLAN-V1.md), Stage 0 only
**Machine-generated evidence:** [`results/summary.md`](results/summary.md), [`results/matrix.tsv`](results/matrix.tsv), [`results/matrix.json`](results/matrix.json), [`results/toolchain.txt`](results/toolchain.txt)

This directory answers one question and nothing else:

> Can representative SPL-0 `QF_BV` obligations produce reproducible evidence that is bound to the exact logical problem and independently checked, without silently trusting cvc5 as the semantic authority?

The answer, on the pinned toolchain recorded below, is **yes, through `cvc5 -> CPC -> Ethos`, and only when the proof carries an explicit `(reference "<problem>.smt2")` command**. `cvc5 -> Alethe -> Carcara` fails the Stage-0 assurance rule on bit-vector obligations.

Nothing here is SPL. There is no frontend, no `K`, no Plan IR, no Plan VM, no Admission Kernel, no artifact layer and no planner. The fixtures are hand-written SMT-LIB prototypes of future obligations, not output of `DeriveObligation`.

## Reproducing

```bash
./scripts/setup-tools.sh          # fetch/build the pinned toolchain into .tools/ (git-ignored)
pip install z3-solver             # optional: independent sat/unsat cross-check
./scripts/run-all.sh              # run the whole matrix, regenerate results/
```

`run-all.sh` exits `0` when the harness ran to completion, whatever the scientific verdicts were, and `1` only when the harness itself malfunctioned — a tool crashed unexpectedly, a fixture stopped classifying as it declares, or a mutation stopped being semantically meaningful. **A pipeline failing its Stage-0 gate is not a harness malfunction and does not change the exit status.**

## Toolchain

| component | pin | how |
| --- | --- | --- |
| cvc5 | `cvc5-1.3.4` (`git f3b21c4`) | official static release asset `cvc5-Linux-x86_64-static.zip` |
| CPC signature | `cvc5-1.3.4`, `proofs/eo/cpc/Cpc.eo` | sparse checkout of the matching cvc5 source tag |
| Ethos | commit `221641668d75eaffd308e0511d63962cea937110` | built from source; this is the commit **cvc5 1.3.4 itself pins** in `contrib/get-ethos-checker` |
| Carcara | commit `6624ea80cf1985ada473c0705869c78353e4282d` (`carcara 1.1.0`) | built from source; the project publishes no tagged release |
| z3 | `5.1.0` (pip `z3-solver`) | independent second solver, **cross-check only**, not on any evidence path |

Exact host, compiler and interpreter versions are written to [`results/toolchain.txt`](results/toolchain.txt) by every run. Nothing is vendored into the repository: `.tools/` is git-ignored and rebuilt by `scripts/setup-tools.sh`.

Ethos has no `--version` flag, so its identity is the pinned commit plus the SHA-256 of the built binary, both recorded in `results/toolchain.txt`.

## Primary sources consulted

Tool behaviour was taken from the pinned versions themselves, not from prior project prose:

- `cvc5-1.3.4:docs/proofs/output_cpc.rst` — CPC is cvc5's native proof format; Ethos returns `incomplete` if any trust step is used and `correct` otherwise; a trust step "proves an arbitrary formula with no provided justification"; CPC proofs are closed refutations that assume formulas from the input.
- `cvc5-1.3.4:contrib/get-ethos-checker` — the Ethos commit this cvc5 release is built against.
- `cvc5-1.3.4:test/regress/cli/run_regression.py` — cvc5's own Ethos harness: prepend `(include ".../cpc/Cpc.eo")`, strip the leading `unsat\n(\n` and trailing `\n)\n`, then run `ethos <file>` and require `correct`/`incomplete` in the output. `scripts/run-cpc-ethos.sh` follows the same procedure.
- `cvc5-1.3.4:src/options/{proof_options,smt_options,base_options}.toml` — the real option surface: `--proof-granularity` modes, `--proof-allow-trust` (default `true`), `--safe-mode`, `--proof-mode=full-proof-strict`.
- `ethos@2216416:user_manual.md` §"Validation Proofs via Reference Inputs" — `(reference "file.smt2")` makes Ethos check that every proof assumption occurs among the referenced file's assertions.
- `carcara@6624ea8:docs/src/checking/rare.md` and `carcara check --help` — `carcara check PROOF PROBLEM`, the `hole` handling, and the `--rare-file` requirement.

## Fixtures

Ten hand-written `QF_BV` fixtures, five expected `unsat` (the shapes that must carry proof evidence) and five expected `sat` (negative validation examples, which must **not** carry proof evidence).

| case | semantic purpose | expected |
| --- | --- | --- |
| `01_wrapping_equiv_valid` | two structurally different u8 realizations of `3*x + y` | unsat |
| `02_checked_intermediate_overflow_invalid` | a candidate cancels `+MAX` then `-MAX` and loses the intermediate `SemanticError(Overflow)` | sat |
| `03_guarded_specialization_valid` | dropping the overflow check of `checked_add(x, 100)` is sound only under `x <u 156` | unsat |
| `04_guard_widening_invalid` | the same specialization with the guard widened past the overflow boundary | sat |
| `05_outcome_tag_mismatch_invalid` | equal payloads, different semantic tags | sat |
| `06_outcome_encoding_non_collapse_valid` | the outcome encoding cannot conflate `Return(v)` with `SemanticError(Overflow)`, for any `v` | unsat |
| `07_signed_overflow_rule_boundary_valid` | the signed-overflow rule used by fixture 02 is proved equal to the classical sign-case rule | unsat |
| `08_wrapping_modeled_as_checked_invalid` | modelling `checked_add` as plain `bvadd` is unsound | sat |
| `09_width_confused_overflow_check_invalid` | a realization that computes in u32 checks overflow at the wrong width | sat |
| `10_signed_shift_specialization_valid` | `bvsdiv x 2` = `bvashr x 1` only under `0 <=s x <s 64` | unsat |

Every expectation is cross-checked with z3, an independently implemented solver. All twenty answers agree; see the case-classification table in [`results/summary.md`](results/summary.md).

### Outcome encoding

An SPL-0 outcome is encoded, for Stage-0 purposes only, as a pair of bit-vectors:

```text
tag     : (_ BitVec 1)   #b0 = Return, #b1 = SemanticError(Overflow)
payload : (_ BitVec 8)   the returned value when tag = #b0
                         canonically #x00 when tag = #b1
```

Because the payload is canonical under the error tag, outcome equality is componentwise equality of the pair. Fixture 05 shows that comparing payloads alone would conflate `Return(0)` with `SemanticError(Overflow)`; fixture 06 *proves* that the pair encoding cannot conflate them for any payload. This is a Stage-0 encoding, not a commitment for the future `ObservedOutcome` domain.

### Checked arithmetic and its overflow rule

Checked operations are never modelled as bare wrapping `bvadd`. Overflow of `a op b` at width 8 is defined by width extension:

```text
unsigned:  zext16(a op8 b) != zext16(a) op16 zext16(b)
signed:    sext16(a op8 b) != sext16(a) op16 sext16(b)
```

Fixture 07 discharges this rule against the classical sign-case rule `(a>=0 and b>=0 and a+b<0) or (a<0 and b<0 and a+b>=0)` over **all** width-8 inputs, so the boundary inputs (`1 + 127`, `-1 + -128`) are covered by proof rather than by spot checks. The fixtures do not depend on `bvsaddo`-style built-ins whose proof support differs between formats.

Fixtures avoid `define-fun` and `let` entirely, using declared intermediates with defining assertions instead. This keeps what the checker parses from the problem file textually identical to what the solver saw, so the binding experiments below measure binding and not parser-feature drift.

## Result — Pipeline A: `cvc5 -> CPC -> Ethos`

```text
PASS
```

All five expected-unsat fixtures produce a CPC proof that Ethos reports as `correct`, with **zero trust steps** in every proof. `grep -c ':rule trust'` and Ethos' own `correct` vs `incomplete` distinction agree on all five, so the trust-freedom claim rests on the checker's semantic completeness result and not only on a text search.

Three facts matter more than the `correct`:

**1. Binding is real, but only through the in-proof `(reference ...)` command.** Ethos does not otherwise consume the original SMT-LIB problem: a CPC proof re-declares its own constants and assumes its own formulas, so a standalone proof is a closed refutation of *whatever it assumed*. With `cvc5 --proof-print-reference` and `(reference "<problem>.smt2")` prepended to the proof, Ethos requires every assumption to occur among the problem's assertions. All eight problem-misbinding attacks are then rejected.

**2. Ethos' documented `--reference=X` command-line option silently does not enforce that check** on the pinned commit. All eight misbinding attacks that the in-proof command rejects are reported `correct` when the same reference file is supplied through the flag. The cause is visible in the source: `State::includeFile` assigns `d_hasReference = isReference` (`src/state.cpp:310`), and the main proof file is subsequently loaded through the two-argument `includeFile` overload (`src/main.cpp:185`), which passes `isReference = false` and clears the flag set by the flag-supplied reference. An SPL integration that used the documented flag would get a silent false ACCEPT on every misbound proof. **The Runtime Semantic TCB must emit the `(reference ...)` command and must keep a negative control that fails if the mechanism ever stops rejecting.**

**3. Ethos signals rejection by aborting, not by a non-zero-with-message convention.** On a binding failure it exits `134` with empty stdout; on a well-formed but incomplete proof it exits `0` and prints `incomplete`. Acceptance must therefore be `exit == 0 && stdout == "correct"`. Treating exit `0` as acceptance would admit every incomplete proof.

Proof mutation: both mutation kinds (premise swap, bit-vector literal flip) are rejected on every case where a mutation site exists.

## Result — Pipeline B: `cvc5 -> Alethe -> Carcara`

```text
FAIL — TRUST/Hole
```

At cvc5's default granularity, the Alethe output for these bit-vector obligations contains `:rule hole :args ("untranslated rewrite")` steps — 1 to 82 per fixture. A `hole` step asserts its own conclusion. Carcara accepts such a proof, prints `holey`, **and exits `0`**. That is the most dangerous single observation in this spike: a harness that checked the exit status would report success on a proof containing 82 unchecked steps.

Raising granularity does not rescue the pipeline. At `--proof-granularity=dsl-rewrite` the holes do disappear, but two other walls appear. cvc5 then prints cvc5-internal terms into the Alethe file — `(zero_extend 8 x)` and `(extract 6 0 x)` rather than the SMT-LIB `((_ zero_extend 8) x)` and `((_ extract 6 0) x)` — which Carcara cannot parse. And the holes are replaced by up to 40 `rare_rewrite` steps per proof, for which no compatible RARE database exists for this version pair: `cvc5 -o rare-db` emits `declare-rule` in the Eunoia/CPC dialect (`/_total`, `0/1`, `@bv`), while Carcara expects `declare-rare-rule` in SMT-LIB syntax. Renaming the command is not enough; Carcara stops at the first Eunoia term.

The exact per-fixture outcome, recorded in `results/matrix.json`:

| case | default granularity | `dsl-rewrite` |
| --- | --- | --- |
| `01` | `holey`, 4 holes | `invalid` — RARE rule `eq-refl` not found |
| `03` | `invalid` — RARE rule `ite-eq` not found (82 holes) | `invalid` — parse error, `zero_extend` not defined |
| `06` | `holey`, 1 hole | `valid` |
| `07` | `invalid` — Alethe rule `bv_bitblast_step_var` unknown (77 holes) | `invalid` — same unknown rule |
| `10` | `invalid` — Alethe rule `bv_bitblast_step_var` unknown (46 holes) | `invalid` — parse error, `extract` not defined |

Net result over the five expected-unsat fixtures: one accepted (`06`, a pure tag/Boolean obligation, `valid` at `dsl-rewrite`), two `holey`, three rejected. No fixture involving bit-vector arithmetic is fully checked, in either granularity.

Carcara's **binding is not the problem**. It takes the original problem as an argument by design and rejects every one of the eight misbinding attacks, with an explicit `could not match term to any of the original problem premises` when the assumption is absent. If the hole and RARE-coverage situation changes in a later version, Pipeline B becomes a serious candidate again on this axis.

## cvc5 safety settings, as actually measured

- `--safe-mode=safe` **rejects** `--proof-format-mode` — it is an expert option. This is not a blocker: the error message itself reveals that CPC is already the default format (`The value for proof-format-mode is already its current value (cpc)`), and `cvc5 --safe-mode=safe --dump-proofs` produces the CPC proof with no expert option at all. Safe mode is therefore compatible with Pipeline A and incompatible with Pipeline B.
- `--proof-mode=full-proof-strict` exists in 1.3.4 and is documented as disabling techniques that lead to incomplete proofs.
- `--proof-allow-trust` exists and **defaults to `true`**. It is a boolean, so the negation is `--no-proof-allow-trust`; `--proof-allow-trust=false` is a parse error. On these fixtures the CPC proofs contain no trust step either way, so the option changed nothing measurable here. It must not be assumed to be a completeness guarantee.
- Proof granularity behaves differently per format. With `--proof-format-mode=cpc` and no explicit granularity, cvc5 produces trust-free proofs; forcing `--proof-granularity=macro` produces 3 trust steps and `ethos` then reports `incomplete`. The harness therefore records granularity explicitly rather than relying on defaults.
- `--check-proofs-complete` exists and is the cvc5-side counterpart of this policy; it is not a substitute for the independent checker's own verdict.

## Selected validation mechanism

```text
cvc5 -> CPC -> Ethos, with an in-proof (reference "<problem>.smt2") command
```

Justified against the frozen priority order:

1. **no unchecked/trusted/hole step** — Pipeline A: 0 trust steps on 5/5. Pipeline B: holes on 3/5 and unknown rules on 3/5. Decisive.
2. **exact proof/problem binding** — both can bind. Pipeline A only through the in-proof command; Pipeline B by default. Tie, with a documented trap on A.
3. **reproducible pinned checker** — both build from a pinned commit; Ethos' commit is pinned by cvc5 itself, Carcara has no release.
4. **smaller/auditable checker TCB** — Ethos is a Eunoia interpreter plus the `Cpc.eo` signature; its rule semantics live in a data file that can be diffed. Carcara implements Alethe rules in Rust and additionally needs a RARE database.
5. **wider Core-A coverage** — Pipeline A covers all ten fixtures' shapes; Pipeline B covers one.
6. **proof size / checking cost** — not decisive at this scale; all proofs check in well under a second.

## TCB consequence

Selecting Pipeline A puts the following into the future **Runtime Semantic TCB**:

```text
Ethos (the pinned binary)
the CPC/Eunoia signature Cpc.eo of the matching cvc5 version
Ethos' Eunoia parser and its proof/reference parser
the SMT-LIB parsing Ethos applies to the referenced problem file
the SPL-side component that emits the (reference ...) command
    and that fails closed if the reference mechanism is not active
the SPL-side rule that accepts only exit 0 with stdout exactly "correct"
```

cvc5 itself stays **outside** the Runtime Semantic TCB under this selection: it is an untrusted proof producer whose output is independently checked and bound to the problem. That is a claim about this pipeline as measured, not a verification claim about Ethos. Ethos is not formally verified, and this spike did not verify it.

The fallback of putting the solver explicitly inside the Runtime Semantic TCB is **not** required by the Stage-0 data, and is not adopted.

## Remaining risks

1. **Binding depends on one command and on one file being the same file.** Ethos binds the proof to the *reference file it parsed*, not to a content identity of a canonical artifact. Stage 0 has no artifact layer, so `K_ref`/`P_ref` hashing was deliberately not faked. When the artifact layer exists, the binder must tie the referenced bytes to the admitted `K`/`P` identity, or the misbinding attack simply moves one level up.
2. **The `--reference=` flag trap is a live false-accept.** It is a documented option that does not do what its documentation implies on the pinned commit. Any future refactor that switches to the flag reintroduces the vulnerability silently. The harness keeps the negative control precisely for this.
3. **`correct` is a statement about the signature, not about SMT-LIB.** The CPC calculus deliberately deviates from SMT-LIB where cvc5's internals do (cvc5's own documentation says so). Whether `Cpc.eo` faithfully expresses the semantics SPL intends for `QF_BV` is unaudited, and is a real common-mode risk shared with the encoder.
4. **Trust-freedom was measured on ten small fixtures.** Nothing here shows that the obligations `DeriveObligation` will actually emit stay trust-free. Larger or differently-shaped obligations may reintroduce trust steps, and the gate must be re-run per obligation shape, not assumed.
5. **Ethos aborts instead of returning a classified status.** A crash and a rejection are hard to tell apart from the exit status alone. The future admission path must distinguish "checker said no" from "checker died", and must fail closed on both.
6. **Pipeline B may recover.** The hole and RARE-coverage findings are properties of this version pair, not of Alethe. Re-running this spike is the way to find out, which is why the harness checks both pipelines rather than only the winner.

## Layout

```text
experiments/validation-spike/
├── README.md                  this report
├── cases/*.smt2               ten QF_BV fixtures, each declaring its expected result
├── cases/mutations.tsv        the problem-misbinding mutations
├── scripts/env.sh             pinned paths and loud missing-tool failures
├── scripts/setup-tools.sh     fetch/build the pinned toolchain into .tools/
├── scripts/run-cpc-ethos.sh   pipeline A, one case, key=value output
├── scripts/run-alethe-carcara.sh  pipeline B, one case, key=value output
├── scripts/mutate-binding.sh  emit a logically mutated problem
├── scripts/mutate-proof.py    controlled textual proof mutation
├── scripts/run_matrix.py      the driver: matrix, attacks, self-tests, reports
├── scripts/run-all.sh         top-level entry point
└── results/                   toolchain.txt, matrix.tsv, matrix.json, summary.md,
                               raw/ (sat witnesses; raw/work/ is git-ignored)
```

## What Stage 0 did not do

No `spl0-t0` tag, no Backend B, no Stage-1 work, and no claim that any interface is frozen beyond these experimental artifacts. The words *verified*, *proof-carrying* and *trusted-independent* are used in this report only where the measured checker path supports them.
