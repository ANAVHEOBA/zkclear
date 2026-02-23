#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/common.sh"

CIRCUIT=${1:-settlement_valid}
ensure_dirs

CIRCUIT_FILE="$CIRCUITS_DIR/${CIRCUIT}.circom"
OUT_DIR="$ARTIFACTS_DIR/${CIRCUIT}"

if [[ ! -f "$CIRCUIT_FILE" ]]; then
  echo "error: circuit file not found: $CIRCUIT_FILE" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"

run_circom "$CIRCUIT_FILE" --r1cs --wasm --sym -o "$OUT_DIR"

echo "compiled: $CIRCUIT"
echo "r1cs: $OUT_DIR/${CIRCUIT}.r1cs"
echo "wasm: $OUT_DIR/${CIRCUIT}_js/${CIRCUIT}.wasm"
