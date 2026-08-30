import type { ButtonHTMLAttributes, ReactNode } from "react";

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

/** 统一按钮：primary=主操作，violet=收件箱收集，ghost=次级操作。 */
export function Button({
  variant = "primary",
  className = "",
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: "primary" | "violet" | "ghost" }) {
  const variants = {
    primary: "bg-accent text-onaccent hover:bg-accent/90",
    violet: "bg-violet text-onaccent hover:bg-violet/90",
    ghost: "border border-line text-ink2 hover:bg-hover hover:text-ink",
  };
  return (
    <button
      className={`rounded-lg px-3 py-1 text-xs font-medium transition-colors disabled:opacity-40 ${variants[variant]} ${className}`}
      {...rest}
    />
  );
}

export function SectionTitle({ children, right }: { children: ReactNode; right?: ReactNode }) {
  return (
    <div className="mb-2 mt-6 flex items-center justify-between first:mt-0">
      <h2 className="text-xs font-semibold uppercase tracking-wider text-ink2">{children}</h2>
      {right}
    </div>
  );
}

/** 子页统一页头：标题 + 可选数量徽标 + 描述 + 右侧主操作。 */
export function PageHeader({
  title,
  count,
  desc,
  actions,
}: {
  title: string;
  count?: number;
  desc?: string;
  actions?: ReactNode;
}) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div className="min-w-0">
        <div className="flex items-baseline gap-2">
          <h1 className="text-xl font-semibold text-ink">{title}</h1>
          {count !== undefined && (
            <span className="text-sm tabular-nums text-ink3">{count}</span>
          )}
        </div>
        {desc && <p className="mt-1 text-xs text-ink2">{desc}</p>}
      </div>
      {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
    </div>
  );
}

/** 空态：大 glyph + 一句引导 + 可选主按钮。 */
export function Empty({
  text,
  glyph,
  action,
}: {
  text: string;
  glyph?: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="flex flex-col items-center gap-2.5 rounded-2xl border border-dashed border-line py-10 text-center">
      {glyph && <div className="text-2xl text-ink3">{glyph}</div>}
      <div className="text-sm text-ink3">{text}</div>
      {action}
    </div>
  );
}

export function Card({
  children,
  className = "",
  hoverable = false,
  onClick,
}: {
  children: ReactNode;
  className?: string;
  hoverable?: boolean;
  /** 传入即为可点卡片（hover 上浮 + 键盘 Enter/Space 触发） */
  onClick?: () => void;
}) {
  const lift = hoverable || onClick !== undefined;
  return (
    <div
      className={`rounded-2xl border border-line bg-surface shadow-card ${
        lift ? "transition-all duration-150 hover:-translate-y-px hover:shadow-lg" : ""
      } ${onClick ? "cursor-pointer" : ""} ${className}`}
      onClick={onClick}
      role={onClick ? "button" : undefined}
      tabIndex={onClick ? 0 : undefined}
      onKeyDown={
        onClick
          ? (e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onClick();
              }
            }
          : undefined
      }
    >
      {children}
    </div>
  );
}

/** 今日完成率圆环（SVG，随主题 token 变色）。value ∈ [0,1]。 */
export function ProgressRing({
  value,
  size = 56,
  stroke = 5,
  className = "",
}: {
  value: number;
  size?: number;
  stroke?: number;
  className?: string;
}) {
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const clamped = Math.min(1, Math.max(0, value));
  return (
    <svg
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      className={className}
      role="img"
      aria-label={`完成率 ${Math.round(clamped * 100)}%`}
    >
      <circle
        cx={size / 2}
        cy={size / 2}
        r={r}
        fill="none"
        stroke="var(--line)"
        strokeWidth={stroke}
      />
      <circle
        cx={size / 2}
        cy={size / 2}
        r={r}
        fill="none"
        stroke="var(--accent)"
        strokeWidth={stroke}
        strokeLinecap="round"
        strokeDasharray={c}
        strokeDashoffset={c * (1 - clamped)}
        transform={`rotate(-90 ${size / 2} ${size / 2})`}
        className="transition-[stroke-dashoffset] duration-500"
      />
    </svg>
  );
}

export function StatCard({
  label,
  value,
  tone = "slate",
  onClick,
}: {
  label: string;
  value: number | string;
  tone?: BadgeTone;
  /** 传入即可点击跳转（hover 上浮） */
  onClick?: () => void;
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
    <Card className="px-4 py-3" onClick={onClick}>
      <div className="flex items-center gap-1.5 text-xs text-ink2">
        <span className={`h-1.5 w-1.5 rounded-full ${dot[tone]}`} />
        {label}
      </div>
      <div className="mt-1 text-xl font-semibold tabular-nums text-ink">{value}</div>
    </Card>
  );
}
