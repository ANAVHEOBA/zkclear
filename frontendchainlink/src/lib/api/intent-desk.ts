import { getBackendBaseUrl } from "@/lib/api/config";
import { loadWalletSession } from "@/lib/auth/session";

export interface DealerIntentInput {
  encryptedPayload: string;
  signature: string;
  signerPublicKey: string;
  nonce: string;
  timestamp: number;
  counterpartyId: string;
  country?: string;
  walletAddress?: string;
}

export interface StartOtcOrchestrationResponse {
  accepted: boolean;
  workflow_run_id: string;
  policy_version: string;
  policy_hash: string;
  attestation_id: string;
  attestation_hash: string;
  intent_submissions: Array<{
    accepted: boolean;
    workflow_run_id: string;
    intent_ids: string[];
    commitment_hashes: string[];
    error_code?: string | null;
    reason: string;
  }>;
  compliance_results: Array<{
    subject_id: string;
    passed: boolean;
    decision: string;
    reason_code?: string | null;
  }>;
  proof_job?: {
    accepted: boolean;
    idempotent: boolean;
    replayed: boolean;
    job_id: string;
    workflow_run_id: string;
    policy_version: string;
    proof_type: string;
    error_code?: string | null;
    reason: string;
  } | null;
  error_code?: string | null;
  reason: string;
}

export interface ProofJobTransition {
  from_status?: string | null;
  to_status: string;
  transitioned_at: number;
  error_code?: string | null;
}

export interface ProofJobTrackerView {
  job_id: string;
  workflow_run_id: string;
  policy_version: string;
  proof_type: string;
  status: string;
  attempt_count?: number;
  retry_count?: number;
  retry_scheduled?: boolean;
  queue_latency_ms?: number | null;
  prove_duration_ms?: number | null;
  transitions: ProofJobTransition[];
  last_error_code?: string | null;
  last_error_message?: string | null;
}

export interface ProofJobsByRunResponse {
  found: boolean;
  workflow_run_id: string;
  jobs: ProofJobTrackerView[];
  error_code?: string | null;
  reason: string;
}

interface ApiErrorResponse {
  error_code?: string | null;
  reason?: string;
}

export async function submitDealerIntents(
  left: DealerIntentInput,
  right: DealerIntentInput,
): Promise<StartOtcOrchestrationResponse> {
  const session = loadWalletSession();
  if (!session?.accessToken) {
    throw new Error("wallet session missing; connect wallet and verify access first");
  }

  const body = {
    intents: [
      {
        encrypted_payload: left.encryptedPayload,
        signature: left.signature,
        signer_public_key: left.signerPublicKey,
        nonce: left.nonce,
        timestamp: left.timestamp,
      },
      {
        encrypted_payload: right.encryptedPayload,
        signature: right.signature,
        signer_public_key: right.signerPublicKey,
        nonce: right.nonce,
        timestamp: right.timestamp,
      },
    ],
    subjects: [
      {
        counterparty: {
          counterparty_id: left.counterpartyId,
          country: left.country || null,
          wallet_address: left.walletAddress || null,
        },
      },
      {
        counterparty: {
          counterparty_id: right.counterpartyId,
          country: right.country || null,
          wallet_address: right.walletAddress || null,
        },
      },
    ],
    proof_type: "settlement",
  };

  const response = await fetch(`${getBackendBaseUrl()}/v1/orchestrations/otc`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      authorization: `Bearer ${session.accessToken}`,
    },
    body: JSON.stringify(body),
  });

  const raw = await response.text();
  let payload: (StartOtcOrchestrationResponse & ApiErrorResponse) | null = null;
  try {
    payload = JSON.parse(raw) as StartOtcOrchestrationResponse & ApiErrorResponse;
  } catch {
    payload = null;
  }

  if (!response.ok) {
    if (payload?.reason) {
      const prefix = payload.error_code ? `[${payload.error_code}] ` : "";
      throw new Error(`${prefix}${payload.reason}`);
    }
    throw new Error(`orchestration failed (${response.status}): ${raw || "empty response"}`);
  }

  if (!payload) {
    throw new Error("orchestration returned non-JSON response");
  }

  if (!payload.accepted) {
    return payload;
  }
  return payload;
}

export async function fetchProofJobsByRun(
  workflowRunId: string,
): Promise<ProofJobsByRunResponse> {
  const session = loadWalletSession();
  const headers: Record<string, string> = {
    "content-type": "application/json",
  };
  if (session?.accessToken) {
    headers.authorization = `Bearer ${session.accessToken}`;
  }

  const response = await fetch(`${getBackendBaseUrl()}/v1/proof-jobs/run/${workflowRunId}`, {
    method: "GET",
    headers,
    cache: "no-store",
  });

  const raw = await response.text();
  let payload: (ProofJobsByRunResponse & ApiErrorResponse) | null = null;
  try {
    payload = JSON.parse(raw) as ProofJobsByRunResponse & ApiErrorResponse;
  } catch {
    payload = null;
  }

  if (!response.ok) {
    if (payload?.reason) {
      const prefix = payload.error_code ? `[${payload.error_code}] ` : "";
      throw new Error(`${prefix}${payload.reason}`);
    }
    throw new Error(`proof-jobs fetch failed (${response.status}): ${raw || "empty response"}`);
  }

  if (!payload) {
    throw new Error("proof-jobs endpoint returned non-JSON response");
  }
  return payload;
}
