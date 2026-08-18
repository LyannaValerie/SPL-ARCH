#!/usr/bin/env bash
# Shared environment for the SPL-ARCH Stage-0 validation spike.
#
# Every path resolved here points inside experiments/validation-spike/.tools,
# which is git-ignored. Nothing in this spike installs global machine state and
# nothing in this spike is vendored into the repository.
#
# Source this file; do not execute it.

set -o pipefail

SPIKE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export SPIKE_ROOT
export TOOLS_DIR="${SPIKE_ROOT}/.tools"
export TOOLS_BIN="${TOOLS_DIR}/bin"
export TOOLS_SRC="${TOOLS_DIR}/src"
export TOOLS_DL="${TOOLS_DIR}/dl"

# The tool paths below can be overridden from the environment; the harness
# self-test uses that to check that a missing tool fails loudly.

# Pinned versions. See results/toolchain.txt for why each pin was chosen.
export CVC5_VERSION="cvc5-1.3.4"
export CVC5_RELEASE_ASSET="cvc5-Linux-x86_64-static.zip"
# Pinned by cvc5-1.3.4 itself, in contrib/get-ethos-checker.
export ETHOS_COMMIT="221641668d75eaffd308e0511d63962cea937110"
# Carcara has no tagged release; this is the commit used for the experiment.
export CARCARA_COMMIT="6624ea80cf1985ada473c0705869c78353e4282d"

export CVC5="${CVC5:-${TOOLS_BIN}/cvc5}"
export ETHOS="${ETHOS:-${TOOLS_BIN}/ethos}"
export CARCARA="${CARCARA:-${TOOLS_BIN}/carcara}"
export CPC_SIG="${CPC_SIG:-${TOOLS_SRC}/cvc5-src/proofs/eo/cpc/Cpc.eo}"

# Independent second solver, used only to sanity-check sat/unsat expectations.
# It is deliberately NOT part of any admission-evidence path.
export Z3="${Z3:-$(command -v z3 || true)}"

# fail loudly on a missing tool
require_tool() {
    local path="$1" name="$2" hint="$3"
    if [ ! -x "$path" ]; then
        echo "STAGE0-HARNESS-ERROR: missing tool '${name}' at ${path}" >&2
        echo "  run: ${hint}" >&2
        return 1
    fi
    return 0
}

require_file() {
    local path="$1" name="$2" hint="$3"
    if [ ! -f "$path" ]; then
        echo "STAGE0-HARNESS-ERROR: missing file '${name}' at ${path}" >&2
        echo "  run: ${hint}" >&2
        return 1
    fi
    return 0
}

require_all_tools() {
    local rc=0
    require_tool "$CVC5"    "cvc5"    "scripts/setup-tools.sh" || rc=1
    require_tool "$ETHOS"   "ethos"   "scripts/setup-tools.sh" || rc=1
    require_tool "$CARCARA" "carcara" "scripts/setup-tools.sh" || rc=1
    require_file "$CPC_SIG" "Cpc.eo"  "scripts/setup-tools.sh" || rc=1
    return $rc
}

# last non-empty line of a file, trimmed to one printable field
last_message() {
    [ -f "$1" ] || { echo ""; return 0; }
    grep -v '^[[:space:]]*$' "$1" | tail -1 | tr -d '\r\n' | cut -c1-200
}
