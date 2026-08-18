#!/usr/bin/env bash
# Top-level Stage-0 entry point.
#
#   ./experiments/validation-spike/scripts/run-all.sh
#
# Runs the whole Stage-0 matrix and regenerates results/toolchain.txt,
# results/matrix.tsv, results/matrix.json and results/summary.md.
#
# Exit status 0 means the harness ran to completion, whatever the scientific
# verdicts were. Exit status 1 means the harness itself malfunctioned: a tool
# crashed unexpectedly, a fixture no longer classifies as declared, or a
# mutation stopped being semantically meaningful. A pipeline failing its
# Stage-0 gate is not a malfunction and does not change this exit status.
set -uo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/env.sh"

require_all_tools || {
    echo "STAGE0-HARNESS-ERROR: toolchain incomplete; run scripts/setup-tools.sh first" >&2
    exit 1
}

if [ -z "$Z3" ]; then
    echo "note: z3 not found; the independent sat/unsat cross-check will be skipped" >&2
    echo "      install it with: pip install z3-solver" >&2
fi

rm -rf "${SPIKE_ROOT}/results/raw/work"
mkdir -p "${SPIKE_ROOT}/results/raw/work"

exec python3 "${SPIKE_ROOT}/scripts/run_matrix.py"
