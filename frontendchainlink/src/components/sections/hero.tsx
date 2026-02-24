export function Hero() {
  return (
    <section className="panel relative overflow-hidden px-6 py-8 md:px-10 md:py-10">
      <p className="pill mb-4">Chainlink Privacy Track</p>
      <h2 className="max-w-4xl text-3xl font-semibold tracking-tight md:text-5xl">
        Prove settlement validity without exposing counterparties, amounts, or credentials.
      </h2>
      <p className="mt-4 max-w-3xl text-sm text-[var(--fg-muted)] md:text-base">
        End-to-end rail for encrypted intent intake, offchain compliance and matching, ZK proof
        generation, and minimal onchain receipt publication.
      </p>
      <div className="mt-7 grid gap-3 md:grid-cols-3">
        <div className="kpi">
          <p className="mono text-xs text-[var(--fg-muted)]">Current Network</p>
          <p className="mt-1 text-xl font-semibold">Ethereum Sepolia</p>
        </div>
        <div className="kpi">
          <p className="mono text-xs text-[var(--fg-muted)]">Coordinator State</p>
          <p className="mt-1 text-xl font-semibold">READY</p>
        </div>
        <div className="kpi">
          <p className="mono text-xs text-[var(--fg-muted)]">Security Mode</p>
          <p className="mt-1 text-xl font-semibold">Wallet Signature Login</p>
        </div>
      </div>
    </section>
  );
}
