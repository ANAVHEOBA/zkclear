export interface Eip1193Provider {
  request: (args: { method: string; params?: unknown[] }) => Promise<unknown>;
}

export interface Eip6963ProviderInfo {
  uuid: string;
  name: string;
  icon: string;
  rdns: string;
}

export interface Eip6963AnnounceDetail {
  info: Eip6963ProviderInfo;
  provider: Eip1193Provider;
}

interface Eip6963AnnounceEvent extends Event {
  detail: Eip6963AnnounceDetail;
}

export interface DiscoveredWalletProvider {
  info: Eip6963ProviderInfo;
  provider: Eip1193Provider;
}

declare global {
  interface Window {
    ethereum?: Eip1193Provider;
  }
}

export async function discoverWalletProviders(
  timeoutMs = 350,
): Promise<DiscoveredWalletProvider[]> {
  if (typeof window === "undefined") {
    return [];
  }

  const byUuid = new Map<string, DiscoveredWalletProvider>();
  const onAnnounce = (event: Event) => {
    const e = event as Eip6963AnnounceEvent;
    if (!e.detail?.info?.uuid || !e.detail.provider) {
      return;
    }
    byUuid.set(e.detail.info.uuid, {
      info: e.detail.info,
      provider: e.detail.provider,
    });
  };

  window.addEventListener("eip6963:announceProvider", onAnnounce);
  window.dispatchEvent(new Event("eip6963:requestProvider"));

  await new Promise((resolve) => window.setTimeout(resolve, timeoutMs));
  window.removeEventListener("eip6963:announceProvider", onAnnounce);

  if (byUuid.size === 0 && window.ethereum) {
    byUuid.set("legacy-window-ethereum", {
      info: {
        uuid: "legacy-window-ethereum",
        name: "Injected Wallet",
        icon: "",
        rdns: "window.ethereum",
      },
      provider: window.ethereum,
    });
  }

  return Array.from(byUuid.values()).sort((a, b) =>
    a.info.name.localeCompare(b.info.name),
  );
}
