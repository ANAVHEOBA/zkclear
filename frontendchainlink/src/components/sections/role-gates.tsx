const roles = [
  {
    name: "Dealer",
    summary: "Submit encrypted intents and trigger OTC orchestration.",
    color: "var(--ok)",
  },
  {
    name: "Ops",
    summary: "Track proving queue, retries, publish status, and health.",
    color: "var(--warn)",
  },
  {
    name: "Compliance",
    summary: "Review attestations, policy snapshots, and risk decisions.",
    color: "var(--bad)",
  },
];

export function RoleGates() {
  return (
    <section className="grid gap-4 md:grid-cols-3">
      {roles.map((role) => (
        <article key={role.name} className="panel px-5 py-5">
          <p className="mono text-xs uppercase tracking-[0.12em] text-[var(--fg-muted)]">Role</p>
          <h3 className="mt-2 text-xl font-semibold">{role.name}</h3>
          <p className="mt-2 text-sm text-[var(--fg-muted)]">{role.summary}</p>
          <span
            className="mt-4 inline-block h-[2px] w-12 rounded-full"
            style={{ backgroundColor: role.color }}
          />
        </article>
      ))}
    </section>
  );
}
