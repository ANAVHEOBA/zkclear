export type UserRole = "dealer" | "ops" | "compliance";

export type PanelKey = "dealer" | "ops" | "compliance";

const PANEL_RULES: Record<PanelKey, readonly UserRole[]> = {
  dealer: ["dealer", "ops"],
  ops: ["ops"],
  compliance: ["compliance", "ops"],
};

export function normalizeRole(role: string): UserRole | null {
  const value = role.trim().toLowerCase();
  if (value === "dealer" || value === "ops" || value === "compliance") {
    return value;
  }
  return null;
}

export function canAccessPanel(role: UserRole, panel: PanelKey): boolean {
  return PANEL_RULES[panel].includes(role);
}

export function getRolePanels(role: UserRole): Record<PanelKey, boolean> {
  return {
    dealer: canAccessPanel(role, "dealer"),
    ops: canAccessPanel(role, "ops"),
    compliance: canAccessPanel(role, "compliance"),
  };
}
