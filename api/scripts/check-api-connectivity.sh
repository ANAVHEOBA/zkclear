#!/usr/bin/env bash
set -euo pipefail

GATEWAY_URL="${GATEWAY_URL:-http://127.0.0.1:8081}"
COMPLIANCE_URL="${COMPLIANCE_URL:-http://127.0.0.1:8082}"
POLICY_URL="${POLICY_URL:-http://127.0.0.1:8083}"
PROOF_URL="${PROOF_URL:-http://127.0.0.1:8084}"

echo "== Health checks =="
echo "gateway:    ${GATEWAY_URL}/v1/intents/submit (POST endpoint, no health route)"
echo "compliance: ${COMPLIANCE_URL}/v1/compliance/health"
echo "policy:     ${POLICY_URL}/v1/policy/active"
echo "proof:      ${PROOF_URL}/v1/proof-jobs/health"

curl -fsS "${COMPLIANCE_URL}/v1/compliance/health" >/tmp/zkclear-compliance-health.json
curl -fsS "${POLICY_URL}/v1/policy/active" >/tmp/zkclear-policy-active.json
curl -fsS "${PROOF_URL}/v1/proof-jobs/health" >/tmp/zkclear-proof-health.json

echo "compliance health: $(cat /tmp/zkclear-compliance-health.json)"
echo "policy active:     $(cat /tmp/zkclear-policy-active.json)"
echo "proof health:      $(cat /tmp/zkclear-proof-health.json)"

echo
echo "== Functional smoke: submit proof job =="

WORKFLOW_RUN_ID="run-smoke-$(date +%s)"
IDEMPOTENCY_KEY="idem-smoke-$(date +%s)"

cat >/tmp/zkclear-proof-submit.json <<JSON
{
  "workflowRunId": "${WORKFLOW_RUN_ID}",
  "policyVersion": "policy-v1",
  "receiptContext": {
    "receiptHash": "0xsmokehash",
    "binding": {
      "workflowRunId": "${WORKFLOW_RUN_ID}",
      "policyVersion": "policy-v1",
      "domainSeparator": "zkclear:v1"
    }
  },
  "proofType": "compliance",
  "idempotencyKey": "${IDEMPOTENCY_KEY}"
}
JSON

curl -fsS -X POST "${PROOF_URL}/v1/proof-jobs" \
  -H "content-type: application/json" \
  --data @/tmp/zkclear-proof-submit.json >/tmp/zkclear-proof-submit-response.json

echo "proof submit response: $(cat /tmp/zkclear-proof-submit-response.json)"

echo
echo "== Query by run id =="
curl -fsS "${PROOF_URL}/v1/proof-jobs/run/${WORKFLOW_RUN_ID}" >/tmp/zkclear-proof-by-run.json
echo "proof jobs by run: $(cat /tmp/zkclear-proof-by-run.json)"

echo
echo "Connectivity checks completed."
