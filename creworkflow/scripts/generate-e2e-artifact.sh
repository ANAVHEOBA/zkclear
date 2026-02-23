#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_DIR="$ROOT_DIR/artifacts/e2e/full-run"
mkdir -p "$ARTIFACT_DIR"

NOW_TS=1750000000

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
  cat <<EON
{
  "intent_id": "${intent_id}",
  "signer": "${signer}",
  "signature_hex": "${sig}",
  "ciphertext_b64": "${cipher}",
  "submitted_at": ${submitted_at}
}
EON
}

STEP1_IN="$ARTIFACT_DIR/step1_intent_intake_input.json"
STEP1_OUT="$ARTIFACT_DIR/step1_intent_intake_output.json"
STEP2_IN="$ARTIFACT_DIR/step2_confidential_match_input.json"
STEP2_OUT="$ARTIFACT_DIR/step2_confidential_match_output.json"
STEP3_IN="$ARTIFACT_DIR/step3_proof_generate_input.json"
STEP3_OUT="$ARTIFACT_DIR/step3_proof_generate_output.json"
STEP4_IN="$ARTIFACT_DIR/step4_settle_private_input.json"
STEP4_OUT="$ARTIFACT_DIR/step4_settle_private_output.json"
STEP5_IN="$ARTIFACT_DIR/step5_publish_receipt_input.json"
STEP5_OUT="$ARTIFACT_DIR/step5_publish_receipt_output.json"
MANIFEST="$ARTIFACT_DIR/manifest.json"

cat > "$STEP1_IN" <<EON
{
  "encrypted_intents": [
    $(make_envelope "intent-buy-1" "0x1111" "$payload_buy" 1749999950),
    $(make_envelope "intent-sell-1" "0x2222" "$payload_sell" 1749999951)
  ],
  "current_unix_ts": ${NOW_TS},
  "seen_nonces": [],
  "payload_reference": "s3://zkclear/e2e/full-run"
}
EON

cd "$ROOT_DIR"

cargo run -q -p intent-intake < "$STEP1_IN" > "$STEP1_OUT"

jq -n \
  --arg workflow_run_id "$(jq -r '.workflow_run_id' "$STEP1_OUT")" \
  --argjson intents "$(jq '[.normalized_private_intents[] | .size = (.size|tonumber) | .limit_price = (.limit_price|tonumber)]' "$STEP1_OUT")" \
  --argjson policy '{"policy_version":1,"expected_policy_version":1,"max_risk_score":70,"max_notional":500000.0}' \
  --argjson external '{"api_available":true,"compliance_passed":true,"risk_score":42,"attestation_payload":"provider=mock_conf_http;result=ok"}' \
  '{
    workflow_run_id: $workflow_run_id,
    policy: $policy,
    intents: $intents,
    external_signals: $external
  }' > "$STEP2_IN"

cargo run -q -p confidential-match < "$STEP2_IN" > "$STEP2_OUT"

jq -n \
  --arg workflow_run_id "$(jq -r '.workflow_run_id' "$STEP2_OUT")" \
  --arg asset_pair "$(jq -r '.private_settlement_params.asset_pair' "$STEP2_OUT")" \
  --arg buy_intent_id "$(jq -r '.private_settlement_params.buy_intent_id' "$STEP2_OUT")" \
  --arg sell_intent_id "$(jq -r '.private_settlement_params.sell_intent_id' "$STEP2_OUT")" \
  --argjson execution_size "$(jq '.private_settlement_params.execution_size' "$STEP2_OUT")" \
  --argjson execution_price "$(jq '.private_settlement_params.execution_price' "$STEP2_OUT")" \
  --argjson notional "$(jq '.private_settlement_params.notional' "$STEP2_OUT")" \
  --argjson policy_version "$(jq '.policy_check_result.policy_version' "$STEP2_OUT")" \
  '{
    workflow_run_id: $workflow_run_id,
    match_result: { accepted: true },
    policy_result: { passed: true, policy_version: $policy_version },
    settlement_params: {
      asset_pair: $asset_pair,
      buy_intent_id: $buy_intent_id,
      sell_intent_id: $sell_intent_id,
      execution_size: $execution_size,
      execution_price: $execution_price,
      notional: $notional
    },
    proving_timeout_ms: 3000,
    estimated_proving_time_ms: 500,
    domain_separator: "zkclear-sepolia-domain-v1",
    witness_seed: "witness-seed-123456"
  }' > "$STEP3_IN"

