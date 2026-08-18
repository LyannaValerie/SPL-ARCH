#!/usr/bin/env bash
# Emit a logically mutated copy of a Stage-0 fixture on stdout.
#
# usage: mutate-binding.sh <case-basename> <mutation-id>
#        mutate-binding.sh --list
#
# Mutations are declared in cases/mutations.tsv. They are exact textual
# substitutions on the SMT-LIB problem, chosen so that each one turns an
# expected-UNSAT fixture into a SAT one.
set -uo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/env.sh"
MUTATIONS="${SPIKE_ROOT}/cases/mutations.tsv"

if [ "${1:-}" = "--list" ]; then
    grep -v '^#' "$MUTATIONS" | grep -v '^$' | cut -f1,2
    exit 0
fi

[ $# -eq 2 ] || { echo "STAGE0-HARNESS-ERROR: usage: mutate-binding.sh <case> <mutation-id>" >&2; exit 2; }

python3 - "$MUTATIONS" "${SPIKE_ROOT}/cases/$1" "$1" "$2" <<'PY'
import sys
mutations, case_path, case, mid = sys.argv[1:5]
for line in open(mutations, encoding="utf-8"):
    if line.startswith("#") or not line.strip():
        continue
    cols = line.rstrip("\n").split("\t")
    if cols[0] == case and cols[1] == mid:
        count, search, replace = int(cols[2]), cols[3], cols[4]
        text = open(case_path, encoding="utf-8").read()
        if search not in text:
            sys.stderr.write("STAGE0-HARNESS-ERROR: mutation pattern not found in %s\n" % case)
            sys.exit(2)
        text = text.replace(search, replace) if count == 0 else text.replace(search, replace, count)
        sys.stdout.write(text)
        sys.exit(0)
sys.stderr.write("STAGE0-HARNESS-ERROR: no mutation %s/%s\n" % (case, mid))
sys.exit(2)
PY
