"use client";

import { useEffect, useMemo, useState } from "react";
import { ConnectButton } from "@rainbow-me/rainbowkit";
import { useAccount, useDisconnect, useSignMessage } from "wagmi";
import {
  restoreRoleGateFromSession,
  verifyConnectedWalletRoleGate,
  type LoginRoleGateResult,
} from "@/lib/auth/login-flow";
import { clearWalletSession } from "@/lib/auth/session";

const navLinks = [
  { label: "Intent Desk", href: "#intents" },
  { label: "Compliance", href: "#compliance" },
  { label: "Proof Pipeline", href: "#proof-jobs" },
  { label: "Publish Receipt", href: "#publish-receipt" },
  { label: "Audit Trail", href: "#audit" },
  { label: "System Health", href: "#health" },
];

export function Hack26Header() {
  const [auth, setAuth] = useState<LoginRoleGateResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeHref, setActiveHref] = useState("#intents");

  const { address, isConnected } = useAccount();
  const { signMessageAsync } = useSignMessage();
  const { disconnect } = useDisconnect();

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const restored = await restoreRoleGateFromSession();
        if (!cancelled) {
          setAuth(restored);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "session restore failed");
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!isConnected || !address) {
      setAuth(null);
    }
  }, [isConnected, address]);

  useEffect(() => {
    const applyHash = () => {
      const hash = window.location.hash || "#intents";
      const known = navLinks.some((item) => item.href === hash);
      setActiveHref(known ? hash : "#intents");
    };
    applyHash();
    window.addEventListener("hashchange", applyHash);
    return () => {
      window.removeEventListener("hashchange", applyHash);
    };
  }, []);

  const shortWallet = useMemo(() => {
    const current = auth?.walletAddress || address;
    if (!current) {
      return "";
    }
    return `${current.slice(0, 6)}...${current.slice(-4)}`;
  }, [address, auth?.walletAddress]);

  async function handleVerifyAccess() {
    if (!address) {
      setError("wallet address unavailable");
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const result = await verifyConnectedWalletRoleGate(address, async (message) =>
        signMessageAsync({ message }),
      );
      setAuth(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : "wallet auth failed");
    } finally {
      setLoading(false);
    }
  }

  function handleDisconnect() {
    disconnect();
    clearWalletSession();
    setAuth(null);
    setError(null);
  }

  function handleNavClick(event: React.MouseEvent<HTMLAnchorElement>, href: string) {
    if (!href.startsWith("#")) {
      return;
    }
    event.preventDefault();
    const target = document.querySelector(href);
    if (!target) {
      return;
    }
    target.scrollIntoView({ behavior: "smooth", block: "start" });
    setActiveHref(href);
    window.history.replaceState(null, "", href);
  }

  return (
    <header className="hack26-navbar-wrap">
      <div className="hack26-navbar14_container">
        <nav className="hack26-navbar14_menu" aria-label="Primary">
          {navLinks.map((item) => (
            <a
              key={item.label}
              href={item.href}
              onClick={(event) => handleNavClick(event, item.href)}
              className={`hack26-navbar14_link ${activeHref === item.href ? "w--current" : ""}`}
              aria-current={activeHref === item.href ? "page" : undefined}
            >
              {item.label}
            </a>
          ))}
        </nav>
        <div className="hack26-auth-box">
          <ConnectButton.Custom>
            {({ account, chain, openConnectModal, mounted }) => {
              const connected = mounted && account && chain;

              if (!connected) {
                return (
                  <button
                    type="button"
                    onClick={openConnectModal}
                    className="hack26-button hack26-is-secondary hack26-is-small"
                  >
                    Connect Wallet
                  </button>
                );
              }

              if (!auth) {
                return (
                  <button
                    type="button"
                    disabled={loading}
                    onClick={() => void handleVerifyAccess()}
                    className="hack26-button hack26-is-secondary hack26-is-small"
                  >
                    {loading ? "Verifying..." : "Verify Access"}
                  </button>
                );
              }

              return (
                <>
                  <span className="hack26-auth-chip">{auth.role.toUpperCase()}</span>
                  <span className="hack26-auth-wallet">{shortWallet}</span>
                  <span className={`hack26-panel-chip ${auth.panels.dealer ? "on" : "off"}`}>
                    Dealer
                  </span>
                  <span className={`hack26-panel-chip ${auth.panels.ops ? "on" : "off"}`}>
                    Ops
                  </span>
                  <span
                    className={`hack26-panel-chip ${auth.panels.compliance ? "on" : "off"}`}
                  >
                    Compliance
                  </span>
                  <button
                    type="button"
                    className="hack26-wallet-logout"
                    onClick={handleDisconnect}
                  >
                    Disconnect
                  </button>
                </>
              );
            }}
          </ConnectButton.Custom>
        </div>
      </div>
      {error ? <p className="hack26-auth-error">{error}</p> : null}
    </header>
  );
}
