# E2E Runbook

## Goal
Run the full 5-step CRE workflow locally, verify success/failure behavior, and produce an auditable E2E artifact.

## Prerequisites

- Rust toolchain installed
- `jq` available
- Workspace compiles:

```bash
cd /home/a/zkclear/creworkflow
cargo check -p orchestrator
```

## 1) Full workflow simulation

```bash
cd /home/a/zkclear/creworkflow
./scripts/cre-cli-simulate.sh
```

Expected:
- `success-path` succeeds
- `failure-risk-threshold` fails in `confidential-match`
- `failure-api-unavailable` fails on confidential HTTP request path

## 2) Demo script

```bash
cd /home/a/zkclear/creworkflow
./scripts/demo.sh
```

This runs:
1. workspace health check
2. workflow simulation
3. artifact generation

## 3) Generate auditable E2E artifact

```bash
cd /home/a/zkclear/creworkflow
./scripts/generate-e2e-artifact.sh
```

Output location:
- `artifacts/e2e/full-run/`

Generated files:
- `step1_intent_intake_input.json`
- `step1_intent_intake_output.json`
- `step2_confidential_match_input.json`
- `step2_confidential_match_output.json`
- `step3_proof_generate_input.json`
- `step3_proof_generate_output.json`
- `step4_settle_private_input.json`
- `step4_settle_private_output.json`
- `step5_publish_receipt_input.json`
- `step5_publish_receipt_output.json`
- `manifest.json`

## 4) Optional onchain publish from `publish-receipt`

If you want real Sepolia writes instead of simulation mode in `publish-receipt`:

```bash
set -a; source /home/a/zkclear/contracts/.env; set +a
cd /home/a/zkclear/creworkflow/publish-receipt
cargo run --quiet < /tmp/publish_receipt_input.json
```

Requires env:
- `ETH_SEPOLIA_RPC_URL`
- `PRIVATE_KEY`
- optional `ETH_SEPOLIA_CHAIN_ID`
