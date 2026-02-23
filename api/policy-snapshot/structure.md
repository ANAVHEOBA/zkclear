Features for api/policy-snapshot

  1. Snapshot registry

  - Store immutable policy snapshots keyed by policy_version and policy_hash.
  - Reject updates to an existing version/hash payload.

  2. Active policy mapping

  - Track which policy version is currently active.
  - Map onchain active policy version to offchain rule bundle metadata.

  3. Deterministic retrieval

  - Return canonical, deterministic JSON for a version/hash.
  - Include content hash in response for reproducibility checks.

  4. Version history + activation windows

  - Keep timeline: version, hash, activated_at, deactivated_at.
  - Support querying policy active at a specific timestamp.

  5. Effective policy for run

  - Endpoint to resolve “policy used for run X” from run metadata (run_id, timestamp, optional version hint).
  - Return exact snapshot and activation record used.

  6. Audit reproducibility

  - Persist immutable record linking run -> resolved policy hash/version.
  - Return signed/hashed evidence fields for audit trail.

  7. Validation + integrity

  - Validate schema of rule bundle.
  - Verify policy_hash == hash(canonical_rule_bundle).

  8. API surface

  - POST /v1/policy/snapshots (create immutable snapshot)
  - POST /v1/policy/activate (activate version window)
  - GET /v1/policy/snapshots/:version
  - GET /v1/policy/snapshots/hash/:policy_hash
  - GET /v1/policy/active
  - GET /v1/policy/effective/:run_id

  9. Storage

  - Mongo: snapshots, activation history, run-policy resolution records.
  - Redis: cache active snapshot + hot lookup keys.

  10. Security

  - Internal auth signature/header check.
  - Strict audit logging (no mutable history rewrites).

  ———

  Suggested structure

  api/policy-snapshot/
  ├── Cargo.toml
  ├── .env.example
  ├── src/
  │   ├── main.rs
  │   ├── lib.rs
  │   ├── app.rs
  │   ├── config/
  │   │   ├── mod.rs
  │   │   ├── environment.rs
  │   │   └── db.rs
  │   ├── infra/
  │   │   ├── mod.rs
  │   │   ├── mongo.rs
  │   │   └── redis.rs
  │   ├── service/
  │   │   ├── mod.rs
  │   │   ├── hash_service.rs
  │   │   ├── canonical_json_service.rs
  │   │   ├── activation_service.rs
  │   │   ├── effective_policy_service.rs
  │   │   ├── idempotency_service.rs
  │   │   └── signature_service.rs
  │   └── module/
  │       └── policy_snapshot/
  │           ├── mod.rs
  │           ├── model.rs
  │           ├── schema.rs
  │           ├── error.rs
  │           ├── crud.rs
  │           ├── controller.rs
  │           └── route.rs
  └── tests/
      ├── create_snapshot.rs
      ├── activate_policy.rs
      ├── get_effective_policy.rs
      └── immutability_enforcement.rs
