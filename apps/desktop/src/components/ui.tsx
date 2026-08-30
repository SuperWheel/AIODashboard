import type { ReactNode } from "react";

export function Badge({ children, tone = "slate" }: { children: ReactNode; tone?: BadgeTone }) {
  const tones: Record<BadgeTone, string> = {
    slate: "bg-slate-500/15 text-slate-300 border-slate-400/20",
    green: "bg-emerald-500/15 text-emerald-300 border-emerald-400/25",
    amber: "bg-amber-500/15 text-amber-300 border-amber-400/25",
    red: "bg-rose-500/15 text-rose-300 border-rose-400/25",
    blue: "bg-sky-500/15 text-sky-300 border-sky-400/25",
    violet: "bg-violet-500/15 text-violet-300 border-violet-400/25",
  };
  return (
    <span
      className={`inline-flex shrink-0 items-center rounded-full border px-2 py-0.5 text-[11px] leading-none ${tones[tone]}`}
    >
      {children}
    </span>
  );
}

export type BadgeTone = "slate" | "green" | "amber" | "red" | "blue" | "violet";

export function SectionTitle({ children, right }: { children: ReactNode; right?: ReactNode }) {
  return (
    <div className="mb-2 mt-6 flex items-center justify-between first:mt-0">
      <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400">{children}</h2>
      {right}
    </div>
  );
}

export function Empty({ text }: { text: string }) {
  return (
    <div className="rounded-lg border border-dashed border-white/10 py-8 text-center text-sm text-slate-500">
      {text}
    </div>
  );
}

export function Card({ children, className = "" }: { children: ReactNode; className?: string }) {
  return (
    <div className={`rounded-xl border border-white/10 bg-[#161a22] ${className}`}>{children}</div>
  );
}

export function StatCard({
  label,
  value,
  tone = "slate",
}: {
  label: string;
  value: number | string;
  tone?: BadgeTone;
}) {
  const dot: Record<BadgeTone, string> = {
    slate: "bg-slate-400",
    green: "bg-emerald-400",
    amber: "bg-amber-400",
    red: "bg-rose-400",
    blue: "bg-sky-400",
    violet: "bg-violet-400",
  };
  return (
    <Card className="px-4 py-3">
      <div className="flex items-center gap-1.5 text-xs text-slate-400">
        <span className={`h-1.5 w-1.5 rounded-full ${dot[tone]}`} />
        {label}
      </div>
      <div className="mt-1 text-xl font-semibold text-white">{value}</div>
    </Card>
  );
}
