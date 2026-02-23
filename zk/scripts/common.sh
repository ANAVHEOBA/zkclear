#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACTS_DIR="$ROOT_DIR/artifacts"
CIRCUITS_DIR="$ROOT_DIR/circuits"
FIXTURES_DIR="$ROOT_DIR/fixtures"

CIRCOM_VERSION=${CIRCOM_VERSION:-2.1.6}
PTAU_POWER=${PTAU_POWER:-12}
PTAU_FILE="$ARTIFACTS_DIR/ptau/powersOfTau28_hez_final_${PTAU_POWER}.ptau"
DETERMINISTIC_ENTROPY=${DETERMINISTIC_ENTROPY:-zkclear-deterministic-entropy-v1}
MAX_SETUP_SECONDS=${MAX_SETUP_SECONDS:-1800}

ensure_dirs() {
  mkdir -p "$ARTIFACTS_DIR/ptau" "$ARTIFACTS_DIR/settlement_valid" "$ARTIFACTS_DIR/compliance_valid" "$ARTIFACTS_DIR/rebate_valid"
}

have_cmd() {
  command -v "$1" >/dev/null 2>&1
}

run_snarkjs() {
  if have_cmd snarkjs; then
    snarkjs "$@"
  elif [[ -x "$ROOT_DIR/node_modules/.bin/snarkjs" ]]; then
    "$ROOT_DIR/node_modules/.bin/snarkjs" "$@"
  else
    npx --yes snarkjs "$@"
  fi
}

run_circom() {
  if have_cmd circom; then
    circom "$@"
  elif [[ -x "$ROOT_DIR/node_modules/.bin/circom2" ]]; then
    "$ROOT_DIR/node_modules/.bin/circom2" "$@"
  elif have_cmd npx; then
    npx --yes circom2 "$@"
  elif have_cmd docker; then
    docker run --rm -v "$ROOT_DIR":/work -w /work ghcr.io/iden3/circom:$CIRCOM_VERSION circom "$@"
  else
    echo "error: circom not found (no local binary, no npx circom2, no docker fallback)" >&2
    exit 1
  fi
}

require_input_file() {
  local f="$1"
  if [[ ! -f "$f" ]]; then
    echo "error: input file not found: $f" >&2
    exit 1
  fi
}

run_with_timeout() {
  if have_cmd timeout; then
    timeout "$@"
  else
    "$@"
  fi
}
