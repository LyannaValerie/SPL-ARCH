#!/usr/bin/env bash
# Pipeline A: cvc5 -> CPC -> Ethos
#
# usage: run-cpc-ethos.sh --problem P.smt2 --reference R.smt2 --work DIR
#                         [--mode reference|standalone] [--proof EXISTING.cpc]
#
#   --problem     SMT-LIB problem the proof is generated FROM
#   --reference   SMT-LIB problem the proof is CHECKED AGAINST
#                 (equal to --problem for the positive case; a mutated problem
#                  for the misbinding tests)
#   --mode        reference  : emit (reference "<R>") so Ethos enforces that every
#                              assumption occurs in R's assertions. cvc5 is run
#                              with --proof-print-reference so the proof does not
#                              redeclare the problem's symbols.
#                 cli-reference : the same proof, but the reference file is passed
#                              through Ethos' documented --reference=X command-line
#                              option instead of the in-proof (reference ...)
#                              command. Included as a control: the two routes are
#                              documented as equivalent and are not.
#                 standalone : the self-contained proof Ethos checks with no
#                              knowledge of any problem file. Used to show what
#                              CPC/Ethos does when binding is not requested.
#   --proof       reuse an existing proof body instead of generating one
#
# Prints key=value lines. Exit status is 0 when the harness ran; the scientific
# verdict is in the printed keys, never in this exit status. A non-zero exit
# means the harness itself malfunctioned.
set -uo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/env.sh"

PROBLEM="" ; REFERENCE="" ; WORK="" ; MODE="reference" ; REUSE=""
while [ $# -gt 0 ]; do
    case "$1" in
        --problem)   PROBLEM="$2"; shift 2 ;;
        --reference) REFERENCE="$2"; shift 2 ;;
        --work)      WORK="$2"; shift 2 ;;
        --mode)      MODE="$2"; shift 2 ;;
        --proof)     REUSE="$2"; shift 2 ;;
        *) echo "STAGE0-HARNESS-ERROR: unknown argument '$1'" >&2; exit 2 ;;
    esac
done
[ -n "$PROBLEM" ] && [ -n "$WORK" ] || { echo "STAGE0-HARNESS-ERROR: --problem and --work are required" >&2; exit 2; }
[ -n "$REFERENCE" ] || REFERENCE="$PROBLEM"

require_tool "$CVC5" "cvc5" "scripts/setup-tools.sh" || exit 2
require_tool "$ETHOS" "ethos" "scripts/setup-tools.sh" || exit 2
require_file "$CPC_SIG" "Cpc.eo" "scripts/setup-tools.sh" || exit 2

mkdir -p "$WORK"
BASE="$(basename "${PROBLEM%.smt2}")"
RAW="${WORK}/${BASE}.${MODE}.cpc.raw"
BODY="${WORK}/${BASE}.${MODE}.cpc"
EO="${WORK}/${BASE}.${MODE}.eo"

echo "pipeline=cpc-ethos"
echo "mode=${MODE}"
echo "problem=${PROBLEM}"
echo "reference=${REFERENCE}"

if [ -n "$REUSE" ]; then
    [ "$(readlink -f "$REUSE")" = "$(readlink -f "$BODY")" ] || cp "$REUSE" "$BODY"
    echo "proof_source=reused:${REUSE}"
    echo "cvc5_exit=skipped"
else
    CVC5_ARGS=(--dump-proofs --proof-format-mode=cpc --proof-print-conclusion)
    case "$MODE" in
        reference|cli-reference) CVC5_ARGS+=(--proof-print-reference) ;;
    esac
    "$CVC5" "${CVC5_ARGS[@]}" "$PROBLEM" > "$RAW" 2> "${WORK}/${BASE}.${MODE}.cpc.stderr"
    echo "cvc5_exit=$?"
    echo "cvc5_args=${CVC5_ARGS[*]}"
    # cvc5 prints "unsat\n(\n" <body> "\n)\n"; the body is what Ethos consumes.
    # This is the same stripping cvc5's own regression runner performs.
    python3 - "$RAW" "$BODY" <<'PY' || { echo "proof_generated=no"; echo "verdict=NO-PROOF"; exit 0; }
import sys
raw = open(sys.argv[1], "rb").read()
if not raw.startswith(b"unsat\n(\n") or not raw.endswith(b"\n)\n"):
    sys.exit(1)
open(sys.argv[2], "wb").write(raw[8:-2])
PY
    echo "proof_source=generated"
fi

echo "proof_generated=yes"
echo "proof_bytes=$(wc -c < "$BODY")"
echo "proof_steps=$(grep -c '^(step' "$BODY")"
# a CPC trust step proves an arbitrary formula with no justification
echo "trust_steps=$(grep -c ':rule trust' "$BODY")"

ABS_REFERENCE="$(cd "$(dirname "$REFERENCE")" && pwd)/$(basename "$REFERENCE")"
{
    # In cli-reference mode the signature is passed with --include so that it is
    # loaded before Ethos parses the reference problem; putting it inside the
    # proof would leave the problem's bit-vector symbols undefined.
    [ "$MODE" = "cli-reference" ] || echo "(include \"${CPC_SIG}\")"
    if [ "$MODE" = "reference" ]; then
        echo "(reference \"${ABS_REFERENCE}\")"
    fi
    cat "$BODY"
} > "$EO"

ETHOS_ARGS=()
if [ "$MODE" = "cli-reference" ]; then
    ETHOS_ARGS+=("--include=${CPC_SIG}" "--reference=${ABS_REFERENCE}")
fi
ETHOS_ARGS+=("$EO")
echo "ethos_args=${ETHOS_ARGS[*]}"

ETHOS_OUT="$("$ETHOS" "${ETHOS_ARGS[@]}" 2> "${WORK}/${BASE}.${MODE}.ethos.stderr")"
ETHOS_EXIT=$?
echo "ethos_exit=${ETHOS_EXIT}"
echo "ethos_stdout=${ETHOS_OUT:-<empty>}"
echo "ethos_stderr_tail=$(last_message "${WORK}/${BASE}.${MODE}.ethos.stderr")"

# Stage-0 acceptance for pipeline A: Ethos must exit 0, print exactly "correct"
# (not "incomplete"), and the proof must contain no trust step.
if [ "$ETHOS_EXIT" -eq 0 ] && [ "$ETHOS_OUT" = "correct" ] && [ "$(grep -c ':rule trust' "$BODY")" -eq 0 ]; then
    echo "verdict=ACCEPT"
else
    echo "verdict=REJECT"
fi
exit 0
