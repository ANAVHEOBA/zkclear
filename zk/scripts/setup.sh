#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/common.sh"

CIRCUIT=${1:-settlement_valid}
ensure_dirs

OUT_DIR="$ARTIFACTS_DIR/${CIRCUIT}"
R1CS_FILE="$OUT_DIR/${CIRCUIT}.r1cs"
ZKEY_0="$OUT_DIR/${CIRCUIT}_0000.zkey"
ZKEY_FINAL="$OUT_DIR/${CIRCUIT}.zkey"
VKEY_FILE="$OUT_DIR/${CIRCUIT}.vkey.json"

if [[ ! -f "$R1CS_FILE" ]]; then
  echo "error: missing r1cs. run scripts/compile.sh $CIRCUIT first" >&2
  exit 1
fi

if [[ -f "$ZKEY_FINAL" && -f "$VKEY_FILE" && "$ZKEY_FINAL" -nt "$R1CS_FILE" && "$VKEY_FILE" -nt "$R1CS_FILE" ]]; then
  echo "setup reuse: existing zkey + vkey found for $CIRCUIT"
  echo "zkey: $ZKEY_FINAL"
  echo "vkey: $VKEY_FILE"
  exit 0
fi

if [[ ! -f "$PTAU_FILE" ]]; then
  PTAU_NEW="$ARTIFACTS_DIR/ptau/pot${PTAU_POWER}_0000.ptau"
  PTAU_CONTRIB="$ARTIFACTS_DIR/ptau/pot${PTAU_POWER}_0001.ptau"

  echo "setup stage: powersoftau new (power=$PTAU_POWER)"
  run_snarkjs powersoftau new bn128 "$PTAU_POWER" "$PTAU_NEW" -v
  echo "setup stage: powersoftau contribute"
  printf '%s\n' "$DETERMINISTIC_ENTROPY" | run_snarkjs powersoftau contribute "$PTAU_NEW" "$PTAU_CONTRIB" --name="zkclear_ptau" -v
  echo "setup stage: powersoftau prepare phase2"
  run_snarkjs powersoftau prepare phase2 "$PTAU_CONTRIB" "$PTAU_FILE" -v
fi

echo "setup stage: groth16 setup (timeout=${MAX_SETUP_SECONDS}s)"
rm -f "$ZKEY_0"
if have_cmd timeout; then
  if [[ -x "$ROOT_DIR/node_modules/.bin/snarkjs" ]]; then
    timeout "${MAX_SETUP_SECONDS}s" "$ROOT_DIR/node_modules/.bin/snarkjs" groth16 setup "$R1CS_FILE" "$PTAU_FILE" "$ZKEY_0"
  elif have_cmd snarkjs; then
    timeout "${MAX_SETUP_SECONDS}s" snarkjs groth16 setup "$R1CS_FILE" "$PTAU_FILE" "$ZKEY_0"
  else
    timeout "${MAX_SETUP_SECONDS}s" npx --yes snarkjs groth16 setup "$R1CS_FILE" "$PTAU_FILE" "$ZKEY_0"
  fi
else
  run_snarkjs groth16 setup "$R1CS_FILE" "$PTAU_FILE" "$ZKEY_0"
fi
echo "setup stage: zkey contribute"
printf '%s\n' "$DETERMINISTIC_ENTROPY" | run_snarkjs zkey contribute "$ZKEY_0" "$ZKEY_FINAL" --name="zkclear_phase2" -v
echo "setup stage: export verification key"
run_snarkjs zkey export verificationkey "$ZKEY_FINAL" "$VKEY_FILE"

echo "setup complete: $CIRCUIT"
echo "zkey: $ZKEY_FINAL"
echo "vkey: $VKEY_FILE"
