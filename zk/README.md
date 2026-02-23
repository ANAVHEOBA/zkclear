# zk

Shared ZK plumbing for:
- `settlement_valid.circom`
- `compliance_valid.circom`
- `rebate_valid.circom`

## Artifacts Produced

Per circuit under `artifacts/<circuit>/`:
- `<circuit>.r1cs`
- `<circuit>_js/<circuit>.wasm`
- `<circuit>.wtns`
- `<circuit>.zkey`
- `<circuit>.vkey.json`
- `<circuit>.proof.json`
- `<circuit>.public.json`
- `<circuit>.verifier.sol`

Trusted setup PTAU is stored in:
- `artifacts/ptau/`

## Deterministic Fixtures

Fixture inputs (stable):
- `fixtures/settlement_valid.fixture.json`
- `fixtures/compliance_valid.fixture.json`
- `fixtures/rebate_valid.fixture.json`

Happy/fail vectors:
- `test-vectors/*.pass.json`
- `test-vectors/*.fail.json`

`settlement_valid` public signal order used onchain:
1) `policy_version_public`
2) `receipt_hash_public`
3) `domain_separator_public`
4) `workflow_run_id_public`
5) `binding_hash_public`
6) `notional_public`

## Scripts

- `scripts/compile.sh <circuit>`
- `scripts/setup.sh <circuit>`
- `scripts/prove.sh <circuit> [input_json]`
- `scripts/verify.sh <circuit>`
- `scripts/export_verifier.sh <circuit>`
- `scripts/pipeline.sh <circuit> [input_json]`
- `scripts/ci-check.sh [circuit ...]`
- `scripts/regression.sh [fast|full]`

Defaults use `settlement_valid`.

## Typical Flow (Settlement)

```bash
cd /home/a/zkclear/zk
./scripts/compile.sh settlement_valid
./scripts/setup.sh settlement_valid
./scripts/prove.sh settlement_valid fixtures/settlement_valid.fixture.json
./scripts/verify.sh settlement_valid
./scripts/export_verifier.sh settlement_valid
```

Or one-shot:

```bash
./scripts/pipeline.sh settlement_valid fixtures/settlement_valid.fixture.json
```

## Tooling Notes

- `snarkjs` is invoked via local binary or `npx` fallback.
- `circom` is invoked from local binary if installed.
- If local `circom` is missing, scripts attempt Docker image `ghcr.io/iden3/circom:<version>`.
- Set `CIRCOM_VERSION` to pin image tag (default `2.1.6`).
- `PTAU_POWER` defaults to `12` for fast local proving.

## Rust Constraint Tests

Fast local checks without circom compile:

```bash
cargo test
```

This validates the same business constraints against fixture vectors.

## Dev Quality Gates

Constraint budget + proving time checks:

```bash
cd /home/a/zkclear/zk
./scripts/ci-check.sh
```

Per-circuit run:

```bash
./scripts/ci-check.sh settlement_valid
```

Budget env knobs:
- `MAX_NONLINEAR_SETTLEMENT_VALID`
- `MAX_LINEAR_SETTLEMENT_VALID`
- `MAX_PROVE_SECONDS_SETTLEMENT_VALID`
- `MAX_NONLINEAR_COMPLIANCE_VALID`
- `MAX_LINEAR_COMPLIANCE_VALID`
- `MAX_PROVE_SECONDS_COMPLIANCE_VALID`
- `MAX_NONLINEAR_REBATE_VALID`
- `MAX_LINEAR_REBATE_VALID`
- `MAX_PROVE_SECONDS_REBATE_VALID`
- `FORCE_SETUP=1` (optional, forces fresh setup in `ci-check.sh`)

Circuit regression runs:

```bash
./scripts/regression.sh fast
./scripts/regression.sh full
```