cargo run -q -p proof-generate < "$STEP3_IN" > "$STEP3_OUT"

jq -n \
  --arg workflow_run_id "$(jq -r '.workflow_run_id' "$STEP2_OUT")" \
  --arg proof_hash "$(jq -r '.proof_hash' "$STEP3_OUT")" \
  --arg receipt_hash "$(jq -r '.receipt_hash' "$STEP3_OUT")" \
  --arg asset "$(jq -r '.private_settlement_params.asset_pair' "$STEP2_OUT")" \
  --arg from_account "$(jq -r '.private_settlement_params.buy_intent_id' "$STEP2_OUT")" \
  --arg to_account "$(jq -r '.private_settlement_params.sell_intent_id' "$STEP2_OUT")" \
  --argjson amount "$(jq '.private_settlement_params.notional' "$STEP2_OUT")" \
  '{
    workflow_run_id: $workflow_run_id,
    proof_bundle: { proof_hash: $proof_hash, receipt_hash: $receipt_hash, approved: true },
    settlement_instruction: {
      asset: $asset,
      amount: $amount,
      from_account: $from_account,
      to_account: $to_account,
      transfer_simulation_ok: true,
      counterparty_conflict: false
    },
    execution: {
      max_retries: 2,
      timeout_ms: 3000,
      estimated_execution_ms: 500,
      retryable_error_sequence: [false]
    }
  }' > "$STEP4_IN"

cargo run -q -p settle-private < "$STEP4_IN" > "$STEP4_OUT"

jq -n \
  --arg workflow_run_id "$(jq -r '.workflow_run_id' "$STEP2_OUT")" \
  --arg proof_hash "$(jq -r '.proof_hash' "$STEP3_OUT")" \
  --arg receipt_hash "$(jq -r '.receipt_hash' "$STEP3_OUT")" \
  --argjson policy_version "$(jq '.policy_version' "$STEP3_OUT")" \
  --arg proof_hex "0x$(jq -r '.proof_bytes[]' "$STEP3_OUT" | awk 'BEGIN{ORS=""} {printf "%02x", $1}')" \
  --argjson public_signals "$(jq '.public_signals' "$STEP3_OUT")" \
  '{
    settlement_registry: "0x3e3a14f46d13e156daa99bf234224a57b1c79da5",
    publisher_address: "0x6D21167d874C842386e8c484519B5ddBBaB87b43",
    workflow_run_id: $workflow_run_id,
    proof_hash: $proof_hash,
    policy_version: $policy_version,
    status: "settled",
    receipt_hash: $receipt_hash,
    proof_hex: $proof_hex,
    public_signals: $public_signals,
    chain_validation: {
      authorized_publisher: true,
      policy_active: true,
      proof_valid: true,
      signal_binding_valid: true,
      duplicate_workflow_run: false,
      duplicate_receipt_hash: false
    }
  }' > "$STEP5_IN"

cargo run -q -p publish-receipt < "$STEP5_IN" > "$STEP5_OUT"

jq -n \
  --arg generated_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg root "$ROOT_DIR" \
  --arg step1_in "$STEP1_IN" \
  --arg step1_out "$STEP1_OUT" \
  --arg step2_in "$STEP2_IN" \
  --arg step2_out "$STEP2_OUT" \
  --arg step3_in "$STEP3_IN" \
  --arg step3_out "$STEP3_OUT" \
  --arg step4_in "$STEP4_IN" \
  --arg step4_out "$STEP4_OUT" \
  --arg step5_in "$STEP5_IN" \
  --arg step5_out "$STEP5_OUT" \
  '{
    generated_at: $generated_at,
    workspace_root: $root,
    steps: [
      {id: "intent-intake", input: $step1_in, output: $step1_out},
      {id: "confidential-match", input: $step2_in, output: $step2_out},
      {id: "proof-generate", input: $step3_in, output: $step3_out},
      {id: "settle-private", input: $step4_in, output: $step4_out},
      {id: "publish-receipt", input: $step5_in, output: $step5_out}
    ]
  }' > "$MANIFEST"

echo "E2E artifact generated: $MANIFEST"
