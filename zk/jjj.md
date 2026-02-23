zk/ full feature list (for your scope):

  1. settlement_valid.circom (first, required)

  - Conservation check (in = out + fees)
  - Positive amount bounds
  - Price/size notional consistency
  - Max notional / limit checks
  - Policy version binding
  - Receipt hash binding
  - Domain separator binding
  - Public signal layout fixed for onchain verifier

  2. compliance_valid.circom (next)

  - Compliance pass bit enforcement
  - Risk score threshold check
  - Policy version binding
  - Attestation hash binding
  - Optional sanction/allowlist commitments

  3. rebate_valid.circom (next)

  - Rebate formula correctness
  - Fee split correctness
  - Bound checks (no negative/overflow equivalent)
  - Policy version binding
  - Recipient commitment binding

  4. Shared zk plumbing

  - Trusted setup artifacts (.zkey, .vkey.json)
  - wasm + witness generator outputs
  - Prover script (generate proof/public signals)
  - Verifier export for Solidity
  - Test vectors (happy + fail cases)
  - Deterministic fixture inputs

  5. Integration features

  - Public signals mapped exactly to publishReceipt flow
  - Verifier compatibility with your Verifier.sol
  - End-to-end proof generation from proof-generate step
  - Replay-safe commitment strategy (bind run/policy/receipt/domain)

  6. Dev quality

  - Constraint count budget tracking
  - Proving-time target checks
  - Circuit regression tests
  - README for how to generate/setup/prove/verify