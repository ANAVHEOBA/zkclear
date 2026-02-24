# ZK-Clear

Private OTC + treasury settlement rail where matching/compliance runs offchain, settlement logic is privacy-preserving, and only minimal proof-backed receipts are published onchain.

## System Overview

ZK-Clear is split into four major domains:

1. `zk/`
2. `contracts/`
3. `creworkflow/`
4. `api/`

Core end-to-end path:

1. Encrypted intents are submitted.
2. Confidential matching and policy checks run offchain.
3. ZK proof artifacts are generated.
4. Private settlement execution step runs.
5. Receipt is published on Sepolia via `SettlementRegistry`.

## Repository Layout

- `zk/`: Circuits, proving scripts, fixtures, vectors, verifier export.
- `contracts/`: Access control, policy manager, verifier bridge, replay protection, settlement registry, deploy scripts.
- `creworkflow/`: Five workflow steps plus orchestrator and workflow spec.
- `api/encrypted-intent-gateway/`: Intake API for encrypted intents.
- `api/compliance-attestation-adapter/`: Compliance normalization and attestation hashing.
- `api/policy-snapshot/`: Policy snapshot versioning/activation/effective policy lookup.
- `api/proof-job-coordinator/`: Bridge between `zk/` proving and `creworkflow/publish-receipt` onchain publish.

## Privacy Boundary

Private/offchain:

- Counterparty identities
- Notional/terms
- Raw compliance provider payloads
- Signing and API secrets

Public/onchain:

- `workflowRunId`
- `proofHash`
- `policyVersion`
- `status`
- `timestamp`
- `receiptHash`

## Workflow Logic (5 Steps)

1. `intent-intake`
2. `confidential-match`
3. `proof-generate`
4. `settle-private`
5. `publish-receipt`

Main orchestrator:

- `creworkflow/orchestrator/src/main.rs`
- Workflow spec: `creworkflow/workflow-spec/otc_settlement.yaml`

## Proof Job Coordinator

Service path: `api/proof-job-coordinator`

Purpose:

- Accept proof jobs for `settlement|compliance|rebate`
- Queue and process jobs with retries/lease
- Run prover from `zk/`
- Validate public signal bindings
- Publish receipt to Sepolia through `publish-receipt` module

Current API:

- `POST /v1/auth/wallet/nonce`
- `POST /v1/auth/wallet/verify`
- `GET /v1/auth/wallet/me`
- `POST /v1/proof-jobs`
- `GET /v1/proof-jobs/:job_id`
- `GET /v1/proof-jobs/run/:workflow_run_id`
- `POST /v1/proof-jobs/:job_id/retry`
- `POST /v1/proof-jobs/:job_id/status`
- `GET /v1/proof-jobs/queue-stats`
- `GET /v1/proof-jobs/health`

State machine:

- `QUEUED -> PROVING -> PROVED -> PUBLISHING -> PUBLISHED`
- or terminal `FAILED`

Persistence:

- In-memory store for hot state
- Redis-backed durable records for:
  - `proof_jobs`
  - `proof_job_attempts`
  - `proof_outputs`
  - `publish_receipts`

## Security Notes

- Keep `.env` files out of version control.
- Write endpoints can require internal signature auth in proof-job-coordinator:
  - `INTERNAL_AUTH_ENABLED=true`
  - `INTERNAL_AUTH_SECRET=<secret>`
  - Header: `x-internal-signature`
- Never log private keys, API keys, or decrypted payloads.

## Local Prerequisites

- Rust toolchain
- Node.js + npm
- Redis
- Foundry (for contracts)
- `snarkjs`/`circom` toolchain (or configured fallback path from `zk/scripts`)

## Quick Start

### 1) ZK artifacts

```bash
cd zk
./scripts/pipeline.sh settlement_valid fixtures/settlement_valid.fixture.json
```

### 2) Contracts

```bash
cd contracts
forge build
forge test
```

### 3) CRE workflow simulation

```bash
cd creworkflow
cargo test
./scripts/cre-cli-simulate.sh
```

### 4) API services (example: proof-job-coordinator)

```bash
cd api/proof-job-coordinator
cargo test
cargo run
```

## Environment Variables (Proof Job Coordinator)

Required:

- `RUST_ENV`
- `API_HOST`
- `API_PORT`

Queue + worker:

- `REDIS_URL`
- `WORKER_ENABLED`
- `WORKER_POLL_SECONDS`
- `WORKER_LEASE_SECONDS`
- `WORKER_MAX_RETRIES`
- `WORKER_BACKOFF_BASE_SECONDS`

Prover:

- `ZK_ROOT_DIR`
- `PROVE_TIMEOUT_SECONDS`
- `PROVE_BUDGET_SETTLEMENT_SECONDS`
- `PROVE_BUDGET_COMPLIANCE_SECONDS`
- `PROVE_BUDGET_REBATE_SECONDS`
- `SIGNAL_DOMAIN_SEPARATOR`

Publish:

- `ETH_SEPOLIA_RPC_URL`
- `PRIVATE_KEY`
- `ETH_SEPOLIA_CHAIN_ID`
- `PUBLISH_SETTLEMENT_REGISTRY`
- `PUBLISH_PUBLISHER_ADDRESS`

Internal auth:

- `INTERNAL_AUTH_ENABLED`
- `INTERNAL_AUTH_SECRET`

Wallet auth:

- `WALLET_AUTH_ENABLED`
- `WALLET_AUTH_NONCE_TTL_SECONDS`
- `WALLET_JWT_SECRET`
- `WALLET_JWT_TTL_SECONDS`
- `WALLET_ROLE_MAP`
- `WALLET_DEFAULT_ROLE`

## Testing

Main validation commands:

```bash
cd zk && cargo test
cd creworkflow && cargo test
cd api/proof-job-coordinator && cargo test
cd api/policy-snapshot && cargo test
cd api/compliance-attestation-adapter && cargo test
```

## Existing Component Docs

- Root concept: `flow.md`, `design.md`
- Contracts: `contracts/README.md`
- CRE workflow: `creworkflow/README.md`
- ZK pipeline: `zk/README.md`
- API flow: `api/flow.md`

## Where Chainlink Is Used

- CRE workflow orchestration spec: `creworkflow/workflow-spec/otc_settlement.yaml`
- CRE orchestration runtime and confidential HTTP call path: `creworkflow/orchestrator/src/main.rs`
- CRE simulation used in demo: `creworkflow/scripts/cre-cli-simulate.sh`
- External API ingestion for compliance (Frankfurter FX context): `api/compliance-attestation-adapter/src/service/confidential_http_service.rs`
- Compliance attestation path that consumes external data and persists provider evidence: `api/compliance-attestation-adapter/src/module/compliance_attestation/crud.rs`
- Proof job publish bridge from API to onchain publish module: `api/proof-job-coordinator/src/service/publish_service.rs`
- CRE publish-receipt onchain write to Sepolia SettlementRegistry: `creworkflow/publish-receipt/src/chain.rs`

## Notes

- Use Sepolia for the primary demo path.
- Keep policy versions and proof public signals deterministic across workflow and onchain verification.
- Prefer idempotent requests and replay-safe keys for all write APIs.
