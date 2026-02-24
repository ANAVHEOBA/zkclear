import { getBackendBaseUrl } from "@/lib/api/config";
import { normalizeRole, type UserRole } from "@/lib/auth/role-gate";

export interface WalletNonceRequest {
  wallet_address: string;
}

export interface WalletNonceResponse {
  accepted: boolean;
  wallet_address: string;
  nonce: string;
  message: string;
  expires_at: number;
  error_code?: string | null;
  reason: string;
}

export interface WalletVerifyRequest {
  wallet_address: string;
  signature: string;
}

export interface WalletVerifyResponse {
  accepted: boolean;
  access_token: string;
  token_type: string;
  expires_at: number;
  wallet_address: string;
  role: string;
  error_code?: string | null;
  reason: string;
}

export interface WalletMeResponse {
  authenticated: boolean;
  wallet_address: string;
  role: string;
  error_code?: string | null;
  reason: string;
}

export interface WalletSession {
  accessToken: string;
  walletAddress: string;
  role: UserRole;
  expiresAt: number;
}

async function parseJson<T>(response: Response): Promise<T> {
  const data = (await response.json()) as T;
  return data;
}

export async function requestWalletNonce(
  input: WalletNonceRequest,
): Promise<WalletNonceResponse> {
  const response = await fetch(`${getBackendBaseUrl()}/v1/auth/wallet/nonce`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(input),
  });
  const payload = await parseJson<WalletNonceResponse>(response);
  if (!response.ok || !payload.accepted) {
    throw new Error(payload.reason || "failed to request wallet nonce");
  }
  return payload;
}

export async function verifyWalletSignature(
  input: WalletVerifyRequest,
): Promise<WalletSession> {
  const response = await fetch(`${getBackendBaseUrl()}/v1/auth/wallet/verify`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(input),
  });
  const payload = await parseJson<WalletVerifyResponse>(response);
  if (!response.ok || !payload.accepted) {
    throw new Error(payload.reason || "wallet verification failed");
  }
  const role = normalizeRole(payload.role);
  if (!role) {
    throw new Error(`unsupported role from backend: ${payload.role}`);
  }
  return {
    accessToken: payload.access_token,
    walletAddress: payload.wallet_address,
    role,
    expiresAt: payload.expires_at,
  };
}

export async function fetchWalletMe(accessToken: string): Promise<WalletMeResponse> {
  const response = await fetch(`${getBackendBaseUrl()}/v1/auth/wallet/me`, {
    method: "GET",
    headers: { authorization: `Bearer ${accessToken}` },
  });
  const payload = await parseJson<WalletMeResponse>(response);
  if (!response.ok || !payload.authenticated) {
    throw new Error(payload.reason || "wallet session is not authenticated");
  }
  return payload;
}
