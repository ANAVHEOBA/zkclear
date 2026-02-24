import {
  fetchWalletMe,
  requestWalletNonce,
  verifyWalletSignature,
  type WalletSession,
} from "@/lib/api/wallet-auth";
import type { Eip1193Provider } from "@/lib/auth/eip6963";
import { getRolePanels, normalizeRole, type PanelKey, type UserRole } from "@/lib/auth/role-gate";
import {
  clearWalletSession,
  isWalletSessionExpired,
  loadWalletSession,
  saveWalletSession,
} from "@/lib/auth/session";

declare global {
  interface Window {
    ethereum?: Eip1193Provider;
  }
}

export interface LoginRoleGateResult {
  walletAddress: string;
  role: UserRole;
  panels: Record<PanelKey, boolean>;
}

function getEthereum(): Eip1193Provider {
  if (typeof window === "undefined" || !window.ethereum) {
    throw new Error("wallet provider not found");
  }
  return window.ethereum;
}

export async function loginWithWalletRoleGate(
  provider?: Eip1193Provider,
): Promise<LoginRoleGateResult> {
  const ethereum = provider ?? getEthereum();
  const accounts = (await ethereum.request({
    method: "eth_requestAccounts",
  })) as string[];
  const walletAddress = accounts?.[0];
  if (!walletAddress) {
    throw new Error("wallet account not available");
  }

  const nonceResp = await requestWalletNonce({ wallet_address: walletAddress });
  const signature = (await ethereum.request({
    method: "personal_sign",
    params: [nonceResp.message, walletAddress],
  })) as string;
  return applyVerifiedWalletSession(walletAddress, signature);
}

export async function verifyConnectedWalletRoleGate(
  walletAddress: string,
  signMessage: (message: string) => Promise<string>,
): Promise<LoginRoleGateResult> {
  const nonceResp = await requestWalletNonce({ wallet_address: walletAddress });
  const signature = await signMessage(nonceResp.message);
  return applyVerifiedWalletSession(walletAddress, signature);
}

export async function restoreRoleGateFromSession(): Promise<LoginRoleGateResult | null> {
  const session = loadWalletSession();
  if (!session) {
    return null;
  }
  if (isWalletSessionExpired(session.expiresAt)) {
    clearWalletSession();
    return null;
  }
  const me = await fetchWalletMe(session.accessToken);
  const role = normalizeRole(me.role);
  if (!role) {
    clearWalletSession();
    throw new Error(`unsupported role from backend: ${me.role}`);
  }

  const refreshed: WalletSession = {
    accessToken: session.accessToken,
    walletAddress: me.wallet_address,
    role,
    expiresAt: session.expiresAt,
  };
  saveWalletSession(refreshed);

  return {
    walletAddress: refreshed.walletAddress,
    role: refreshed.role,
    panels: getRolePanels(refreshed.role),
  };
}

async function applyVerifiedWalletSession(
  walletAddress: string,
  signature: string,
): Promise<LoginRoleGateResult> {
  const session = await verifyWalletSignature({
    wallet_address: walletAddress,
    signature,
  });
  saveWalletSession(session);

  return {
    walletAddress: session.walletAddress,
    role: session.role,
    panels: getRolePanels(session.role),
  };
}
