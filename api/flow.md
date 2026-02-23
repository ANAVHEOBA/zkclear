
  1. api/encrypted-intent-gateway (first)
  2. api/compliance-attestation-adapter
  3. api/policy-snapshot-service
  4. api/proof-job-coordinator (ties zk proving + publish-receipt together)


api/encrypted-intent-gateway

  - Accept encrypted buy/sell intents from client/desk.
  - Verify signature, timestamp, nonce, and schema.
  - Decrypt only inside confidential runtime boundary.
  - Store encrypted payload + commitment hashes + metadata in DB.
  - Return workflowRunId, intentIds, and acceptance/rejection reason.
  - Prevent replay (nonce + intent hash uniqueness).

  api/compliance-attestation-adapter

  - Call external compliance/risk providers (sanctions/KYC/risk score).
  - Normalize responses into one internal attestation format.
  - Produce attestation hash/commitment for downstream proof binding.
  - Cache provider responses with TTL and track provider outages.
  - Return pass/fail, risk score, policy flags, and attestation reference.

  api/policy-snapshot-service

  - Serve immutable policy snapshot by version/hash.
  - Map active onchain policy version to offchain rule bundle.
  - Return deterministic rule set used for a run (for audit reproducibility).
  - Keep version history + activation windows.
  - Expose “effective policy for run X” endpoint.

  api/proof-job-coordinator

  - Orchestrate async job: match result -> zk input -> proof generation -> publish receipt.
  - Build circuit inputs and bind run/policy/receipt/domain consistently.
  - Call zk prover, validate outputs, compute proofHash/receiptHash.
  - Submit to publish-receipt module and persist tx/event result.
  - Handle retries, dead-letter queue, idempotency, and job status API.