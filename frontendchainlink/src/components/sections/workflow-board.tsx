const steps = [
  "1. Encrypted Intent Intake",
  "2. Confidential Match + Policy",
  "3. ZK Proof Generation",
  "4. Private Settlement Action",
  "5. Onchain Receipt Publish",
];

export function WorkflowBoard() {
  return (
    <section className="panel px-6 py-6">
      <div className="flex items-end justify-between gap-3">
        <div>
          <p className="mono text-xs uppercase tracking-[0.12em] text-[var(--fg-muted)]">
            Workflow
          </p>
          <h3 className="mt-1 text-2xl font-semibold">Operational Pipeline</h3>
        </div>
        <span className="pill">Deterministic State Machine</span>
      </div>
      <div className="mt-5 grid gap-3 md:grid-cols-5">
        {steps.map((step, idx) => (
          <article key={step} className="kpi min-h-[96px]">
            <p className="mono text-[11px] uppercase tracking-[0.1em] text-[var(--fg-muted)]">
              Stage {idx + 1}
            </p>
            <p className="mt-2 text-sm leading-5">{step.replace(`${idx + 1}. `, "")}</p>
          </article>
        ))}
      </div>
    </section>
  );
}
