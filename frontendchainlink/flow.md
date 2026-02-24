1. Login + Role Gate

  - Dealer, Ops, Compliance views (same app, role-based panels).

  2. Intent Desk (Dealer)

  - Create encrypted buy/sell intents.
  - Submit both counterparties.
  - Show workflow_run_id and intake status.

  3. Match + Policy Snapshot

  - Trigger OTC orchestration (POST /v1/orchestrations/otc).
  - UI shows selected policy version/hash and match summary.

  4. Compliance Result

  - Show pass/fail attestation per party.
  - If fail: reason code + stop flow.
  - If pass: continue automatically to proof job.

  5. Proof Pipeline Tracker (Ops)

  - Live state timeline:
      - QUEUED -> PROVING -> PROVED -> PUBLISHING -> PUBLISHED (or FAILED)
  - Show retries, attempt count, prove duration, queue latency.

  6. Onchain Receipt Panel

  - Tx hash, block number, chain (Sepolia), receipt hash/proof hash.
  - Link to explorer.
  - Immutable audit evidence view.

  7. History + Search

  - Query by workflow_run_id / job_id.
  - Filter by proof type (settlement, compliance, rebate) and status.

  8. Health/Monitoring (Ops)

  - Service health cards for each backend API.
  - Queue depth + recent failures.
  - Quick retry button for failed jobs.