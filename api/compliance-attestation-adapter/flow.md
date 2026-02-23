api/compliance-attestation-adapter feature scope should be:

  1. Intake + normalization

  - Accept compliance/risk check requests from workflow
  - Normalize counterparty/entity payloads into one internal schema
  - Validate required fields, timestamps, and request IDs

  2. Confidential external API integration

  - Call third-party compliance/risk APIs via confidential path
  - Keep API keys/secrets out of logs and responses
  - Support retries, timeout, and provider failover policy

  3. Attestation generation

  - Produce deterministic attestation output:
  - attestation_id
  - workflow_run_id
  - policy_version
  - decision (PASS/REVIEW/FAIL)
  - risk_score
  - attestation_hash
  - issued_at/expires_at

  4. Hash/commitment binding

  - Hash canonicalized response + policy version + run context
  - Return attestation_hash for downstream compliance_valid binding
  - Prevent tampering between API step and proof step

  5. Policy snapshot check

  - Read active policy snapshot/version and evaluate thresholds
  - Enforce “decision came from approved policy version”
  - Record policy hash/version used in attestation

  6. Persistence layer

  - Mongo:
  - request records
  - provider raw response references (encrypted/redacted)
  - final attestation documents
  - Redis:
  - idempotency keys
  - short-lived cache for repeated checks
  - job/status keys

  7. Idempotency + replay protection

  - Same request key should not produce conflicting attestations
  - Reject stale/replayed requests by nonce/request hash
  - Deterministic response for duplicate idempotent calls

  8. Security controls

  - Signature verification for internal callers (if required)
  - Strict PII minimization in logs
  - Encryption at rest for sensitive fields
  - Audit trail events for each decision transition

  9. API surface (controller + CRUD separation)

  - POST /v1/compliance/attest
  - GET /v1/compliance/attest/:id
  - GET /v1/compliance/health
  - Controller = HTTP only, CRUD/service = business logic

  10. Error codes + observability

  - Typed error codes (PROVIDER_TIMEOUT, POLICY_MISMATCH, RISK_THRESHOLD_FAIL, etc.)
  - Structured logs with correlation/workflow IDs
  - Metrics: pass/fail/review counts, latency, provider failure rate

  11. CRE/workflow integration contract

  - Output format compatible with confidential-match and proof-generate
  - Includes exact fields needed by compliance_valid circuit:
  - pass bit
  - risk score
  - policy version
  - attestation hash
  - optional sanction/allowlist commitments

  12. Test coverage

  - Unit tests: policy logic + hashing + idempotency
  - Integration tests: real HTTP route + Mongo/Redis
  - Failure-path tests: API unavailable, timeout, malformed provider response