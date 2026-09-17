import { useCallback, useEffect, useRef, useState } from "react";
import {
  AlertCircle,
  CalendarCheck2,
  CheckCircle2,
  Flame,
  Gauge,
  TrendingUp,
  Zap,
  type LucideIcon,
} from "lucide-react";
import { api } from "../api";
import { localToday } from "../hooks";
import type { HeatmapDay, PeriodOverview, TaskDayView } from "../types";
import { taskColor } from "../taskVisual";
import { dayDetailText, MonthCalendarGrid, YearHeatmap } from "./Heatmap";
import { CheckinControls } from "./TaskCard";
import { Button, Card, DatePickerPanel } from "./ui";
import { toastError } from "./DialogHost";

type Period = "week" | "month" | "year";

const PERIODS: { key: Period; label: string }[] = [
  { key: "week", label: "周" },
  { key: "month", label: "月" },
  { key: "year", label: "年" },
];

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

/** 热力图/柱状图的状态色：格内永远不渲染文字。 */
function tileBg(d: HeatmapDay, accent: string): string {
  switch (d.display_state) {
    case "complete":
      return `color-mix(in srgb, ${accent} 92%, transparent)`;
    case "partial_low":
    case "partial_high":
      return `color-mix(in srgb, ${accent} 45%, transparent)`;
    case "zero":
      return `color-mix(in srgb, ${accent} 14%, transparent)`;
    case "not_applicable":
      return "color-mix(in srgb, var(--ink) 4.5%, transparent)";
    default:
      return "color-mix(in srgb, var(--ink) 6%, transparent)";
  }
}

/** 一组（已过去且适用的）日子的完整完成率；空组返回 null。 */
function groupRate(days: HeatmapDay[]): number | null {
  if (days.length === 0) return null;
  return days.filter((d) => d.display_state === "complete").length / days.length;
}

const fmtRate = (r: number | null) => (r === null ? "—" : `${Math.round(r * 100)}%`);

function applicableDays(days: HeatmapDay[]): HeatmapDay[] {
  return days.filter((d) => d.display_state !== "future" && d.display_state !== "not_applicable");
}

function targetProgress(days: HeatmapDay[]): number {
  const usable = applicableDays(days).filter((d) => d.target_count && d.target_count > 0);
  const target = usable.reduce((n, d) => n + (d.target_count ?? 0), 0);
  const actual = usable.reduce((n, d) => n + Math.min(d.actual_count, d.target_count ?? 0), 0);
  return target > 0 ? actual / target : 0;
}

function statusCounts(days: HeatmapDay[]) {
  const usable = applicableDays(days);
  return {
    complete: usable.filter((d) => d.display_state === "complete").length,
    partial: usable.filter((d) => d.display_state === "partial_low" || d.display_state === "partial_high").length,
    missed: usable.filter((d) => d.display_state === "zero").length,
    over: usable.filter((d) => d.is_overachieved).length,
  };
}

