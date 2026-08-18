# SPL-0 Artifact Encoding — decision record

**Status:** decided at Stage 1, for Stage-1 artifacts
**Scope:** `KArtifact`, `PlanArtifact`, `QArtifact`
**Plan dependency:** [`SPL-0-IMPLEMENTATION-PLAN-V1.md`](./SPL-0-IMPLEMENTATION-PLAN-V1.md) §9

The implementation plan names deterministic CBOR as the leading candidate and deliberately does not freeze a Rust library, with an explicit warning: *do not assume `serde + CBOR = canonical`*. This file records what was checked, what was chosen, and what the choice does not give us.

## What was measured

Two candidate crates were checked against their own documentation and then against their actual behaviour.

| question | `ciborium` 0.2.2 | `minicbor` 2.3.0 |
| --- | --- | --- |
| documents a canonical/deterministic guarantee | no | no |
| encoder emits shortest-form integer heads | yes | yes |
| encoder emits definite-length arrays | yes | yes |
| decoder accepts a non-shortest integer head (`0x18 0x05` for 5) | yes | yes |
| decoder accepts indefinite-length arrays | yes | n/a for the profile |
| decoder accepts trailing bytes after a complete item | yes | yes |
| `unsafe` in the crate's own sources | none | none |

Neither crate documents the property the plan needs. Both encoders are in practice deterministic for the item kinds we use; both decoders are permissive in exactly the ways that would let two different byte strings denote one artifact.

The conclusion is not "pick the better crate". It is that **the library provides the wire format and the SPL layer provides canonicality**.

## Choice

`ciborium` 0.2.2, used through `ciborium::value::Value` rather than through `serde` derive.

Reasons, in order:

1. Going through the dynamic `Value` tree means the artifact shape is written out explicitly in `spl-core::artifact` and `spl-plan::artifact` instead of being whatever a derive macro decided. For code intended to end up in a semantic TCB, the encoding being readable in one place matters more than the convenience of `#[derive]`.
2. `ciborium` carries no `unsafe` in its own sources.
3. It rejects deeply nested input rather than exhausting the host stack — measured at nesting depth 1 000 and above, and pinned by the `nesting_is_bounded` test.

`minicbor` would also have been serviceable. Nothing in this record depends on the choice between them, because the canonicality rule below does the actual work.

## The SPL-0 canonical profile

1. Every artifact is a definite-length CBOR **array**. Fields are positional. Maps are forbidden outright, which removes the map-key-ordering question instead of answering it.
2. Permitted item kinds: unsigned integer, negative integer, byte string, text string, boolean, array. Rejected on both encode and decode: floats, tags, `null`, `undefined`, maps, indefinite-length items, and anything the library's `#[non_exhaustive]` value type grows later.
3. Integers are shortest-form.
4. Untrusted input is bounded: at most 64 KiB, with nesting bounded by the decoder.

Every artifact is wrapped in an envelope:

```text
[ schema_tag, schema_version, body ]
```

so a `K` artifact cannot decode as a `P` artifact even if the bodies were byte-identical. The tags are `1 = K`, `2 = P`, `3 = Q`, and they are visible in the first bytes: a contract starts `83 01 01 …`, a plan starts `83 02 01 …`.

## Canonical decode rule

Conformance is not established by trusting the library. It is established by round-tripping:

```text
decode(bytes)   -> value
encode(value)   -> canonical_bytes
bytes == canonical_bytes        required
```

Bytes that decode but are not canonical are rejected as an artifact. One rule covers all three permissive behaviours measured above: a non-shortest integer, an indefinite-length array and a trailing byte each re-encode to something different from the input. Each of those three is a test.

## Content identity

```text
ContentId = SHA-256( exact canonical artifact bytes )
textual form: sha256:<64 lowercase hex>
```

The digest is taken over bytes, never over a value — computing it from a value would make the identity name the encoder's current behaviour rather than the bytes that were actually stored.

Identity is over exact content and is **not** a semantic equivalence class. Two contracts that compute the same function have different identities if they are written differently, and the test suite asserts exactly that. The frozen theory says the same thing about `K_ref`.

Stage-1 artifact sizes, for scale:

```text
K   checked_roundtrip_u8      51 bytes
P   p_reference_checked       77 bytes
P   p_specialized_compare     69 bytes
P   p_bad_identity            32 bytes
Q   stage1_full_core_a        19 bytes
```

## Limitations

These are real and are not worked around anywhere in the code.

1. **The canonicality guarantee is ours, not the library's.** If a future `ciborium` version stops emitting shortest-form heads or definite-length arrays, every previously stored artifact starts failing the round-trip check. That is a loud failure rather than a silent divergence, and `canonical::assert_shortest_form_integers` localizes it, but the dependency is real. `Cargo.lock` is committed so the version does not move on its own.
2. **The dependency closure is not `unsafe`-free.** Both Stage-1 crates carry `#![forbid(unsafe_code)]`, and `ciborium` itself has none, but `half` (pulled in by `ciborium` for a float type the profile never encodes) and `sha2` do contain `unsafe`. A future TCB audit has to account for that; Stage 1 only records it.
3. **The profile is not RFC 8949 §4.2 "deterministically encoded CBOR" in general.** It is a strict subset that avoids the parts of that specification we would otherwise have to implement and test — principally map key ordering. Any later artifact that genuinely needs a map has to extend this record first, not improvise.
4. **`SHA-256` is a choice, not a requirement derived from anything.** It is recorded in the identity's textual form so that changing it is visible in every stored identity rather than silent.
5. **The 64 KiB bound is a Stage-1 number.** Stage-1 artifacts are tens of bytes. A later stage with larger artifacts must raise it deliberately and say why.
