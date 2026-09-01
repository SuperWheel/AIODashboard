import { useState } from "react";
import { api } from "../api";
import type { TaskDayView } from "../types";
import { taskColor } from "../taskVisual";
import { toastError } from "./DialogHost";

/**
 * 首页今日任务卡：圆角矩形，主题色从左按比例填充（count/target）；
 * 末尾圆形勾选框——未完成空心，全部完成打钩。点卡片主体 +1；
 * 完成后点圆圈 = 撤销一步；hover 浮现 −（减一次）；标题进详情。
 */
export default function TodayTaskCard({
  view,
  onChanged,
  onOpenDetail,
}: {
  view: TaskDayView;
  onChanged: () => void;
  onOpenDetail: (taskId: string) => void;
}) {
  const task = view.task;
  const accent = taskColor(task.color_hex);
  const target = view.target ?? 0;
  const count = view.count;
  const na = view.state === "not_applicable";
  const done = !na && target > 0 && count >= target;
  const ratio = na || target <= 0 ? 0 : Math.min(1, count / target);
  const [pending, setPending] = useState(false);

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

  const sub = na
    ? "今天不适用"
    : target > 1
      ? `${count}/${target} ${task.unit}`
      : done
        ? "已完成"
        : "待打卡";
  const titleColor = done ? "var(--on-accent)" : "var(--ink)";
  const subColor = done
    ? "color-mix(in srgb, var(--on-accent) 78%, transparent)"
    : "var(--ink-3)";

  return (
    <div
      className={`group relative overflow-hidden rounded-xl border transition-all duration-200 ${
        na ? "opacity-50" : "cursor-pointer hover:-translate-y-px hover:shadow-md"
      }`}
      style={{
        borderColor: `color-mix(in srgb, ${accent} 26%, var(--line))`,
        background: "var(--surface)",
      }}
      onClick={() => {
        if (!na) void run(() => api.taskCheckin(task.id));
      }}
      title={na ? "循环规则今天不适用" : "打卡 +1"}
    >
      {/* 渐进填充层：从左到右按比例；完成时整卡加深 */}
      <div
        className="absolute inset-y-0 left-0 transition-[width] duration-300"
        style={{
          width: `${ratio * 100}%`,
          background: `color-mix(in srgb, ${accent} ${done ? 80 : 26}%, transparent)`,
        }}
      />
      <div className="relative flex items-center gap-2.5 px-3 py-2.5">
        <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-sm">
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
        {!na && count > 0 && (
          <button
            className="flex h-6 w-6 shrink-0 items-center justify-center rounded-md text-xs opacity-0 transition-opacity hover:bg-black/10 group-hover:opacity-100"
            style={{ color: done ? "var(--on-accent)" : "var(--ink-3)" }}
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
            borderColor: done ? accent : `color-mix(in srgb, ${accent} 55%, var(--line))`,
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
