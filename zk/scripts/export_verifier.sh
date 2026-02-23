#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/common.sh"

CIRCUIT=${1:-settlement_valid}
ensure_dirs

OUT_DIR="$ARTIFACTS_DIR/${CIRCUIT}"
ZKEY_FINAL="$OUT_DIR/${CIRCUIT}.zkey"
VERIFIER_SOL="$OUT_DIR/${CIRCUIT}.verifier.sol"

require_input_file "$ZKEY_FINAL"

run_snarkjs zkey export solidityverifier "$ZKEY_FINAL" "$VERIFIER_SOL"

echo "verifier exported: $VERIFIER_SOL"