function BarChart({
  days,
  buckets,
  accent,
  mode,
  selected,
  onSelect,
}: {
  days?: HeatmapDay[];
  buckets?: PeriodOverview["buckets"];
  accent: string;
  mode: "day" | "bucket";
  selected: string | null;
  onSelect?: (value: string) => void;
}) {
  const items = mode === "day" ? (days ?? []).map((d) => ({
    key: d.logical_day,
    label: new Date(`${d.logical_day}T00:00:00`).toLocaleDateString("zh-CN", { weekday: "short" }),
    value: d.capped_rate ?? 0,
    detail: `${d.actual_count}/${d.target_count ?? "—"}`,
    day: d,
  })) : (buckets ?? []).map((b) => ({
    key: b.label,
    label: b.label,
    value: b.complete_day_rate,
    detail: `${Math.round(b.complete_day_rate * 100)}%`,
    day: undefined,
  }));
  return (
    <div className="mt-4" role="img" aria-label={mode === "day" ? "每日目标完成率柱状图" : "分段完成率柱状图"}>
      <div className="flex h-36 items-end gap-2 border-b border-line px-1">
        {items.map((item) => {
          const active = selected === item.key;
          return (
            <button
              key={item.key}
              type="button"
              className="group flex h-full min-w-0 flex-1 flex-col justify-end gap-1 rounded-t-md px-0.5 pt-2 hover:bg-hover"
              onClick={() => onSelect?.(item.key)}
              aria-pressed={active}
              title={`${item.key}：${item.detail}`}
            >
              <span className="text-[10px] tabular-nums text-ink3 opacity-0 transition-opacity group-hover:opacity-100">
                {item.detail}
              </span>
              <span className="relative flex min-h-1 flex-1 items-end rounded-t-md bg-hover">
                <span
                  className="block w-full rounded-t-md transition-[height]"
                  style={{
                    height: `${Math.max(4, item.value * 100)}%`,
                    background: item.day ? tileBg(item.day, accent) : `color-mix(in srgb, ${accent} 72%, transparent)`,
                    boxShadow: active ? `0 0 0 2px ${accent}` : undefined,
                  }}
                />
              </span>
              <span className="truncate text-[10px] text-ink3">{item.label}</span>
            </button>
          );
        })}
      </div>
      <div className="mt-2 flex justify-between text-[10px] text-ink3">
        <span>0%</span><span>目标完成率</span><span>100%</span>
      </div>
    </div>
  );
}

function InsightStat({
  icon: Icon,
  label,
  value,
  accent,
}: {
  icon: LucideIcon;
  label: string;
  value: string;
  accent: string;
}) {
  return (
    <div className="flex min-w-0 items-center gap-2 rounded-xl bg-surface2 px-2.5 py-2">
      <span
        className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg"
        style={{ background: `color-mix(in srgb, ${accent} 14%, transparent)`, color: accent }}
      >
        <Icon size={16} strokeWidth={2} />
      </span>
      <span className="min-w-0">
        <span className="block truncate text-sm font-semibold tabular-nums text-ink">{value}</span>
        <span className="block truncate text-[10px] text-ink3">{label}</span>
      </span>
    </div>
  );
}

function SelectedDaySummary({
  day,
  unit,
  accent,
}: {
  day: HeatmapDay | null;
  unit: string;
  accent: string;
}) {
  if (!day) {
    return (
      <div className="mt-4 flex items-center gap-2 rounded-xl bg-surface2 px-3 py-2 text-xs text-ink3">
        <Gauge size={15} />
        <span>选择一个日期查看完成进度</span>
      </div>
    );
  }
  const rate = Math.round((day.capped_rate ?? 0) * 100);
  const target = day.target_count ?? 0;
  return (
    <div className="mt-4 rounded-xl bg-surface2 px-3 py-2.5">
      <div className="flex items-center gap-2">
        <CalendarCheck2 size={15} style={{ color: accent }} />
        <span className="text-xs font-medium text-ink">{day.logical_day}</span>
        <span className="ml-auto text-xs tabular-nums text-ink2">{day.actual_count} / {target} {unit}</span>
      </div>
      <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-hover">
        <div className="h-full rounded-full" style={{ width: `${rate}%`, background: accent }} />
      </div>
    </div>
  );
}

/** 周期导航：◀ 标签 ▶ 一体分段控件（与任务页 DayNavigator 同规格，总高 28px）；
 *  标签点击弹日期面板跳任意日；非本期时显示「回到本期」。 */
