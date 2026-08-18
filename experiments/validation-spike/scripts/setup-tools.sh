#!/usr/bin/env bash
# Fetch and build the pinned Stage-0 toolchain into experiments/validation-spike/.tools
#
# Nothing here writes global machine state except the system packages listed
# under PREREQUISITES, which are not installed by this script.
#
# PREREQUISITES (not installed here):
#   git, curl, unzip, cmake >= 3.12, a C++17 compiler, GNU make, libgmp-dev,
#   cargo/rustup (Carcara pins its own toolchain via rust-toolchain.toml)
#
# Optional, for the independent sat/unsat cross-check only:
#   pip install z3-solver
set -euo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/env.sh"

mkdir -p "$TOOLS_BIN" "$TOOLS_SRC" "$TOOLS_DL"

echo "== cvc5 ${CVC5_VERSION} (official static release build)"
if [ ! -x "${TOOLS_DL}/cvc5-Linux-x86_64-static/bin/cvc5" ]; then
    curl -sSL --fail -o "${TOOLS_DL}/${CVC5_RELEASE_ASSET}" \
        "https://github.com/cvc5/cvc5/releases/download/${CVC5_VERSION}/${CVC5_RELEASE_ASSET}"
    ( cd "$TOOLS_DL" && unzip -o -q "${CVC5_RELEASE_ASSET}" )
fi
ln -sf "${TOOLS_DL}/cvc5-Linux-x86_64-static/bin/cvc5" "${TOOLS_BIN}/cvc5"

echo "== cvc5 source at ${CVC5_VERSION} (only for proofs/eo, the CPC signature)"
if [ ! -f "$CPC_SIG" ]; then
    rm -rf "${TOOLS_SRC}/cvc5-src"
    git clone --depth 1 --filter=blob:none --sparse --branch "${CVC5_VERSION}" \
        https://github.com/cvc5/cvc5.git "${TOOLS_SRC}/cvc5-src"
    ( cd "${TOOLS_SRC}/cvc5-src" && git sparse-checkout set proofs/eo contrib )
fi

echo "== ethos ${ETHOS_COMMIT} (the commit cvc5 ${CVC5_VERSION} pins in contrib/get-ethos-checker)"
if [ ! -x "${TOOLS_SRC}/ethos/build/src/ethos" ]; then
    if [ ! -d "${TOOLS_SRC}/ethos/.git" ]; then
        git clone --depth 1 https://github.com/cvc5/ethos.git "${TOOLS_SRC}/ethos"
    fi
    ( cd "${TOOLS_SRC}/ethos" \
      && git fetch --depth 1 origin "${ETHOS_COMMIT}" \
      && git checkout -q "${ETHOS_COMMIT}" \
      && ./configure.sh --prefix="${TOOLS_DIR}" \
      && cd build && make -j"$(nproc)" )
fi
ln -sf "${TOOLS_SRC}/ethos/build/src/ethos" "${TOOLS_BIN}/ethos"

echo "== carcara ${CARCARA_COMMIT}"
if [ ! -x "${TOOLS_SRC}/carcara/target/release/carcara" ]; then
    if [ ! -d "${TOOLS_SRC}/carcara/.git" ]; then
        git clone --depth 1 https://github.com/ufmg-smite/carcara.git "${TOOLS_SRC}/carcara"
    fi
    ( cd "${TOOLS_SRC}/carcara" \
      && git fetch --depth 1 origin "${CARCARA_COMMIT}" \
      && git checkout -q "${CARCARA_COMMIT}" \
      && cargo build --release )
fi
ln -sf "${TOOLS_SRC}/carcara/target/release/carcara" "${TOOLS_BIN}/carcara"

require_all_tools
echo "== toolchain ready in ${TOOLS_DIR}"
