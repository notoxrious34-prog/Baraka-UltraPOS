import type { ReactElement } from "react";

interface StatusCardProps {
  title: string;
  value: string;
  tone: "ok" | "warn" | "idle";
}

const toneClasses: Record<StatusCardProps["tone"], string> = {
  ok: "border-emerald-500/40 bg-emerald-50",
  warn: "border-amber-500/40 bg-amber-50",
  idle: "border-slate-300 bg-white",
};

export function StatusCard({ title, value, tone }: StatusCardProps): ReactElement {
  return (
    <section className={`rounded-lg border p-4 shadow-sm ${toneClasses[tone]}`}>
      <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-500">{title}</h2>
      <p className="mt-1 text-lg font-bold text-slate-900">{value}</p>
    </section>
  );
}
