#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

NOW_TS=1750000000
EXPIRY_TS=1750003600

payload_buy='{"asset_pair":"ETH/USDC","side":"buy","size":"5.0","limit_price":"3020.0","expiry":1750003600,"counterparty_constraints":{"allow_list":["desk_a"],"deny_list":["desk_x"]},"nonce":"nonce-a","issued_at":1749999900}'
payload_sell='{"asset_pair":"ETH/USDC","side":"sell","size":"4.0","limit_price":"3000.0","expiry":1750003600,"counterparty_constraints":{"allow_list":["desk_a"],"deny_list":["desk_x"]},"nonce":"nonce-b","issued_at":1749999905}'

make_envelope() {
  local intent_id="$1"
  local signer="$2"
  local payload="$3"
  local submitted_at="$4"
  local sig
  local cipher
  sig="$(printf '%s%s' "$signer" "$payload" | sha256sum | awk '{print $1}')"
  cipher="$(printf '%s' "$payload" | base64 -w0)"
  cat <<EOF
{
  "intent_id": "${intent_id}",
  "signer": "${signer}",
  "signature_hex": "${sig}",
  "ciphertext_b64": "${cipher}",
  "submitted_at": ${submitted_at}
}
EOF
}

build_input() {
  local risk_score="$1"
  local api_enabled_json="$2"
  local endpoint_json="$3"
  cat <<EOF
{
  "intent_intake": {
    "encrypted_intents": [
      $(make_envelope "intent-buy-1" "0x1111" "$payload_buy" 1749999950),
      $(make_envelope "intent-sell-1" "0x2222" "$payload_sell" 1749999951)
    ],
    "current_unix_ts": ${NOW_TS},
    "seen_nonces": [],
    "payload_reference": "s3://zkclear/demo/run-1"
  },
  "policy": {
    "policy_version": 1,
    "expected_policy_version": 1,
    "max_risk_score": 70,
    "max_notional": 500000.0
  },
  "external_signals": {
    "api_available": true,
    "compliance_passed": true,
    "risk_score": ${risk_score},
    "attestation_payload": "provider=mock_conf_http;result=ok"
  },
  "confidential_http": {
    "enabled": ${api_enabled_json},
    "endpoint": ${endpoint_json},
    "timeout_ms": 1200
  },
  "proving": {
    "proving_timeout_ms": 3000,
    "estimated_proving_time_ms": 500,
    "domain_separator": "zkclear-sepolia-domain-v1",
    "witness_seed": "witness-seed-123456"
  },
  "settlement_execution": {
    "max_retries": 2,
    "timeout_ms": 3000,
    "estimated_execution_ms": 500,
    "retryable_error_sequence": [false]
  },
  "publish": {
    "settlement_registry": "0x3e3a14f46d13e156daa99bf234224a57b1c79da5",
    "publisher_address": "0x6D21167d874C842386e8c484519B5ddBBaB87b43"
  }
}
EOF
}

run_case() {
  local name="$1"
  local input_file="$2"
  local expect_fail="$3"

  echo "=== ${name} ==="
  if cargo run -q -p orchestrator < "$input_file" > "${TMP_DIR}/${name}.out" 2> "${TMP_DIR}/${name}.err"; then
    if [[ "$expect_fail" == "true" ]]; then
      echo "[unexpected] ${name} succeeded"
      cat "${TMP_DIR}/${name}.out"
      return 1
    fi
    echo "[ok] ${name} succeeded"
    cat "${TMP_DIR}/${name}.out"
  else
    if [[ "$expect_fail" == "false" ]]; then
      echo "[unexpected] ${name} failed"
      cat "${TMP_DIR}/${name}.err"
      return 1
    fi
    echo "[ok] ${name} failed as expected"
    cat "${TMP_DIR}/${name}.err"
  fi
  echo
}

success_input="${TMP_DIR}/success.json"
fail_risk_input="${TMP_DIR}/fail-risk.json"
fail_api_input="${TMP_DIR}/fail-api.json"

build_input "42" "false" "\"\"" > "$success_input"
build_input "99" "false" "\"\"" > "$fail_risk_input"
build_input "42" "true" "\"http://127.0.0.1:9/confidential-signals\"" > "$fail_api_input"

cd "$ROOT_DIR"

echo "Running CRE workflow simulation from workspace: $ROOT_DIR"
echo

run_case "success-path" "$success_input" "false"
run_case "failure-risk-threshold" "$fail_risk_input" "true"
run_case "failure-api-unavailable" "$fail_api_input" "true"

echo "Simulation complete."