function PeriodNavigator({
  period,
  anchor,
  label,
  isCurrent,
  onChange,
}: {
  period: Period;
  anchor: string;
  label: string;
  isCurrent: boolean;
  onChange: (a: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [open]);
  const segBtn =
    "flex h-6 items-center justify-center rounded-md text-xs text-ink2 transition-colors hover:bg-surface hover:text-ink";
  return (
    <div className="ml-auto flex items-center gap-2" ref={ref}>
      {!isCurrent && (
        <button
          className="rounded-lg px-2 py-1 text-xs text-accent transition-colors hover:bg-accent/10"
          onClick={() => onChange(localToday())}
        >
          回到本期
        </button>
      )}
      <div className="relative flex items-center rounded-lg bg-hover p-0.5">
        <button
          className={`${segBtn} w-6`}
          onClick={() => onChange(shiftAnchor(anchor, period, -1))}
          title="上一期"
        >
          ◀
        </button>
        <div className="relative">
          <button
            className={`${segBtn} px-2 tabular-nums ${isCurrent ? "" : "font-medium text-accent"}`}
            onClick={() => setOpen((v) => !v)}
            title="选择日期"
          >
            {label}
          </button>
          {open && (
            <div className="absolute left-1/2 top-8 z-30 -translate-x-1/2">
              <DatePickerPanel
                value={anchor}
                granularity={period === "week" ? "day" : period}
                onSelect={(d) => {
                  onChange(d);
                  setOpen(false);
                }}
              />
            </div>
          )}
        </div>
        <button
          className={`${segBtn} w-6`}
          onClick={() => onChange(shiftAnchor(anchor, period, 1))}
          title="下一期"
        >
          ▶
        </button>
      </div>
    </div>
  );
}

/**
 * 任务详情：三个周期视图按用户目的差异化——
 * 周 = 行动台（本周进度 + 7 天大状态格，今天即打卡入口）；
 * 月 = 日历复盘（真实日历排布 + 断链/工作日vs周末/上下月对比分析）；
 * 年 = 成就墙（GitHub 热力图 + 里程碑徽章 + 叙事统计 + 月度趋势）。
 * 统计卡随视图换指标。快捷键 ⌘1/2/3、⌥←/→、Esc 返回。
 */
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
  const [anchor, setAnchor] = useState<string>(() => localToday());
  const [ov, setOv] = useState<PeriodOverview | null>(null);
  const [prevOv, setPrevOv] = useState<PeriodOverview | null>(null);
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

  // 上一期总览：周/月/年的环比对比
  useEffect(() => {
    let alive = true;
    setPrevOv(null);
    api
      .taskOverview(taskId, period, shiftAnchor(anchor, period, -1))
      .then((r) => {
        if (alive) setPrevOv(r);
      })
      .catch(() => {
        if (alive) setPrevOv(null);
      });
    return () => {
      alive = false;
    };
  }, [taskId, period, anchor]);

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
  const today = localToday();
  const isCurrentPeriod = ov.start_day <= today && today <= ov.end_day;

  const counts = statusCounts(ov.days);
  const progressPct = Math.round(targetProgress(ov.days) * 100);
  const prevPct = prevOv ? Math.round(targetProgress(prevOv.days) * 100) : null;
  const diff = prevPct === null ? null : progressPct - prevPct;
  const diffText = diff === null ? "" : diff > 0 ? " ↑" : diff < 0 ? " ↓" : "";
  const pastApplicable = applicableDays(ov.days);
  const breakPoints = counts.partial + counts.missed;
  const weekendRate = groupRate(pastApplicable.filter((d) => d.weekday_index >= 5));
  const weekdayRate = groupRate(pastApplicable.filter((d) => d.weekday_index < 5));

  // 年：完美月（当月完成率 100% 的分桶）
  const perfectMonths = ov.buckets.filter((b) => b.applicable_day_count > 0 && b.complete_day_rate >= 1).length;

  // 统计卡随视图换指标（周=行动差距、月=复盘模式、年=长期证据）
  const stats =
    period === "week"
      ? [
          { label: "目标完成率", value: `${progressPct}%` },
          { label: "实际次数", value: `${s.actual_count} 次` },
          { label: "达标天数", value: `${s.complete_day_count}/${s.applicable_day_count}` },
          { label: "当前连续", value: s.current_streak },
        ]
      : period === "month"
        ? [
            { label: "目标完成率", value: `${progressPct}%` },
            { label: "达标天数", value: `${s.complete_day_count} 天` },
            { label: "断点", value: `${breakPoints} 天` },
            { label: "周末完成率", value: fmtRate(weekendRate) },
          ]
        : [
            { label: "目标完成率", value: `${progressPct}%` },
            { label: "累计打卡", value: s.actual_count },
            { label: "达标天数", value: `${s.complete_day_count} 天` },
            { label: "最长连续", value: `${s.longest_streak} 天` },
          ];

  return (
    <div>
      {/* 头部身份卡：返回 + 图标 + 标题/今日进度 + 打卡 + 编辑 */}
      <Card className="p-4">
        <div className="flex items-center gap-3">
          <button
            onClick={onBack}
            title="返回任务列表"
            className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-line bg-surface2 text-base text-ink2 transition-colors hover:bg-hover hover:text-ink"
          >
            ←
          </button>
          <div
            className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl text-base"
            style={{ background: `color-mix(in srgb, ${accent} 16%, transparent)` }}
          >
            {task?.icon || "✓"}
          </div>
          <div className="min-w-0 flex-1">
            <h1 className="truncate text-lg font-semibold text-ink">{task?.title ?? "任务"}</h1>
            <div className="text-xs text-ink3">
              {view?.target
                ? `今日 ${view.count} / ${view.target} ${task?.unit ?? ""}`
                : "今日不适用"}
              {view?.can_undo && (
                <button
                  className="ml-2 underline-offset-2 transition-colors hover:text-ink hover:underline disabled:cursor-not-allowed disabled:opacity-50"
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
          {view && <CheckinControls view={view} onChanged={onChanged} />}
          <Button variant="ghost" onClick={onEdit}>
            编辑
          </Button>
        </div>
      </Card>

      {/* 工具条：周/月/年 分段控件 + 周期导航（与任务页控件同规格、同高） */}
      <div className="mt-4 flex flex-wrap items-center justify-between gap-x-3 gap-y-2">
        <div className="flex items-center rounded-lg bg-hover p-0.5">
          {PERIODS.map((p) => (
            <button
              key={p.key}
              onClick={() => setPeriod(p.key)}
              className={`h-6 rounded-md px-3 text-xs transition-colors ${
                period === p.key
                  ? "bg-surface font-medium text-ink shadow-sm"
                  : "text-ink2 hover:text-ink"
              }`}
            >
              {p.label}
            </button>
          ))}
        </div>
        <PeriodNavigator
          period={period}
          anchor={anchor}
          label={periodLabel(ov)}
          isCurrent={isCurrentPeriod}
          onChange={setAnchor}
        />
      </div>

      {/* 统计条：指标随视图定制 */}
      <div className="mt-4 grid grid-cols-4 gap-3">
        {stats.map((x) => (
          <Card key={x.label} className="px-3 py-2.5">
            <div className="text-[11px] text-ink3">{x.label}</div>
            <div className="mt-0.5 text-lg font-semibold tabular-nums text-ink">{x.value}</div>
          </Card>
        ))}
      </div>

      {/* 主视图卡 */}
      <Card className="mt-4 p-4">
        {period === "week" && (
          <div>
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-sm font-semibold text-ink">本周完成节奏</h2>
                <p className="mt-1 text-xs text-ink3">柱高表示当天目标完成率，点击查看当天详情</p>
              </div>
              <span className="text-xs tabular-nums text-ink2">{progressPct}%</span>
            </div>
            <BarChart days={ov.days} accent={accent} mode="day" selected={selectedDay} onSelect={setSelectedDay} />
            <div className="mt-4 grid grid-cols-2 gap-2 text-xs text-ink2 sm:grid-cols-4">
              <InsightStat icon={CheckCircle2} label="达标天数" value={`${counts.complete} 天`} accent={accent} />
              <InsightStat icon={AlertCircle} label="漏做天数" value={`${counts.missed} 天`} accent={accent} />
              <InsightStat icon={TrendingUp} label="上周变化" value={prevPct === null ? "…" : `${diffText || "持平"}`} accent={accent} />
              <InsightStat icon={Zap} label="超额天数" value={`${counts.over} 天`} accent={accent} />
            </div>
          </div>
        )}

        {period === "month" && (
          <div>
            <div className="mb-3 flex items-center justify-between">
              <div>
                <h2 className="text-sm font-semibold text-ink">本月完成分布</h2>
                <p className="mt-1 text-xs text-ink3">格子只表达状态，点击日期查看记录</p>
              </div>
              <span className="text-xs tabular-nums text-ink2">{counts.complete} 达标 · {counts.partial} 部分 · {counts.missed} 漏做</span>
            </div>
            <div className="mx-auto grid w-full max-w-4xl items-center gap-8 lg:grid-cols-2">
              <MonthCalendarGrid
                days={ov.days}
                color={accent}
                dayTitle={(d) => dayDetailText(d, task?.unit ?? "")}
                selected={selectedDay}
                onSelect={(d) => setSelectedDay(d.logical_day)}
              />
              <div>
                <h3 className="text-xs font-medium text-ink2">每周完成率</h3>
                <BarChart
                  buckets={ov.buckets}
                  accent={accent}
                  mode="bucket"
                  selected={null}
                  onSelect={(label) => {
                    const bucket = ov.buckets.find((b) => b.label === label);
                    if (bucket) {
                      setPeriod("week");
                      setAnchor(bucket.start_day);
                      setSelectedDay(null);
                    }
                  }}
                />
              </div>
            </div>
            <div className="mt-3 grid grid-cols-2 gap-2 text-xs text-ink2 sm:grid-cols-4">
              <InsightStat icon={CalendarCheck2} label="工作日完成率" value={fmtRate(weekdayRate)} accent={accent} />
              <InsightStat icon={Flame} label="周末完成率" value={fmtRate(weekendRate)} accent={accent} />
              <InsightStat icon={AlertCircle} label="部分完成" value={`${counts.partial} 天`} accent={accent} />
              <InsightStat icon={Zap} label="超额完成" value={`${counts.over} 天`} accent={accent} />
            </div>
          </div>
        )}

        {period === "year" && (
          <div>
            <div className="mb-3 flex items-center justify-between">
              <div>
                <h2 className="text-sm font-semibold text-ink">长期完成轨迹</h2>
                <p className="mt-1 text-xs text-ink3">年度热力图保持无文字，点击月份趋势可继续复盘</p>
              </div>
              <span className="text-xs text-ink2">完美月 ×{perfectMonths}</span>
            </div>
            <YearHeatmap
              days={ov.days}
              leadingEmpty={ov.leading_empty_count}
              color={accent}
              unit={task?.unit ?? ""}
              gap={4}
              selected={selectedDay}
              onSelect={(d) => setSelectedDay(d.logical_day)}
            />
            <BarChart
              buckets={ov.buckets}
              accent={accent}
              mode="bucket"
              selected={null}
              onSelect={(label) => {
                const bucket = ov.buckets.find((b) => b.label === label);
                if (bucket) {
                  setPeriod("month");
                  setAnchor(bucket.start_day);
                  setSelectedDay(null);
                }
              }}
            />
            <div className="mt-3 grid grid-cols-2 gap-2 text-xs text-ink2 sm:grid-cols-4">
              <InsightStat icon={CheckCircle2} label="达标天数" value={`${s.complete_day_count} 天`} accent={accent} />
              <InsightStat icon={AlertCircle} label="漏做天数" value={`${counts.missed} 天`} accent={accent} />
              <InsightStat icon={Flame} label="最长连续" value={`${s.longest_streak} 天`} accent={accent} />
              <InsightStat icon={TrendingUp} label="完美月份" value={`×${perfectMonths}`} accent={accent} />
            </div>
          </div>
        )}

        <SelectedDaySummary day={selected} unit={task?.unit ?? ""} accent={accent} />
      </Card>

    </div>
  );
}
