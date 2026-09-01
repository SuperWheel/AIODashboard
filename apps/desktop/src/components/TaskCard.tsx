import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import type { PeriodOverview, Task, TaskDayView } from "../types";
import { recurrenceLabel, taskColor } from "../taskVisual";
import { HeatmapCell, YearHeatmap } from "./Heatmap";
import { toastError } from "./DialogHost";

/** 任务色进度圆环（打卡进度；完成率封顶 100%）。 */
export function TaskRing({
  value,
  color,
  size = 44,
  stroke = 5,
  onClick,
  title,
  children,
}: {
  value: number;
  color: string;
  size?: number;
  stroke?: number;
  onClick?: () => void;
  title?: string;
  children?: React.ReactNode;
}) {
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const clamped = Math.min(1, Math.max(0, value));
  const accent = taskColor(color);
  const ring = (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} role="img" aria-label={title}>
      <circle
        cx={size / 2}
        cy={size / 2}
        r={r}
        fill="none"
        stroke={`color-mix(in srgb, ${accent} 18%, transparent)`}
        strokeWidth={stroke}
      />
      <circle
        cx={size / 2}
        cy={size / 2}
        r={r}
        fill="none"
        stroke={accent}
        strokeWidth={stroke}
        strokeLinecap="round"
        strokeDasharray={c}
        strokeDashoffset={c * (1 - clamped)}
        transform={`rotate(-90 ${size / 2} ${size / 2})`}
        className="transition-[stroke-dashoffset] duration-300"
      />
    </svg>
  );
  const inner = (
    <div className="absolute inset-0 flex items-center justify-center">{children}</div>
  );
  if (onClick) {
    return (
      <button
        onClick={onClick}
        title={title}
        className="relative shrink-0 rounded-full transition-transform hover:scale-105 active:scale-95"
      >
        {ring}
        {inner}
      </button>
    );
  }
  return (
    <div className="relative shrink-0" title={title}>
      {ring}
      {inner}
    </div>
  );
}

/** 打卡控件区：目标=1 → 圆环即按钮；目标>1 → [圆环] [-][+]（圆角矩形同侧并排）。 */
export function CheckinControls({
  view,
  onChanged,
  size = "md",
}: {
  view: TaskDayView;
  onChanged: () => void;
  size?: "sm" | "md";
}) {
  const [pending, setPending] = useState(false);
  const target = view.target ?? 1;
  const count = view.count;
  const done = target > 0 && count >= target;
  const accent = taskColor(view.task.color_hex);
  const ringSize = size === "sm" ? 36 : 44;

  const run = async (fn: () => Promise<unknown>) => {
    if (pending) return;
    setPending(true);
    try {
      await fn();
      onChanged();
    } catch (e) {
      toastError(String(e));
    } finally {
      setPending(false);
    }
  };

  const btnCls =
    "flex items-center justify-center rounded-lg border border-line bg-surface2 text-ink2 transition-colors hover:bg-hover hover:text-ink disabled:opacity-30";
  const btnSize = size === "sm" ? "h-6 w-8 text-xs" : "h-7 w-9 text-sm";

  if (pending) {
    return (
      <div
        className="flex shrink-0 items-center justify-center text-ink3"
        style={{ width: ringSize + 76 }}
      >
        <span className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-ink3 border-t-transparent" />
      </div>
    );
  }

  // 循环规则今天不命中（如每周三的周卡今天周四）：控件整体停用（004）
  if (view.state === "not_applicable") {
    return <span className="shrink-0 text-[11px] text-ink3">今天不适用</span>;
  }

  if (target <= 1) {
    return (
      <TaskRing
        value={done ? 1 : 0}
        color={accent}
        size={ringSize}
        onClick={() =>
          run(() => (done ? api.taskUndo(view.task.id) : api.taskCheckin(view.task.id)))
        }
        title={done ? "撤销完成" : "完成打卡"}
      >
        {done ? (
          <span className="text-sm font-bold" style={{ color: accent }}>
            ✓
          </span>
        ) : (
          <span className="text-xs text-ink3">○</span>
        )}
      </TaskRing>
    );
  }

  return (
    <div className="flex shrink-0 items-center gap-2">
      <TaskRing
        value={target > 0 ? count / target : 0}
        color={accent}
        size={ringSize}
        title={`今日 ${count} / ${target} ${view.task.unit}`}
      >
        <span className="text-[10px] font-semibold tabular-nums text-ink">
          {count}/{target}
        </span>
      </TaskRing>
      {/* +/- 同侧并排（圆角矩形，设计语言决策 7） */}
      <div className="flex gap-1">
        <button
          className={`${btnCls} ${btnSize}`}
          disabled={count <= 0}
          onClick={() => run(() => api.taskDecrement(view.task.id))}
          title="减少一次"
        >
          −
        </button>
        <button
          className={`${btnCls} ${btnSize}`}
          onClick={() => run(() => api.taskCheckin(view.task.id))}
          title="打卡 +1"
        >
          +
        </button>
      </div>
    </div>
  );
}

