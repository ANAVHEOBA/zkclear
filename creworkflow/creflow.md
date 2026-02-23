Primary workflow sequence

  1. intent-intake
  2. confidential-match
  3. proof-generate
  4. settle-private
  5. publish-receipt

  ———

  1) intent-intake

  - Input: two encrypted intents (asset pair, side, size, limits, expiry, counterparty constraints).
  - Action:
      - decrypt in confidential environment only,
      - validate schema/signature/timestamp/nonce,
      - create workflowRunId,
      - store encrypted payload reference (not raw in logs).
  - Output:
      - normalized private intent objects,
      - workflowRunId,
      - intent commitment hashes.
  - Fail conditions:
      - invalid signature, stale nonce, expired intent, malformed payload.

  ———

  2) confidential-match

  - Input: normalized intents + policy version + external risk/compliance signals.
  - Action:
      - call external compliance/risk API via Confidential HTTP,
      - apply private matching rules (price/size/limits),
      - apply policy checks tied to policyVersion.
  - Output:
      - matchDecision (accept/reject),
      - private settlement params,
      - policyCheckResult,
      - compliance attestation hash.
  - Fail conditions:
      - API unavailable, policy mismatch, risk threshold fail, no match.

  ———

  3) proof-generate

  - Input: accepted match result + policy result + settlement params.
  - Action:
      - generate ZK witness/proof offchain,
      - compute public commitment fields for onchain:
          - receiptHash,
          - proofHash,
          - bound policyVersion,
          - domain separator binding.
  - Output:
      - proof bytes,
      - public signals,
      - proofHash,
      - receiptHash.
  - Fail conditions:
      - circuit constraint failure, witness generation failure, proving timeout.

  ———

  4) settle-private

  - Input: approved proof bundle + private settlement instructions.
  - Action:
      - execute private token movement / private settlement leg(s),
      - record confidential execution status and private audit refs.
  - Output:
      - settlement status (SETTLED or FAILED),
      - private execution reference IDs.
  - Fail conditions:
      - transfer failure, counterparty settlement conflict, timeout/retry exhausted.

  ———

  5) publish-receipt

  - Input:
      - workflowRunId,
      - proofHash,
      - policyVersion,
      - status,
      - receiptHash,
      - proof + public signals.
  - Action:
      - call deployed SettlementRegistry.publishReceipt(...) on Sepolia,
      - contract enforces:
          - authorized publisher,
          - active policy,
          - proof valid,
          - signal binding valid,
          - no replay (workflowRunId / receiptHash).
  - Output:
      - onchain receipt event + stored receipt record.
  - Fail conditions:
      - invalid proof, stale policy, duplicate run/hash, unauthorized caller.

  ———

  Privacy boundary (must stay explicit in docs/video)

  - Private/offchain:
      - identities, notionals, terms, API credentials, raw API responses, matching details.
  - Public/onchain:
      - workflowRunId, proofHash, policyVersion, status, timestamp, receiptHash.

  ———

  Operational controls

  - Retries for transient API/network failures.
  - Deterministic idempotency key = workflowRunId.
  - Dead-letter queue for failed runs.
  - Full run trace with redacted logs only.
  - Pause switch respected before onchain publish.

  ———

  What to show in demo

  1. Submit two encrypted intents.
  2. Trigger CRE workflow.
  3. Show Confidential HTTP call executed (without revealing key).
  4. Show match + policy pass in confidential step.
  5. Show proof generation.
  6. Show Sepolia publishReceipt tx success.
  7. Show onchain receipt containing only minimal fields.

  ———



  Still needed:

  1. Add a top-level cre-workflows Cargo workspace (single workspace, per-step crates).
  2. Wire step-to-step orchestration (actual CRE workflow spec + execution order).
  3. Add CRE CLI simulation script that runs end-to-end and shows success/failure paths.
  4. Connect confidential API call path in orchestration (not just module-local logic).
  5. Add end-to-end runbook + demo script + README mapping each step to Chainlink usage.
  6. Add one full E2E test artifact (inputs/outputs across all 5 steps).