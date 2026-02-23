# publish-receipt

Rust workflow module for the final step:
- publish settlement proof metadata to `SettlementRegistry.publishReceipt(...)`
- return tx/event info plus stored receipt payload

## What It Does

Input:
- `workflow_run_id`
- `proof_hash`
- `policy_version`
- `status`
- `receipt_hash`
- `proof_hex`
- `public_signals`

Action:
- validates input shape
- in onchain mode: sends `publishReceipt(...)` to deployed `SettlementRegistry`
- in simulation mode: runs local contract-style checks

Output:
- `tx_hash`
- `onchain_receipt_event_id`
- `stored_receipt_record`

## Project Structure

- `src/main.rs`: CLI entrypoint (`stdin` JSON -> `stdout` JSON)
- `src/chain.rs`: real Sepolia onchain publishing (ethers-rs)
- `src/handler.rs`: simulation path
- `src/models.rs`: request/response models
- `src/errors.rs`: error types
- `tests/publish_receipt_integration.rs`: integration tests

## Modes

### 1) Onchain Mode

Enabled when both env vars exist:
- `ETH_SEPOLIA_RPC_URL`
- `PRIVATE_KEY`

Optional:
- `ETH_SEPOLIA_CHAIN_ID` (default `11155111`)

### 2) Simulation Mode

Used when neither `ETH_SEPOLIA_RPC_URL` nor `PRIVATE_KEY` is set.

## Build and Test

```bash
cargo build
cargo test
```

## Run (Onchain)

```bash
cd /home/a/zkclear/creworkflow/publish-receipt
set -a; source /home/a/zkclear/contracts/.env; set +a
cargo run --quiet < /tmp/publish_receipt_input.json
```

## Example Input JSON

```json
{
  "settlement_registry": "0x3e3a14f46d13e156daa99bf234224a57b1c79da5",
  "publisher_address": "0x6D21167d874C842386e8c484519B5ddBBaB87b43",
  "workflow_run_id": "0x1111111111111111111111111111111111111111111111111111111111111111",
  "proof_hash": "0x3333333333333333333333333333333333333333333333333333333333333333",
  "policy_version": 1,
  "status": "settled",
  "receipt_hash": "0x2222222222222222222222222222222222222222222222222222222222222222",
  "proof_hex": "0xc0ffee",
  "public_signals": [
    "0x2c9d671801df8638be5a7d72b780d2aa06aed483d12bfb6d396285bff515f656"
  ],
  "chain_validation": {
    "authorized_publisher": true,
    "policy_active": true,
    "proof_valid": true,
    "signal_binding_valid": true,
    "duplicate_workflow_run": false,
    "duplicate_receipt_hash": false
  }
}
```

## Notes

- `chain_validation` is used only by simulation path.
- Onchain path enforces actual contract checks.
- If transaction reverts, this module returns an onchain error.
- `public_signals` must follow settlement circuit order:
  1) `policy_version_public`
  2) `receipt_hash_public` (uint64 projection)
  3) `domain_separator_public` (uint64 projection)
  4) `workflow_run_id_public` (uint64 projection)
  5) `binding_hash_public`
  6) `notional_public`
- `proof_hex` must be `abi.encode(uint256[2] pA, uint256[2][2] pB, uint256[2] pC)`.
