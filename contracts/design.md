1. Verifier module
     What it does:

  - Takes ZK proof + public inputs.
  - Returns pass/fail only.
  - Guarantees math correctness of your offchain claims without revealing private data.

  Why it exists:

  - It is the cryptographic gate before anything is written onchain.

  2. PolicyManager module
     What it does:

  - Stores committed policy versions as hashes.
  - Marks which version is active.
  - Lets registry check “was this settlement validated under an approved policy version?”

  Why it exists:

  - Prevents invisible rule changes and gives policy auditability over time.

  3. SettlementRegistry module
     What it does:

  - Final onchain recorder.
  - Accepts only authorized workflow submissions.
  - Verifies:
      - policy version is active,
      - proof is valid (via Verifier),
      - run is not duplicate.
  - Stores minimal public receipt and emits audit event.

  Why it exists:

  - It is the canonical public evidence layer.

  4. AccessControl module
     What it does:

  - Defines who can:
      - publish receipts (workflow role),
      - update policies (policy admin),
      - upgrade verifier pointers/admin tasks.
  - Supports pause/unpause.

  Why it exists:

  - Prevents unauthorized writes and supports incident response.

  5. Replay/Uniqueness logic
     What it does:

  - Ensures each workflowRunId (and/or receiptHash) can only be finalized once.

  Why it exists:

  - Stops duplicate or replayed settlements.

  6. Signal Binding logic
     What it does:

  - Ensures proof’s public inputs are bound to exact onchain metadata (policyVersion, receiptHash, domain separator).

  Why it exists:

  - Prevents valid proof reuse against different receipts.