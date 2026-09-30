/** Baseline health probe for the sovereign cockpit (no I/O, pure status). */

export interface HealthProbe {
  key: string;
  labelFr: string;
  ready: boolean;
}

/** Static cockpit probes; live values arrive with TASK-BRK-002 (SQLite). */
export function baselineProbes(): HealthProbe[] {
  return [
    { key: "core", labelFr: "Noyau Rust", ready: true },
    { key: "offline", labelFr: "Mode hors-ligne", ready: true },
    { key: "database", labelFr: "Base SQLite", ready: false },
    { key: "printer", labelFr: "Imprimante ticket", ready: false },
  ];
}
