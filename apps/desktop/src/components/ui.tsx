import type { ReactNode } from "react";

/**
 * 语义化 Badge：tone 映射语义 token（accent/danger/warn/info/violet），
 * 双主题对比度由 token 保证（深色 400 系 / 浅色 600-700 系）。
 */
export function Badge({ children, tone = "slate" }: { children: ReactNode; tone?: BadgeTone }) {
  const tones: Record<BadgeTone, string> = {
    slate: "bg-ink3/10 text-ink2 border-ink3/25",
    green: "bg-accent/10 text-accent border-accent/30",
    amber: "bg-warn/10 text-warn border-warn/30",
    red: "bg-danger/10 text-danger border-danger/30",
    blue: "bg-info/10 text-info border-info/30",
    violet: "bg-violet/10 text-violet border-violet/30",
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
      <h2 className="text-xs font-semibold uppercase tracking-wider text-ink2">{children}</h2>
      {right}
    </div>
  );
}

export function Empty({ text }: { text: string }) {
  return (
    <div className="rounded-lg border border-dashed border-line py-8 text-center text-sm text-ink3">
      {text}
    </div>
  );
}

export function Card({ children, className = "" }: { children: ReactNode; className?: string }) {
  return (
    <div className={`rounded-2xl border border-line bg-surface shadow-card ${className}`}>
      {children}
    </div>
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
    slate: "bg-ink3",
    green: "bg-accent",
    amber: "bg-warn",
    red: "bg-danger",
    blue: "bg-info",
    violet: "bg-violet",
  };
  return (
    <Card className="px-4 py-3">
      <div className="flex items-center gap-1.5 text-xs text-ink2">
        <span className={`h-1.5 w-1.5 rounded-full ${dot[tone]}`} />
        {label}
      </div>
      <div className="mt-1 text-xl font-semibold tabular-nums text-ink">{value}</div>
    </Card>
  );
}
