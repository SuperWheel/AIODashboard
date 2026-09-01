import { useEffect, useRef, type CSSProperties } from "react";
import type { AggregateHeatmapDay, HeatmapDay, HeatmapState } from "../types";
import { fillIntensity, isPrevYear, taskColor } from "../taskVisual";

/**
 * 热力格：纯色填充（格内无任何符号标记）。圆角矩形；色阶 color-mix；
 * 虚线=未来日/上一年度（窗口边缘），实线=本年已过；粗框=今天。
 * fluid 模式宽度撑满父级均分（aspect-square）。
 */
export function HeatmapCell({
  state,
  rate,
  color,
  size = 12,
  isToday = false,
  title,
  onClick,
  selected = false,
  fluid = false,
  prevYear = false,
}: {
  state: HeatmapState;
  rate?: number | null;
  color: string;
  size?: number;
  isToday?: boolean;
  title?: string;
  onClick?: () => void;
  selected?: boolean;
  /** 流体模式：宽度撑满父级（flex/grid 均分）；size 仅用于圆角推算 */
  fluid?: boolean;
  /** 上一年度的格子：虚线边框 */
  prevYear?: boolean;
}) {
  const accent = taskColor(color);
  const intensity = fillIntensity(state, rate ?? null);
  const radius = Math.min(size * 0.24, 6);

  let bg: string;
  let borderColor: string;
  if (state === "not_applicable") {
    bg = "color-mix(in srgb, var(--ink) 4.5%, transparent)";
    borderColor = "color-mix(in srgb, var(--ink) 26%, transparent)";
  } else if (state === "future") {
    bg = "color-mix(in srgb, var(--ink) 8%, transparent)";
    borderColor = "color-mix(in srgb, var(--ink) 26%, transparent)";
  } else {
    bg = `color-mix(in srgb, ${accent} ${intensity}%, transparent)`;
    borderColor = `color-mix(in srgb, ${accent} ${Math.max(intensity, 32)}%, transparent)`;
  }
  let dashed = state === "future" || prevYear;
  if (isToday) {
    borderColor = accent;
    dashed = false;
  }

  const base: CSSProperties = {
    borderRadius: radius,
    background: bg,
    border: `${isToday ? 1.8 : 0.8}px ${dashed ? "dashed" : "solid"} ${borderColor}`,
    boxShadow: selected ? `0 0 0 2px var(--ink)` : undefined,
  };
  const style: CSSProperties = fluid
    ? { ...base, width: "100%", aspectRatio: "1 / 1", flex: "1 1 0%", minWidth: 0 }
    : { ...base, width: size, height: size };

  return (
    <div
      style={style}
      title={title}
      onClick={onClick}
      role={onClick ? "button" : undefined}
      className={`relative shrink-0 ${onClick ? "cursor-pointer" : ""}`}
      aria-label={title}
    />
  );
}

/** 聚合热力图单日 hover 文案（重要日综合 / 全局首页共用）。 */
export function aggregateDayText(d: AggregateHeatmapDay): string {
  if (d.display_state === "rate") {
    return `${d.logical_day}：完成率 ${Math.round((d.rate ?? 0) * 100)}%（${d.active_task_count} 个任务）`;
  }
  return `${d.logical_day}：${d.display_state === "future" ? "尚未到达" : "不适用"}`;
}

/** 聚合热力图（滚动 53 周，rate 色阶 0–100%）：重要日综合 / 全局首页共用。
 *  color 支持 hex 预设或 CSS 变量（如 var(--accent)）。
 *  未来日与上一年度格子用虚线边框；右端列恒为本周（今天在其中）；首屏滚到最右。 */
