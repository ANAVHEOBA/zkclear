# ZK-Clear CRE Workflows

This folder contains the full 5-step private settlement workflow implemented as Rust modules and orchestrated as a CRE-style pipeline.

## Steps

1. `intent-intake`
2. `confidential-match`
3. `proof-generate`
4. `settle-private`
5. `publish-receipt`

## Chainlink Mapping

| Step | Purpose | Chainlink Usage | Module |
|---|---|---|---|
| `intent-intake` | decrypt + validate encrypted intents | CRE orchestration step, confidential execution boundary | `intent-intake` |
| `confidential-match` | compliance/risk + matching rules | Confidential HTTP (external API fetch) + Confidential Compute logic | `confidential-match` |
| `proof-generate` | witness/proof generation + commitments | Confidential Compute offchain proving path | `proof-generate` |
| `settle-private` | private transfer legs + confidential audit refs | CRE private execution step | `settle-private` |
| `publish-receipt` | onchain publish to settlement registry | CRE EVM write action to Sepolia `SettlementRegistry.publishReceipt(...)` | `publish-receipt` |

## Workspace

- Root workspace: `creworkflow/Cargo.toml`
- Orchestrator: `orchestrator` crate
- Workflow spec: `workflow-spec/otc_settlement.yaml`

## How To Run

### Build/check

```bash
cd /home/a/zkclear/creworkflow
cargo check -p orchestrator
```

### Simulate E2E (success + failures)

```bash
cd /home/a/zkclear/creworkflow
./scripts/cre-cli-simulate.sh
```

### Generate E2E artifact (step-by-step input/output)

```bash
cd /home/a/zkclear/creworkflow
./scripts/generate-e2e-artifact.sh
```

Artifacts are written to:
- `artifacts/e2e/full-run/`

## Runbook and Demo

- Runbook: `docs/E2E_RUNBOOK.md`
- Demo script: `scripts/demo.sh`

## Latest Sepolia Update

### Environment and Git Hygiene

- Added `creworkflow/.gitignore` to ignore:
  - `.env`, `.env.*` (except `.env.example`)
  - Rust `target/` outputs
  - editor/OS temp files
- Added `creworkflow/.env.sepolia` with current Sepolia RPC/key + deployed addresses.
- Added `creworkflow/.env` (copied from `.env.sepolia`) for local execution.

### Current Sepolia Deployment

- Chain: `Ethereum Sepolia (11155111)`
- `AccessController`: `0x67f9aa6f37fc36482c9a0b5f65e1ee28e3ce4409`
- `PolicyManager`: `0x49728d5c119c0497c2478cd54c63097ed47ce9e1`
- `Verifier`: `0xe866e60522ba58da0f65956d417402bc35a5d04b`
- `SettlementValidGroth16Verifier`: `0x91d60b0e89874c8371290443fb4967ff1ff23d55`
- `SignalBinding`: `0x753eaac5674e92631161a3b66b38f9cee2432d2a`
- `ReplayProtection`: `0x0d9c1384a207c2b8c8ef9a5b9cccf5eca7a82737`
- `SettlementRegistry`: `0x3e3a14f46d13e156daa99bf234224a57b1c79da5`

### Live `publish-receipt` Execution

- Submitted successfully from `creworkflow/publish-receipt`.
- Transaction hash: `0xc9fb3408c00283b374ef9cd54a9a09d9e8bbd1d34bee2a80fb9ab97439c04aa1`
- Event reference: `0xc9fb3408c00283b374ef9cd54a9a09d9e8bbd1d34bee2a80fb9ab97439c04aa1:1`
- Published record:
  - `workflow_run_id`: `run-1771797229`
  - `policy_version`: `1`
  - `proof_hash`: `0xd6e31b2af28e4ba085cf203c8ddc0c5847e418d96b4026bb2361c02471b5ab14`
  - `receipt_hash`: `0x4218fd3a6290cc85e8517766c702c95499bf91818cd9742171aa1b1ddd9c37dd`

All workflow references to the old settlement registry were updated to the new deployed address.
