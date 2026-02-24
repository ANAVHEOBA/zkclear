"use client";

import { useEffect, useMemo, useState } from "react";
import {
  fetchProofJobsByRun,
  type ProofJobTrackerView,
  submitDealerIntents,
  type StartOtcOrchestrationResponse,
} from "@/lib/api/intent-desk";
import { buildIntentPayload } from "@/lib/crypto/intent-builder";

type Side = "left" | "right";

interface IntentFormState {
  counterpartyId: string;
  country: string;
  walletAddress: string;
  side: "buy" | "sell";
  assetPair: string;
  amount: string;
  limitPrice: string;
  settlementCurrency: string;
}

function defaultSide(prefix: string, side: "buy" | "sell"): IntentFormState {
  return {
    counterpartyId: `${prefix.toUpperCase()}-CP`,
    country: "US",
    walletAddress: "",
    side,
    assetPair: "ETH/USDC",
    amount: "100000",
    limitPrice: "2800",
    settlementCurrency: "USDC",
  };
}

export function IntentDeskPanel() {
  const [left, setLeft] = useState<IntentFormState>(() => defaultSide("dealer-a", "buy"));
  const [right, setRight] = useState<IntentFormState>(() => defaultSide("dealer-b", "sell"));
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<StartOtcOrchestrationResponse | null>(null);
  const [proofJob, setProofJob] = useState<ProofJobTrackerView | null>(null);
  const [proofError, setProofError] = useState<string | null>(null);
  const [proofLoading, setProofLoading] = useState(false);

  const intakeSummary = useMemo(() => {
    if (!result) {
      return null;
    }
    const acceptedCount = result.intent_submissions.filter((i) => i.accepted).length;
    return `${acceptedCount}/${result.intent_submissions.length} intents accepted`;
  }, [result]);

  const matchSummary = useMemo(() => {
    if (!result) {
      return null;
    }
    const acceptedCount = result.intent_submissions.filter((i) => i.accepted).length;
    const commitmentCount = result.intent_submissions.reduce(
      (sum, item) => sum + item.commitment_hashes.length,
      0,
    );
    const intentCount = result.intent_submissions.reduce(
      (sum, item) => sum + item.intent_ids.length,
      0,
    );
    return {
      acceptedCount,
      total: result.intent_submissions.length,
      commitmentCount,
      intentCount,
    };
  }, [result]);

  const complianceSummary = useMemo(() => {
    if (!result?.compliance_results?.length) {
      return null;
    }
    const passCount = result.compliance_results.filter((r) => r.passed).length;
    return `${passCount}/${result.compliance_results.length} parties passed`;
  }, [result]);

  const proofTimeline = useMemo(() => {
    if (!proofJob?.transitions?.length) {
      return [];
    }
    return [...proofJob.transitions].sort((a, b) => a.transitioned_at - b.transitioned_at);
  }, [proofJob]);

  useEffect(() => {
    const workflowRunId = result?.workflow_run_id;
    const shouldTrack = Boolean(workflowRunId && result?.proof_job?.accepted);
    if (!shouldTrack || !workflowRunId) {
      setProofJob(null);
      setProofError(null);
      setProofLoading(false);
      return;
    }

    let cancelled = false;
    let intervalId: ReturnType<typeof setInterval> | null = null;

    const load = async () => {
      if (!cancelled) {
        setProofLoading(true);
      }
      try {
        const payload = await fetchProofJobsByRun(workflowRunId);
        const job = payload.jobs?.[0] || null;
        if (!cancelled) {
          setProofJob(job);
          setProofError(null);
          if (
            job &&
            (job.status === "PUBLISHED" || job.status === "FAILED") &&
            intervalId !== null
          ) {
            clearInterval(intervalId);
            intervalId = null;
          }
        }
      } catch (err) {
        if (!cancelled) {
          setProofError(err instanceof Error ? err.message : "proof tracker fetch failed");
        }
      } finally {
        if (!cancelled) {
          setProofLoading(false);
        }
      }
    };

    void load();
    intervalId = setInterval(() => {
      void load();
    }, 3000);

    return () => {
      cancelled = true;
      if (intervalId !== null) {
        clearInterval(intervalId);
      }
    };
  }, [result?.proof_job?.accepted, result?.workflow_run_id]);

  async function handleSubmit() {
    setLoading(true);
    setError(null);
    setResult(null);
    setProofJob(null);
    setProofError(null);
    setProofLoading(false);
    try {
      const builtLeft = await buildIntentPayload({
        side: left.side,
        asset_pair: left.assetPair,
        amount: left.amount,
        limit_price: left.limitPrice,
        settlement_currency: left.settlementCurrency,
        counterparty_id: left.counterpartyId,
      });
      const builtRight = await buildIntentPayload({
        side: right.side,
        asset_pair: right.assetPair,
        amount: right.amount,
        limit_price: right.limitPrice,
        settlement_currency: right.settlementCurrency,
        counterparty_id: right.counterpartyId,
      });

      const response = await submitDealerIntents(
        {
          ...builtLeft,
          counterpartyId: left.counterpartyId,
          country: left.country || undefined,
          walletAddress: left.walletAddress || undefined,
        },
        {
          ...builtRight,
          counterpartyId: right.counterpartyId,
          country: right.country || undefined,
          walletAddress: right.walletAddress || undefined,
        },
      );
      setResult(response);
      if (!response.accepted) {
        const prefix = response.error_code ? `[${response.error_code}] ` : "";
        setError(`${prefix}${response.reason || "compliance gate blocked orchestration"}`);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "intent submission failed");
    } finally {
      setLoading(false);
    }
  }

  function update(side: Side, key: keyof IntentFormState, value: string) {
    if (side === "left") {
      setLeft((prev) => ({ ...prev, [key]: value }));
      return;
    }
    setRight((prev) => ({ ...prev, [key]: value }));
  }

  return (
    <section id="intents" className="intent-desk-panel">
      <div className="intent-desk-head">
        <p>Dealer Intent Desk</p>
        <span>Plain input only. Encryption/signing auto-generated on submit.</span>
      </div>
      <div className="intent-grid">
        <div className="intent-card">
          <h3>Counterparty A</h3>
          <input
            placeholder="counterparty_id"
            value={left.counterpartyId}
            onChange={(e) => update("left", "counterpartyId", e.target.value)}
          />
          <select value={left.side} onChange={(e) => update("left", "side", e.target.value)}>
            <option value="buy">buy</option>
            <option value="sell">sell</option>
          </select>
          <input
            placeholder="asset_pair"
            value={left.assetPair}
            onChange={(e) => update("left", "assetPair", e.target.value)}
          />
          <input
            placeholder="amount"
            value={left.amount}
            onChange={(e) => update("left", "amount", e.target.value)}
          />
          <input
            placeholder="limit_price"
            value={left.limitPrice}
            onChange={(e) => update("left", "limitPrice", e.target.value)}
          />
          <input
            placeholder="settlement_currency"
            value={left.settlementCurrency}
            onChange={(e) => update("left", "settlementCurrency", e.target.value)}
          />
        </div>
        <div className="intent-card">
          <h3>Counterparty B</h3>
          <input
            placeholder="counterparty_id"
            value={right.counterpartyId}
            onChange={(e) => update("right", "counterpartyId", e.target.value)}
          />
          <select value={right.side} onChange={(e) => update("right", "side", e.target.value)}>
            <option value="buy">buy</option>
            <option value="sell">sell</option>
          </select>
          <input
            placeholder="asset_pair"
            value={right.assetPair}
            onChange={(e) => update("right", "assetPair", e.target.value)}
          />
          <input
            placeholder="amount"
            value={right.amount}
            onChange={(e) => update("right", "amount", e.target.value)}
          />
          <input
            placeholder="limit_price"
            value={right.limitPrice}
            onChange={(e) => update("right", "limitPrice", e.target.value)}
          />
          <input
            placeholder="settlement_currency"
            value={right.settlementCurrency}
            onChange={(e) => update("right", "settlementCurrency", e.target.value)}
          />
        </div>
      </div>

      <div className="intent-actions">
        <button type="button" disabled={loading} onClick={() => void handleSubmit()}>
          {loading ? "Submitting..." : "Submit Both Intents"}
        </button>
      </div>

      {result ? (
        <div className="intent-result">
          <p>
            workflow_run_id: <strong>{result.workflow_run_id}</strong>
          </p>
          <p>
            policy_version: <strong>{result.policy_version}</strong>
          </p>
          <p>
            policy_hash: <strong>{result.policy_hash || "unavailable"}</strong>
          </p>
          <p>
            intake status: <strong>{intakeSummary}</strong>
          </p>
          {matchSummary ? (
            <p>
              match summary:{" "}
              <strong>
                accepted {matchSummary.acceptedCount}/{matchSummary.total}, intents{" "}
                {matchSummary.intentCount}, commitments {matchSummary.commitmentCount}
              </strong>
            </p>
          ) : null}
          <p>
            proof job:{" "}
            <strong>
              {result.proof_job?.accepted
                ? `queued (${result.proof_job.job_id})`
                : "not started (blocked before proving)"}
            </strong>
          </p>
        </div>
      ) : null}

      <div id="compliance" className="intent-result proof-tracker">
        <p>
          Compliance Result:{" "}
          <strong>{complianceSummary || "waiting for orchestration response"}</strong>
        </p>
        {result?.compliance_results?.length ? (
          <div>
            {result.compliance_results.map((item) => (
              <p key={item.subject_id}>
                {item.subject_id}: <strong>{item.passed ? "PASS" : "FAIL"}</strong>{" "}
                {!item.passed ? `(${item.reason_code || item.decision})` : ""}
              </p>
            ))}
          </div>
        ) : (
          <p>No compliance attestation yet.</p>
        )}
      </div>

      <div id="proof-jobs" className="intent-result proof-tracker">
        <p>
          proof job id:{" "}
          <strong>{result?.proof_job?.job_id || "not started"}</strong>
        </p>
        {result?.proof_job?.accepted ? (
          <>
          <p>
            Proof Pipeline Tracker:{" "}
            <strong>{proofLoading ? "refreshing..." : "live"}</strong>
          </p>
          <p>
            current status: <strong>{proofJob?.status || "QUEUED"}</strong>
          </p>
          <p>
            attempts: <strong>{proofJob?.attempt_count ?? 0}</strong>
          </p>
          <p>
            retries: <strong>{proofJob?.retry_count ?? 0}</strong>
            {" / "}scheduled:{" "}
            <strong>{proofJob?.retry_scheduled ? "yes" : "no"}</strong>
          </p>
          <p>
            queue latency:{" "}
            <strong>
              {typeof proofJob?.queue_latency_ms === "number"
                ? `${proofJob.queue_latency_ms} ms`
                : "pending"}
            </strong>
          </p>
          <p>
            prove duration:{" "}
            <strong>
              {typeof proofJob?.prove_duration_ms === "number"
                ? `${proofJob.prove_duration_ms} ms`
                : "pending"}
            </strong>
          </p>
          {proofTimeline.length ? (
            <div className="proof-tracker-timeline">
              {proofTimeline.map((item, idx) => (
                <p key={`${item.to_status}-${item.transitioned_at}-${idx}`}>
                  {item.to_status} @ {new Date(item.transitioned_at * 1000).toISOString()}
                  {item.error_code ? ` [${item.error_code}]` : ""}
                </p>
              ))}
            </div>
          ) : (
            <p>timeline: waiting for transitions...</p>
          )}
          {proofJob?.last_error_code ? (
            <p>
              last error:{" "}
              <strong>
                {proofJob.last_error_code}
                {proofJob.last_error_message ? ` - ${proofJob.last_error_message}` : ""}
              </strong>
            </p>
          ) : null}
          </>
        ) : (
          <p>Proof pipeline not started yet.</p>
        )}
        {proofError ? <p className="intent-error">{proofError}</p> : null}
      </div>

      {error ? <p className="intent-error">{error}</p> : null}
    </section>
  );
}
