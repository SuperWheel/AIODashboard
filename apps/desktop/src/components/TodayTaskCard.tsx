import { memo, useState } from "react";
import { api } from "../api";
import type { TaskDayView } from "../types";
import { toastError } from "./DialogHost";

/**
 * 首页今日任务卡：统一使用低饱和主题绿，按 count/target 渐进填充；
 * 末尾圆形勾选框——未完成空心，全部完成打钩。点卡片主体 +1；
 * 完成后点圆圈 = 撤销一步；hover 浮现 −（减一次）；标题进详情。
 */
function TodayTaskCard({
  view,
  dragging,
  onChanged,
  onOpenDetail,
}: {
  view: TaskDayView;
  /** 拖拽会话中暂停卡片自身 hover transform，避免与 FLIP 位移动画叠加。 */
  dragging: boolean;
  onChanged: (taskId: string) => void;
  onOpenDetail: (taskId: string) => void;
}) {
  const task = view.task;
  const accent = "var(--accent)";
  const target = view.target ?? 0;
  const count = view.count;
  const na = view.state === "not_applicable";
  const done = !na && target > 0 && count >= target;
  // 完成即归档的一次性任务：当天以完成态留痕（明天起退出），
  // 主体不可再打卡，仅圆圈可撤销
  const archived = task.status === "archived";
  const ratio = na || target <= 0 ? 0 : Math.min(1, count / target);
  const [pending, setPending] = useState(false);

  const run = async (fn: () => Promise<unknown>) => {
    if (pending) return;
    setPending(true);
    try {
      await fn();
      onChanged(task.id);
    } catch (e) {
      toastError(String(e));
    } finally {
      setPending(false);
    }
  };

  const sub = na
    ? "今天不适用"
    : target > 1
      ? `${count}/${target} ${task.unit}`
      : done
        ? "已完成"
        : "待打卡";
  const titleColor = done ? "var(--ink-2)" : "var(--ink)";
  const subColor = done ? accent : "var(--ink-2)";

  return (
    <div
      className={`group relative overflow-hidden rounded-xl border ${
        na
          ? "opacity-50"
          : dragging
            ? "cursor-grabbing"
            : archived
              ? ""
              : "cursor-pointer transition-all duration-200 hover:-translate-y-px hover:shadow-md"
      }`}
      style={{
        borderColor: done
          ? `color-mix(in srgb, ${accent} 20%, var(--line))`
          : "var(--line)",
        background: "var(--surface)",
      }}
      onClick={() => {
        if (!na && !archived) void run(() => api.taskCheckin(task.id));
      }}
      title={na ? "循环规则今天不适用" : archived ? "已完成 · 明天起归入历史" : "打卡 +1"}
    >
      {/* 渐进填充层：从左到右按比例；完成时整卡加深 */}
      <div
        className="absolute inset-y-0 left-0 transition-[width] duration-300"
        style={{
          width: `${ratio * 100}%`,
          background: `color-mix(in srgb, ${accent} ${done ? 16 : 9}%, transparent)`,
        }}
      />
      <div className="relative flex items-center gap-2.5 px-3 py-2.5">
        <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-sm opacity-70 grayscale">
          {task.icon || "✓"}
        </span>
        <button
          className="min-w-0 flex-1 text-left"
          onClick={(e) => {
            e.stopPropagation();
            onOpenDetail(task.id);
          }}
          title="查看详情"
        >
          <div className="truncate text-sm font-medium" style={{ color: titleColor }}>
            {task.title}
          </div>
          <div className="text-[11px] tabular-nums" style={{ color: subColor }}>
            {sub}
          </div>
        </button>
        {!na && !archived && count > 0 && (
          <button
            className="flex h-6 w-6 shrink-0 items-center justify-center rounded-md text-xs opacity-0 transition-opacity hover:bg-black/10 group-hover:opacity-100"
            style={{ color: "var(--ink-2)" }}
            onClick={(e) => {
              e.stopPropagation();
              void run(() => api.taskDecrement(task.id));
            }}
            title="减少一次"
          >
            −
          </button>
        )}
        <button
          className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full border-[1.5px] text-xs font-bold transition-colors"
          style={{
            borderColor: done ? accent : "var(--ink-3)",
            background: done ? accent : "transparent",
            color: done ? "var(--on-accent)" : "transparent",
          }}
          onClick={(e) => {
            e.stopPropagation();
            if (na) return;
            void run(() => (done ? api.taskUndo(task.id) : api.taskCheckin(task.id)));
          }}
          title={na ? "循环规则今天不适用" : done ? "撤销完成" : "打卡 +1"}
        >
          ✓
        </button>
      </div>
    </div>
  );
}

export default memo(TodayTaskCard);
