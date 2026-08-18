#!/usr/bin/env bash
# Pipeline B: cvc5 -> Alethe -> Carcara
#
# usage: run-alethe-carcara.sh --problem P.smt2 --reference R.smt2 --work DIR
#                              [--granularity MODE] [--proof EXISTING.alethe]
#
#   --problem      SMT-LIB problem the proof is generated FROM
#   --reference    SMT-LIB problem Carcara checks the proof AGAINST
#   --granularity  cvc5 --proof-granularity value (default: the cvc5 default)
#   --proof        reuse an existing proof instead of generating one
#
# Prints key=value lines. Exit status is 0 when the harness ran; the scientific
# verdict is in the printed keys. A non-zero exit means the harness malfunctioned.
set -uo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/env.sh"

PROBLEM="" ; REFERENCE="" ; WORK="" ; GRAN="" ; REUSE=""
while [ $# -gt 0 ]; do
    case "$1" in
        --problem)     PROBLEM="$2"; shift 2 ;;
        --reference)   REFERENCE="$2"; shift 2 ;;
        --work)        WORK="$2"; shift 2 ;;
        --granularity) GRAN="$2"; shift 2 ;;
        --proof)       REUSE="$2"; shift 2 ;;
        *) echo "STAGE0-HARNESS-ERROR: unknown argument '$1'" >&2; exit 2 ;;
    esac
done
[ -n "$PROBLEM" ] && [ -n "$WORK" ] || { echo "STAGE0-HARNESS-ERROR: --problem and --work are required" >&2; exit 2; }
[ -n "$REFERENCE" ] || REFERENCE="$PROBLEM"

require_tool "$CVC5" "cvc5" "scripts/setup-tools.sh" || exit 2
require_tool "$CARCARA" "carcara" "scripts/setup-tools.sh" || exit 2

mkdir -p "$WORK"
BASE="$(basename "${PROBLEM%.smt2}")"
TAG="${GRAN:-default}"
PROOF="${WORK}/${BASE}.${TAG}.alethe"

echo "pipeline=alethe-carcara"
echo "granularity=${TAG}"
echo "problem=${PROBLEM}"
echo "reference=${REFERENCE}"

if [ -n "$REUSE" ]; then
    [ "$(readlink -f "$REUSE")" = "$(readlink -f "$PROOF")" ] || cp "$REUSE" "$PROOF"
    echo "proof_source=reused:${REUSE}"
    echo "cvc5_exit=skipped"
else
    CVC5_ARGS=(--dump-proofs --proof-format-mode=alethe)
    [ -n "$GRAN" ] && CVC5_ARGS+=("--proof-granularity=${GRAN}")
    "$CVC5" "${CVC5_ARGS[@]}" "$PROBLEM" > "${WORK}/${BASE}.${TAG}.raw" 2> "${WORK}/${BASE}.${TAG}.cvc5.stderr"
    echo "cvc5_exit=$?"
    echo "cvc5_args=${CVC5_ARGS[*]}"
    # Carcara consumes the proof without the leading "unsat" line.
    if [ "$(head -1 "${WORK}/${BASE}.${TAG}.raw")" != "unsat" ]; then
        echo "proof_generated=no"
        echo "verdict=NO-PROOF"
        exit 0
    fi
    tail -n +2 "${WORK}/${BASE}.${TAG}.raw" > "$PROOF"
    echo "proof_source=generated"
fi

echo "proof_generated=yes"
echo "proof_bytes=$(wc -c < "$PROOF")"
echo "proof_steps=$(grep -c '^(step' "$PROOF")"
# an Alethe hole step is an unchecked escape hatch: it asserts its conclusion
echo "hole_steps=$(grep -c ':rule hole' "$PROOF")"
# rare_rewrite steps are only checkable when a matching RARE database is supplied
echo "rare_rewrite_steps=$(grep -c ':rule rare_rewrite' "$PROOF")"

CARCARA_OUT="$("$CARCARA" check "$PROOF" "$REFERENCE" 2> "${WORK}/${BASE}.${TAG}.carcara.stderr")"
CARCARA_EXIT=$?
echo "carcara_exit=${CARCARA_EXIT}"
echo "carcara_stdout=${CARCARA_OUT:-<empty>}"
echo "carcara_stderr_tail=$(last_message "${WORK}/${BASE}.${TAG}.carcara.stderr")"

# Stage-0 acceptance for pipeline B: Carcara must exit 0 and print exactly
# "valid". "holey" means it accepted a proof containing steps it did not check,
# and is NOT acceptance under the Stage-0 assurance rule.
if [ "$CARCARA_EXIT" -eq 0 ] && [ "$CARCARA_OUT" = "valid" ] && [ "$(grep -c ':rule hole' "$PROOF")" -eq 0 ]; then
    echo "verdict=ACCEPT"
else
    echo "verdict=REJECT"
fi
exit 0
