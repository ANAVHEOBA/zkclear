export function TopNav() {
  return (
    <header className="panel sticky top-4 z-20 mx-auto flex w-full max-w-6xl items-center justify-between px-5 py-4 md:px-7">
      <div>
        <p className="mono text-xs tracking-[0.15em] text-[var(--fg-muted)]">ZK-CLEAR</p>
        <h1 className="text-base font-semibold md:text-lg">Private Settlement Ops Desk</h1>
      </div>
      <button className="mono rounded-full border border-[var(--bg-panel-border)] bg-[#04086a]/70 px-4 py-2 text-xs font-medium uppercase tracking-[0.08em] text-[var(--accent-strong)] transition hover:bg-[#0d1290]">
        Connect Wallet
      </button>
    </header>
  );
}