const STYLES = [
  { key: "day", label: "日卡" },
  { key: "week", label: "周卡" },
  { key: "month", label: "月卡" },
  { key: "year", label: "年卡" },
] as const;

/** 卡片样式切换菜单（右键 / ⋯ 按钮共用）。 */
function StyleMenu({
  task,
  onChanged,
  onClose,
  onEdit,
}: {
  task: Task;
  onChanged: () => void;
  onClose: () => void;
  onEdit: (task: Task) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [onClose]);
  return (
    <div
      ref={ref}
      className="absolute right-0 top-6 z-20 w-28 rounded-xl border border-line bg-surface py-1 shadow-lg"
    >
      <button
        className="flex w-full items-center px-3 py-1.5 text-xs text-ink2 transition-colors hover:bg-hover"
        onClick={() => {
          onClose();
          onEdit(task);
        }}
      >
        编辑任务
      </button>
      <div className="my-1 border-t border-line" />
      {STYLES.map((s) => (
        <button
          key={s.key}
          className={`flex w-full items-center justify-between px-3 py-1.5 text-xs transition-colors hover:bg-hover ${
            task.card_style === s.key ? "font-medium text-accent" : "text-ink2"
          }`}
          onClick={async () => {
            onClose();
            try {
              await api.updateTask({ id: task.id, cardStyle: s.key });
              onChanged();
            } catch (e) {
              toastError(String(e));
            }
          }}
        >
          {s.label}
          {task.card_style === s.key && <span>✓</span>}
        </button>
      ))}
    </div>
  );
}

function subtitleFor(view: TaskDayView, ov: PeriodOverview | null): string {
  const t = view.target;
  const unit = view.task.unit;
  const rec = recurrenceLabel(view.task.recurrence);
  const prefix = rec ? `${rec} · ` : "";
  if (view.state === "not_applicable") {
    return `${prefix}今天不适用`;
  }
  switch (view.task.card_style) {
    case "week":
      return ov
        ? `${prefix}本周完成 ${ov.summary.complete_day_count}/${ov.summary.applicable_day_count} 天 · 连续 ${ov.summary.current_streak} 天 · ${ov.summary.actual_count} ${unit}`
        : "加载中…";
    case "month":
      return ov
        ? `${prefix}本月完成 ${ov.summary.complete_day_count}/${ov.summary.applicable_day_count} 天 · ${ov.summary.actual_count} ${unit} · ${Math.round(ov.summary.complete_day_rate * 100)}%`
        : "加载中…";
    case "year":
      return ov
        ? `${prefix}本年完成 ${ov.summary.complete_day_count}/${ov.summary.applicable_day_count} 天 · ${ov.summary.actual_count} ${unit} · ${Math.round(ov.summary.complete_day_rate * 100)}%`
        : "加载中…";
    default:
      if (view.state === "completed") return `${prefix}今日已完成（${view.count} ${unit}）`;
      if (t && t > 1) return `${prefix}今日 ${view.count} / ${t} ${unit}`;
      return rec || "尚未完成";
  }
}

