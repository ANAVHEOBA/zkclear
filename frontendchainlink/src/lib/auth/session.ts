import type { WalletSession } from "@/lib/api/wallet-auth";

const SESSION_KEY = "zkclear.wallet.session";

export function saveWalletSession(session: WalletSession): void {
  if (typeof window === "undefined") {
    return;
  }
  window.localStorage.setItem(SESSION_KEY, JSON.stringify(session));
}

export function loadWalletSession(): WalletSession | null {
  if (typeof window === "undefined") {
    return null;
  }
  const raw = window.localStorage.getItem(SESSION_KEY);
  if (!raw) {
    return null;
  }
  try {
    return JSON.parse(raw) as WalletSession;
  } catch {
    return null;
  }
}

export function clearWalletSession(): void {
  if (typeof window === "undefined") {
    return;
  }
  window.localStorage.removeItem(SESSION_KEY);
}

export function isWalletSessionExpired(expiresAt: number): boolean {
  const now = Math.floor(Date.now() / 1000);
  return now >= expiresAt;
}
