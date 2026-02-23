#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/common.sh"

CIRCUIT=${1:-settlement_valid}
INPUT_FILE=${2:-"$FIXTURES_DIR/${CIRCUIT}.fixture.json"}
ensure_dirs
require_input_file "$INPUT_FILE"

OUT_DIR="$ARTIFACTS_DIR/${CIRCUIT}"
WASM_DIR="$OUT_DIR/${CIRCUIT}_js"
WASM_FILE="$WASM_DIR/${CIRCUIT}.wasm"
WITNESS_BIN="$WASM_DIR/generate_witness.js"
ZKEY_FINAL="$OUT_DIR/${CIRCUIT}.zkey"

if [[ ! -f "$WASM_FILE" || ! -f "$WITNESS_BIN" ]]; then
  echo "error: missing wasm witness generator. run scripts/compile.sh $CIRCUIT first" >&2
  exit 1
fi

if [[ ! -f "$ZKEY_FINAL" ]]; then
  echo "error: missing zkey. run scripts/setup.sh $CIRCUIT first" >&2
  exit 1
fi

WITNESS_FILE="$OUT_DIR/${CIRCUIT}.wtns"
PROOF_FILE="$OUT_DIR/${CIRCUIT}.proof.json"
PUBLIC_FILE="$OUT_DIR/${CIRCUIT}.public.json"

node "$WITNESS_BIN" "$WASM_FILE" "$INPUT_FILE" "$WITNESS_FILE"
run_snarkjs groth16 prove "$ZKEY_FINAL" "$WITNESS_FILE" "$PROOF_FILE" "$PUBLIC_FILE"

echo "proof generated: $CIRCUIT"
echo "proof: $PROOF_FILE"
echo "public signals: $PUBLIC_FILE"
