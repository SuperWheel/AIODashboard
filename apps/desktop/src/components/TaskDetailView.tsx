import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import type { PeriodOverview, TaskDayView } from "../types";
import { isPrevYear, taskColor } from "../taskVisual";
import { dayDetailText, HeatmapCell, YearHeatmap } from "./Heatmap";
import { CheckinControls, TaskRing } from "./TaskCard";
import { Button } from "./ui";
import { toastError } from "./DialogHost";

type Period = "week" | "month" | "year";

function shiftAnchor(anchor: string, period: Period, dir: 1 | -1): string {
  const d = new Date(`${anchor}T00:00:00`);
  if (period === "week") d.setDate(d.getDate() + 7 * dir);
  else if (period === "month") d.setMonth(d.getMonth() + dir);
  else d.setFullYear(d.getFullYear() + dir);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

function periodLabel(ov: PeriodOverview): string {
  if (ov.kind === "week") return `${ov.start_day.slice(5)} ~ ${ov.end_day.slice(5)}`;
  if (ov.kind === "month")
    return `${ov.start_day.slice(0, 4)} 年 ${Number(ov.start_day.slice(5, 7))} 月`;
  return `${ov.start_day.slice(0, 4)} 年`;
}

/** 任务详情：周/月/年三档总览 + 统计 + 打卡。快捷键 ⌘1/2/3、⌥←/→。 */
export default function TaskDetailView({
  taskId,
  refreshKey,
  onChanged,
  onBack,
  onEdit,
}: {
  taskId: string;
  refreshKey: number;
  onChanged: () => void;
  onBack: () => void;
  onEdit: () => void;
}) {
  const [period, setPeriod] = useState<Period>("week");
  const [anchor, setAnchor] = useState<string>(() => {
    const d = new Date();
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  });
  const [ov, setOv] = useState<PeriodOverview | null>(null);
  const [view, setView] = useState<TaskDayView | null>(null);
  const [selectedDay, setSelectedDay] = useState<string | null>(null);
  // 撤销按钮防抖：请求返回前快速连点会连撤两条
  const [undoBusy, setUndoBusy] = useState(false);

  const load = useCallback(() => {
    api.taskOverview(taskId, period, anchor).then(setOv).catch((e) => toastError(String(e)));
    api
      .getToday()
      .then((t) => setView(t.today_tasks.find((x) => x.task.id === taskId) ?? null))
      .catch(console.error);
  }, [taskId, period, anchor]);

  useEffect(load, [load, refreshKey]);

  // 快捷键：⌘1/2/3 切周期，⌥←/→ 前后周期，Esc 返回
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey && e.key === "1") setPeriod("week");
      else if (e.metaKey && e.key === "2") setPeriod("month");
      else if (e.metaKey && e.key === "3") setPeriod("year");
      else if (e.altKey && e.key === "ArrowLeft") setAnchor((a) => shiftAnchor(a, period, -1));
      else if (e.altKey && e.key === "ArrowRight") setAnchor((a) => shiftAnchor(a, period, 1));
      else if (e.key === "Escape") onBack();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [period, onBack]);

  if (!ov) {
    return <div className="py-20 text-center text-sm text-ink3">加载中…</div>;
  }
  const task = view?.task;
  const accent = taskColor(task?.color_hex ?? "");
  const s = ov.summary;
  const selected = ov.days.find((d) => d.logical_day === selectedDay) ?? null;

  return (
    <div>
      {/* 头部 */}
      <div className="flex items-center gap-3">
        <Button variant="ghost" onClick={onBack}>
          ← 返回
        </Button>
        <div
          className="flex h-10 w-10 items-center justify-center rounded-xl text-base"
          style={{ background: `color-mix(in srgb, ${accent} 16%, transparent)` }}
        >
          {task?.icon || "✓"}
        </div>
        <div className="min-w-0 flex-1">
          <h1 className="truncate text-lg font-semibold text-ink">{task?.title ?? "任务"}</h1>
          <div className="text-xs text-ink3">
            {view?.target ? `今日 ${view.count} / ${view.target} ${task?.unit ?? ""}` : "今日不适用"}
          </div>
        </div>
        {view && <CheckinControls view={view} onChanged={onChanged} />}
        <Button variant="ghost" onClick={onEdit}>
          编辑
        </Button>
      </div>

      {/* 周期切换 */}
      <div className="mt-5 flex items-center justify-between">
        <div className="flex gap-1">
          {(["week", "month", "year"] as Period[]).map((p) => (
            <button
              key={p}
              onClick={() => setPeriod(p)}
              className={`rounded-lg px-3 py-1 text-xs transition-colors ${
                period === p ? "bg-accent/10 font-medium text-accent" : "text-ink3 hover:text-ink"
              }`}
            >
              {p === "week" ? "周" : p === "month" ? "月" : "年"}
            </button>
          ))}
        </div>
        <div className="flex items-center gap-2">
          <button
            className="rounded-md px-2 py-1 text-xs text-ink3 hover:bg-hover hover:text-ink"
            onClick={() => setAnchor((a) => shiftAnchor(a, period, -1))}
          >
            ←
          </button>
          <span className="text-xs tabular-nums text-ink2">{periodLabel(ov)}</span>
          <button
            className="rounded-md px-2 py-1 text-xs text-ink3 hover:bg-hover hover:text-ink"
            onClick={() => setAnchor((a) => shiftAnchor(a, period, 1))}
          >
            →
          </button>
        </div>
      </div>

      {/* 统计条 */}
      <div className="mt-4 grid grid-cols-5 gap-3">
        {[
          { label: "实际次数", value: s.actual_count },
          { label: "适用天数", value: s.applicable_day_count },
          { label: "完整完成", value: s.complete_day_count },
          { label: "完成率", value: `${Math.round(s.complete_day_rate * 100)}%` },
          {
            label: period === "week" ? "当前连续" : "最长连续",
            value: period === "week" ? s.current_streak : s.longest_streak,
          },
        ].map((x) => (
          <div key={x.label} className="rounded-xl border border-line bg-surface px-3 py-2">
            <div className="text-[11px] text-ink3">{x.label}</div>
            <div className="mt-0.5 text-lg font-semibold tabular-nums text-ink">{x.value}</div>
          </div>
        ))}
      </div>

      {/* 周期视图 */}
      <div className="mt-4 rounded-2xl border border-line bg-surface p-4">
        {period === "week" && (
          <div>
            {/* 每日次数柱状图（CSS 实现，高度按封顶完成率） */}
            <div className="flex h-24 items-end gap-2">
              {ov.days.map((d) => (
                <button
                  key={d.logical_day}
                  className="flex flex-1 flex-col items-center gap-1"
                  onClick={() => setSelectedDay(d.logical_day)}
                  title={dayDetailText(d, task?.unit ?? "")}
                >
                  <div
                    className="w-full rounded-t-md"
                    style={{
                      height: `${Math.max(4, (d.capped_rate ?? 0) * 88)}px`,
                      background: `color-mix(in srgb, ${accent} ${d.display_state === "complete" ? 94 : d.display_state.startsWith("partial") ? 45 : 12}%, transparent)`,
                    }}
                  />
                  <span className={`text-[10px] ${d.is_today ? "font-bold text-accent" : "text-ink3"}`}>
                    {d.logical_day.slice(8)}
                  </span>
                </button>
              ))}
            </div>
            <div className="mt-3 flex gap-2">
              {ov.days.map((d) => (
                <HeatmapCell
                  key={d.logical_day}
                  state={d.display_state}
                  rate={d.capped_rate}
                  color={accent}
                  size={34}
                  isToday={d.is_today}
                  selected={selectedDay === d.logical_day}
                  title={dayDetailText(d, task?.unit ?? "")}
                  onClick={() => setSelectedDay(d.logical_day)}
                />
              ))}
            </div>
          </div>
        )}
        {period === "month" && (
          <div className="grid grid-cols-7 gap-1.5">
            {Array.from({ length: ov.days[0]?.weekday_index ?? 0 }).map((_, i) => (
              <div key={`pad${i}`} />
            ))}
            {ov.days.map((d) => (
              <div key={d.logical_day} className="flex flex-col items-center gap-0.5">
                <HeatmapCell
                  state={d.display_state}
                  rate={d.capped_rate}
                  color={accent}
                  size={26}
                  isToday={d.is_today}
                  prevYear={isPrevYear(d.logical_day)}
                  selected={selectedDay === d.logical_day}
                  title={dayDetailText(d, task?.unit ?? "")}
                  onClick={() => setSelectedDay(d.logical_day)}
                />
                <span className="text-[9px] tabular-nums text-ink3">{d.logical_day.slice(8)}</span>
              </div>
            ))}
          </div>
        )}
        {period === "year" && (
          <YearHeatmap
            days={ov.days}
            leadingEmpty={ov.leading_empty_count}
            color={accent}
            unit={task?.unit ?? ""}
            cellSize={12}
            gap={4}
            selected={selectedDay}
            onSelect={(d) => setSelectedDay(d.logical_day)}
          />
        )}
        {selected && (
          <div className="mt-3 rounded-xl bg-surface2 px-3 py-2 text-xs text-ink2">
            {dayDetailText(selected, task?.unit ?? "")}
          </div>
        )}
      </div>

      {/* 趋势桶 */}
      <div className="mt-4 rounded-2xl border border-line bg-surface p-4">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-ink2">
          {period === "week" ? "每日" : period === "month" ? "按周" : "按月"}完整完成率
        </h3>
        <div className="mt-3 flex h-20 items-end gap-1.5">
          {ov.buckets.map((b) => (
            <div key={b.label} className="flex flex-1 flex-col items-center gap-1">
              <div
                className="w-full rounded-t"
                style={{
                  height: `${Math.max(3, b.complete_day_rate * 72)}px`,
                  background: `color-mix(in srgb, ${accent} ${b.complete_day_rate >= 1 ? 94 : 45}%, transparent)`,
                }}
                title={`${b.label}：${Math.round(b.complete_day_rate * 100)}%（${b.complete_day_count}/${b.applicable_day_count} 天）`}
              />
              <span className="max-w-full truncate text-[9px] text-ink3">{b.label}</span>
            </div>
          ))}
        </div>
      </div>

      {/* 今日圆环小结 */}
      {view && view.target && (
        <div className="mt-4 flex items-center gap-4 rounded-2xl border border-line bg-surface p-4">
          <TaskRing value={view.count / view.target} color={accent} size={52}>
            <span className="text-[11px] font-semibold tabular-nums text-ink">
              {view.count}/{view.target}
            </span>
          </TaskRing>
          <div className="text-xs text-ink2">
            今日 {view.count} / {view.target} {task?.unit ?? ""}
            {view.can_undo && (
              <button
                className="ml-3 text-ink3 underline-offset-2 hover:text-ink hover:underline disabled:cursor-not-allowed disabled:opacity-50"
                disabled={undoBusy}
                onClick={() => {
                  if (undoBusy) return;
                  setUndoBusy(true);
                  api
                    .taskUndo(taskId)
                    .then(onChanged)
                    .catch((e) => toastError(String(e)))
                    .finally(() => setUndoBusy(false));
                }}
              >
                撤销最近打卡
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