export function RateHeatmapGrid({
  days,
  leadingEmpty,
  color,
  cellSize = 11,
  gap = 3,
  onDayClick,
}: {
  days: AggregateHeatmapDay[];
  leadingEmpty: number;
  color: string;
  cellSize?: number;
  gap?: number;
  onDayClick?: (d: AggregateHeatmapDay) => void;
}) {
  const accent = color.startsWith("var(") ? color : taskColor(color);
  const cells: (AggregateHeatmapDay | null)[] = [
    ...Array<null>(leadingEmpty).fill(null),
    ...days,
  ];
  const weeks: (AggregateHeatmapDay | null)[][] = [];
  for (let i = 0; i < cells.length; i += 7) weeks.push(cells.slice(i, i + 7));
  const scrollRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollLeft = el.scrollWidth;
  }, [days.length]);
  return (
    <div ref={scrollRef} className="overflow-x-auto pb-1" style={{ scrollbarWidth: "thin" }}>
      <div className="flex" style={{ gap }}>
        {weeks.map((wk, wi) => (
          <div key={wi} className="flex flex-col" style={{ gap }}>
            {wk.map((d, di) => {
              if (!d) return <div key={`e${di}`} style={{ width: cellSize, height: cellSize }} />;
              const dashed = d.display_state === "future" || isPrevYear(d.logical_day);
              const bg =
                d.display_state === "future"
                  ? "color-mix(in srgb, var(--ink) 8%, transparent)"
                  : d.display_state === "not_applicable"
                    ? "color-mix(in srgb, var(--ink) 4.5%, transparent)"
                    : `color-mix(in srgb, ${accent} ${Math.round(18 + (d.rate ?? 0) * 76)}%, transparent)`;
              return (
                <div
                  key={d.logical_day}
                  title={aggregateDayText(d)}
                  onClick={onDayClick ? () => onDayClick(d) : undefined}
                  role={onDayClick ? "button" : undefined}
                  className={onDayClick ? "cursor-pointer" : undefined}
                  style={{
                    width: cellSize,
                    height: cellSize,
                    borderRadius: 3,
                    background: bg,
                    border: d.is_today
                      ? `1.6px solid ${accent}`
                      : dashed
                        ? "0.8px dashed color-mix(in srgb, var(--ink) 30%, transparent)"
                        : "0.7px solid transparent",
                  }}
                />
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
}

/** 日期详情文案（hover/选中）。 */
export function dayDetailText(d: HeatmapDay, unit: string): string {
  const date = d.logical_day;
  switch (d.display_state) {
    case "not_applicable":
      return `${date}，不适用`;
    case "future":
      return `${date}，尚未到达${d.target_count ? `，目标 ${d.target_count} ${unit}` : ""}`;
    default: {
      const t = d.target_count ?? 0;
      const rate = Math.round((d.capped_rate ?? 0) * 100);
      if (d.is_overachieved) {
        return `${date}，实际 ${d.actual_count} ${unit}，目标 ${t} ${unit}，超额 ${d.actual_count - t} ${unit}`;
      }
      return `${date}，实际 ${d.actual_count} ${unit}，目标 ${t} ${unit}，完成率 ${rate}%`;
    }
  }
}

/** 年热力图（GitHub 式：7 行 × 53 周列，横向滚动，首屏定位最右）。
 *  窗口为滚动 53 周（右端列=本周）；未来日与上一年度格子虚线边框。 */
export function YearHeatmap({
  days,
  leadingEmpty,
  color,
  unit,
  cellSize = 12,
  gap = 4,
  selected,
  onSelect,
}: {
  days: HeatmapDay[];
  leadingEmpty: number;
  color: string;
  unit: string;
  cellSize?: number;
  gap?: number;
  selected?: string | null;
  onSelect?: (d: HeatmapDay) => void;
}) {
  // 按 (week_index, weekday_index) 摆格子；补窗口起点前的前置空格（滚动窗口恒为 0）
  const cells: (HeatmapDay | null)[] = [
    ...Array<HeatmapDay | null>(leadingEmpty).fill(null),
    ...days,
  ];
  const weeks: (HeatmapDay | null)[][] = [];
  for (let i = 0; i < cells.length; i += 7) {
    weeks.push(cells.slice(i, i + 7));
  }
  const scrollRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollLeft = el.scrollWidth;
  }, [days.length]);
  return (
    <div ref={scrollRef} className="overflow-x-auto pb-1" style={{ scrollbarWidth: "thin" }}>
      <div className="flex" style={{ gap }}>
        {weeks.map((wk, wi) => (
          <div key={wi} className="flex flex-col" style={{ gap }}>
            {wk.map((d, di) =>
              d ? (
                <HeatmapCell
                  key={d.logical_day}
                  state={d.display_state}
                  rate={d.capped_rate}
                  color={color}
                  size={cellSize}
                  isToday={d.is_today}
                  prevYear={isPrevYear(d.logical_day)}
                  selected={selected === d.logical_day}
                  title={dayDetailText(d, unit)}
                  onClick={onSelect ? () => onSelect(d) : undefined}
                />
              ) : (
                <div key={`e${di}`} style={{ width: cellSize, height: cellSize }} />
              ),
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