/**
 * 任务卡片：日/周/月/年四模式共用 header；周/月/年懒加载对应周期总览。
 */
export default function TaskCard({
  view,
  refreshKey,
  onChanged,
  onOpenDetail,
  onEdit,
}: {
  view: TaskDayView;
  refreshKey: number;
  onChanged: () => void;
  onOpenDetail: (taskId: string) => void;
  onEdit: (task: Task) => void;
}) {
  const task = view.task;
  const accent = taskColor(task.color_hex);
  const style = task.card_style;
  const [ov, setOv] = useState<PeriodOverview | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    if (style === "day") return;
    let alive = true;
    api
      .taskOverview(task.id, style)
      .then((r) => {
        if (alive) setOv(r);
      })
      .catch(console.error);
    return () => {
      alive = false;
    };
  }, [task.id, style, refreshKey]);

  return (
    <div
      // 菜单打开时整卡提升层级：卡片之间互为兄弟节点，仅靠菜单自身的
      // z-index 压不过 DOM 靠后的相邻卡片，会被遮挡
      className={`group relative rounded-2xl border bg-surface p-4 shadow-card transition-all duration-150 hover:-translate-y-px hover:shadow-lg ${
        menuOpen ? "z-30" : ""
      }`}
      style={{ borderColor: `color-mix(in srgb, ${accent} 22%, var(--line))` }}
      onContextMenu={(e) => {
        e.preventDefault();
        setMenuOpen(true);
      }}
    >
      {/* header：图标 + 标题 + 副标题 + 控件 */}
      <div className="flex items-center gap-3">
        <div
          className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl text-lg"
          style={{ background: `color-mix(in srgb, ${accent} 16%, transparent)` }}
        >
          {task.icon || "✓"}
        </div>
        <button
          className="min-w-0 flex-1 text-left"
          onClick={() => onOpenDetail(task.id)}
          title="查看详情"
        >
          <div className="truncate text-sm font-semibold text-ink">{task.title}</div>
          <div className="mt-0.5 truncate text-xs text-ink3">{subtitleFor(view, ov)}</div>
        </button>
        <div className="relative flex items-center gap-1.5">
          <CheckinControls view={view} onChanged={onChanged} />
          <button
            className="rounded-md px-1.5 py-1 text-xs text-ink3 transition-colors hover:bg-hover hover:text-ink"
            onClick={() => setMenuOpen((v) => !v)}
            title="卡片样式"
          >
            ⋯
          </button>
          {menuOpen && (
            <StyleMenu
              task={task}
              onChanged={onChanged}
              onClose={() => setMenuOpen(false)}
              onEdit={onEdit}
            />
          )}
        </div>
      </div>

      {/* 周期内容区 */}
      {style === "week" && ov && (
        <button className="mt-3 block w-full" onClick={() => onOpenDetail(task.id)}>
          <div className="flex gap-2">
            {ov.days.map((d) => (
              <HeatmapCell
                key={d.logical_day}
                state={d.display_state}
                rate={d.capped_rate}
                color={accent}
                size={30}
                isToday={d.is_today}
                title={`${d.logical_day}`}
              />
            ))}
          </div>
        </button>
      )}
      {style === "month" && ov && (
        <button className="mt-3 block w-full" onClick={() => onOpenDetail(task.id)}>
          <div className="grid grid-cols-11 gap-[5px]">
            {ov.days.map((d) => (
              <HeatmapCell
                key={d.logical_day}
                state={d.display_state}
                rate={d.capped_rate}
                color={accent}
                size={14}
                isToday={d.is_today}
                title={d.logical_day}
              />
            ))}
          </div>
        </button>
      )}
      {style === "year" && ov && (
        <div className="mt-3">
          <YearHeatmap
            days={ov.days}
            leadingEmpty={ov.leading_empty_count}
            color={accent}
            unit={task.unit}
            cellSize={9}
            gap={2}
          />
        </div>
      )}
    </div>
  );
}
