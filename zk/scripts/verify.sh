#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/common.sh"

CIRCUIT=${1:-settlement_valid}
ensure_dirs

OUT_DIR="$ARTIFACTS_DIR/${CIRCUIT}"
VKEY_FILE="$OUT_DIR/${CIRCUIT}.vkey.json"
PROOF_FILE="$OUT_DIR/${CIRCUIT}.proof.json"
PUBLIC_FILE="$OUT_DIR/${CIRCUIT}.public.json"

require_input_file "$VKEY_FILE"
require_input_file "$PROOF_FILE"
require_input_file "$PUBLIC_FILE"

run_snarkjs groth16 verify "$VKEY_FILE" "$PUBLIC_FILE" "$PROOF_FILE"
