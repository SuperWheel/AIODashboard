import type { CSSProperties } from "react";
import type { AggregateHeatmapDay, HeatmapDay, HeatmapState } from "../types";
import { fillIntensity, stateMarker, taskColor } from "../taskVisual";

/**
 * 热力格：六态渲染。圆角矩形；色阶 color-mix；虚线=不适用；粗框=今天；符号标记第二通道。
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
}: {
  state: HeatmapState;
  rate?: number | null;
  color: string;
  size?: number;
  isToday?: boolean;
  title?: string;
  onClick?: () => void;
  selected?: boolean;
}) {
  const accent = taskColor(color);
  const intensity = fillIntensity(state, rate ?? null);
  const radius = Math.min(size * 0.24, 6);

  let bg: string;
  let borderColor: string;
  let dashed = false;
  if (state === "not_applicable") {
    bg = "color-mix(in srgb, var(--ink) 4.5%, transparent)";
    borderColor = "color-mix(in srgb, var(--ink) 26%, transparent)";
    dashed = true;
  } else if (state === "future") {
    bg = "color-mix(in srgb, var(--ink) 8%, transparent)";
    borderColor = "color-mix(in srgb, var(--ink) 26%, transparent)";
  } else {
    bg = `color-mix(in srgb, ${accent} ${intensity}%, transparent)`;
    borderColor = `color-mix(in srgb, ${accent} ${Math.max(intensity, 32)}%, transparent)`;
  }
  if (isToday) {
    borderColor = accent;
    dashed = false;
  }

  const style: CSSProperties = {
    width: size,
    height: size,
    borderRadius: radius,
    background: bg,
    border: `${isToday ? 1.8 : 0.8}px ${dashed ? "dashed" : "solid"} ${borderColor}`,
    boxShadow: selected ? `0 0 0 2px var(--ink)` : undefined,
  };

  const marker = stateMarker(state);
  const m = size; // 标记尺寸随格子缩放
  return (
    <div
      style={style}
      title={title}
      onClick={onClick}
      role={onClick ? "button" : undefined}
      className={`relative shrink-0 ${onClick ? "cursor-pointer" : ""}`}
      aria-label={title}
    >
      {marker === "dot" && (
        <div
          className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-full"
          style={{
            width: m * 0.2,
            height: m * 0.2,
            background: "var(--ink-3)",
          }}
        />
      )}
      {marker === "dash" && (
        <div
          className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-full"
          style={{
            width: m * 0.42,
            height: Math.max(1, m * 0.1),
            background: accent,
          }}
        />
      )}
      {marker === "dots1" && (
        <div
          className="absolute rounded-full"
          style={{
            width: m * 0.22,
            height: m * 0.22,
            left: m * 0.18,
            bottom: m * 0.18,
            background: "color-mix(in srgb, var(--ink) 65%, transparent)",
          }}
        />
      )}
      {marker === "dots2" && (
        <div
          className="absolute flex gap-[1px]"
          style={{ left: m * 0.14, bottom: m * 0.18 }}
        >
          {[0, 1].map((i) => (
            <div
              key={i}
              className="rounded-full"
              style={{
                width: m * 0.2,
                height: m * 0.2,
                background: "color-mix(in srgb, var(--ink) 70%, transparent)",
              }}
            />
          ))}
        </div>
      )}
      {marker === "check" && (
        <svg
          className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2"
          width={m * 0.52}
          height={m * 0.52}
          viewBox="0 0 10 10"
        >
          <path
            d="M1.5 5.2 L4 7.5 L8.5 2.5"
            fill="none"
            stroke="white"
            strokeWidth={1.8}
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      )}
    </div>
  );
}

/** 聚合热力图单日 hover 文案（重要日综合 / 全局首页共用）。 */
export function aggregateDayText(d: AggregateHeatmapDay): string {
  if (d.display_state === "rate") {
    return `${d.logical_day}：完成率 ${Math.round((d.rate ?? 0) * 100)}%（${d.active_task_count} 个任务）`;
  }
  return `${d.logical_day}：${d.display_state === "future" ? "尚未到达" : "不适用"}`;
}

/** 聚合年度热力图（rate 色阶 0–100%）：重要日综合 / 全局首页共用。
 *  color 支持 hex 预设或 CSS 变量（如 var(--accent)）。 */
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
  return (
    <div className="overflow-x-auto pb-1" style={{ scrollbarWidth: "thin" }}>
      <div className="flex" style={{ gap }}>
        {weeks.map((wk, wi) => (
          <div key={wi} className="flex flex-col" style={{ gap }}>
            {wk.map((d, di) => {
              if (!d) return <div key={`e${di}`} style={{ width: cellSize, height: cellSize }} />;
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
                    border: d.is_today ? `1.6px solid ${accent}` : "0.7px solid transparent",
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

/** 年热力图（GitHub 式：7 行 × N 周列，横向滚动，首屏定位最近周）。 */
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
  // 按 (week_index, weekday_index) 摆格子；补年首空格
  const cells: (HeatmapDay | null)[] = [
    ...Array<HeatmapDay | null>(leadingEmpty).fill(null),
    ...days,
  ];
  const weeks: (HeatmapDay | null)[][] = [];
  for (let i = 0; i < cells.length; i += 7) {
    weeks.push(cells.slice(i, i + 7));
  }
  return (
    <div className="overflow-x-auto pb-1" style={{ scrollbarWidth: "thin" }}>
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
