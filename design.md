Full Scope Stack

  - Chainlink: CRE workflows, Confidential Compute, Confidential HTTP
  - ZK: Circom + SnarkJS (Groth16) for hackathon-speed proving
  - Contracts: Solidity + Foundry
  - Backend: Node.js + TypeScript (Fastify), libsodium for encryption
  - Data: Postgres (intents, attestations, run logs), Redis (job queue)
  - Frontend: Next.js + viem/wagmi
  - Infra: Docker Compose for local, optional cloud deploy for demo

  System Modules

  1. zk/

  - compliance_valid.circom
  - settlement_valid.circom
  - rebate_valid.circom
  - witness/prover scripts + test vectors

  2. contracts/

  - Verifier.sol (Groth16 verifier)
  - SettlementRegistry.sol (minimal public receipts)
  - PolicyManager.sol (policy hash/version commitments)

  3. cre-workflows/

  - intent-intake
  - confidential-match
  - proof-generate
  - settle-private
  - publish-receipt

  4. api/

  - encrypted intent gateway
  - compliance attestation adapter
  - policy snapshot service
  - proof job coordinator

  5. ui/

  - dealer intent submit
  - ops queue + run status
  - compliance view (private)
  - public audit receipt explorer

  Execution Order (critical)

  1. Contracts + receipt schema finalized.
  2. CRE workflow skeleton runs end-to-end with mocked proof.
  3. Confidential HTTP integration with real external API key.
  4. Confidential Compute matching + policy checks.
  5. Add settlement_valid proof and onchain verification.
  6. Add compliance_valid.
  7. Add rebate_valid.
  8. Private settlement + rebate orchestration.
  9. UI hardening + demo script + README evidence mapping.

  Proof/Public Data Boundary

  - Public onchain: workflowRunId, proofHash, policyVersion, status, timestamp, receiptHash
  - Private offchain: identities, notionals, pricing terms, API creds, raw compliance/risk responses

  7-Min Demo Flow

  1. Submit two encrypted intents.
  2. Show CRE run kickoff.
  3. Show Confidential HTTP call success (key hidden).
  4. Show confidential match + policy pass.
  5. Generate/verify ZK proof onchain.
  6. Execute private settlement + rebate.
  7. Show minimal public receipt + audit trace.

  Team Split (if 3–4 people)

  1. ZK + verifier/contracts integration
  2. CRE workflows + confidential services
  3. API/backend + data model
  4. UI + demo tooling + docs/video

  Non-negotiables for judges

  - CRE simulation/deploy visible in video
  - Confidential HTTP visibly used
  - Public repo with runnable instructions
  - README section: “Where Chainlink is used” with file links
  - Clear threat model and privacy table
