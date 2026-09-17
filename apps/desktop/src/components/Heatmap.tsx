import { useEffect, useRef, useState, type CSSProperties, type RefObject } from "react";
import type { AggregateHeatmapDay, HeatmapDay, HeatmapState } from "../types";
import { fillIntensity, isPrevYear, taskColor } from "../taskVisual";

/** 窗口外（未来日/上一年度）色阶衰减：填充色稍淡。 */
const PALE_FACTOR = 0.45;

/** 月度网格补位格（末行不属于本月的空位）：比常规空格底色更淡的中性色。 */
const MONTH_PAD_BG = "color-mix(in srgb, var(--ink) 2%, transparent)";

/**
 * 热力格：纯色填充——无边框、无符号标记。
 *  未来日/上一年度填充色减淡；fluid 模式宽度撑满父级均分（aspect-square）。
 */
export function HeatmapCell({
  state,
  rate,
  color,
  size = 12,
  title,
  onClick,
  selected = false,
  fluid = false,
  prevYear = false,
  radius,
}: {
  state: HeatmapState;
  rate?: number | null;
  color: string;
  size?: number;
  /** 保留入参兼容调用方；今天不再用边框表达（位置=最右列即今天） */
  isToday?: boolean;
  title?: string;
  onClick?: () => void;
  selected?: boolean;
  /** 流体模式：宽度撑满父级（flex/grid 均分）；size 仅用于圆角推算 */
  fluid?: boolean;
  /** 上一年度的格子：填充色减淡 */
  prevYear?: boolean;
  /** 显式圆角（px）：fluid 格宽由父级决定、size 推算不准时使用 */
  radius?: number;
}) {
  const accent = taskColor(color);
  const intensity = fillIntensity(state, rate ?? null);
  const r = radius ?? Math.min(size * 0.24, 6);

  let bg: string;
  if (state === "not_applicable") {
    bg = "color-mix(in srgb, var(--ink) 4.5%, transparent)";
  } else if (state === "future") {
    bg = "color-mix(in srgb, var(--ink) 6%, transparent)";
  } else {
    const p = prevYear ? Math.round(intensity * PALE_FACTOR) : intensity;
    bg = `color-mix(in srgb, ${accent} ${p}%, transparent)`;
  }

  const base: CSSProperties = {
    borderRadius: r,
    background: bg,
    outline: selected ? "2px solid var(--ink)" : undefined,
    outlineOffset: selected ? 2 : undefined,
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

/** 实测容器宽度，按最小格宽决定显示最近多少周列，并反推精确格边长——
 *  格子恰好填满、不横向滚动；宽度不足时截断为最近 N 周（右端恒为本周、今天在其中）。 */
function useFitWeeks(
  totalWeeks: number,
  gap: number,
  minCell: number,
  fallback: number,
): {
  wrapRef: RefObject<HTMLDivElement>;
  cols: number;
  size: number;
} {
  const wrapRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver((es) => {
      for (const e of es) setWidth(e.contentRect.width);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  const cols =
    width > 0
      ? Math.max(1, Math.min(totalWeeks, Math.floor((width + gap) / (minCell + gap))))
      : totalWeeks;
  const size = width > 0 && cols > 0 ? (width - (cols - 1) * gap) / cols : fallback;
  return { wrapRef, cols, size };
}

/** 聚合热力图（滚动 53 周，rate 色阶 0–100%）：重要日综合 / 全局首页共用。
 *  color 支持 hex 预设或 CSS 变量（如 var(--accent)）。
 *  无边框；未来日/上一年度填充减淡；格子恰好填满容器宽度（不足时只显示最近 N 周）。 */
export function RateHeatmapGrid({
  days,
  leadingEmpty,
  color,
  gap = 3,
  onDayClick,
}: {
  days: AggregateHeatmapDay[];
  leadingEmpty: number;
  color: string;
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
  const { wrapRef, cols, size } = useFitWeeks(weeks.length, gap, 14, 11);
  const shownWeeks = weeks.slice(-cols);
  const radius = Math.min(size * 0.24, 6);

  return (
    <div ref={wrapRef} className="overflow-hidden">
      <div className="flex" style={{ gap }}>
        {shownWeeks.map((wk, wi) => (
          <div key={wi} className="flex flex-col" style={{ gap }}>
            {wk.map((d, di) => {
              if (!d) return <div key={`e${di}`} style={{ width: size, height: size }} />;
              const bg =
                d.display_state === "future"
                  ? "color-mix(in srgb, var(--ink) 6%, transparent)"
                  : d.display_state === "not_applicable"
                    ? "color-mix(in srgb, var(--ink) 4.5%, transparent)"
                    : `color-mix(in srgb, ${accent} ${Math.round(
                        (18 + (d.rate ?? 0) * 76) * (isPrevYear(d.logical_day) ? PALE_FACTOR : 1),
                      )}%, transparent)`;
              return (
                <div
                  key={d.logical_day}
                  title={aggregateDayText(d)}
                  onClick={onDayClick ? () => onDayClick(d) : undefined}
                  role={onDayClick ? "button" : undefined}
                  className={onDayClick ? "cursor-pointer" : undefined}
                  style={{
                    width: size,
                    height: size,
                    borderRadius: radius,
                    background: bg,
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

/**
 * 月度热力网格：行优先流水排布（不按日历对齐），格内无文字；
 * 每行固定 11 格（不随容器收窄减列），格子边长封顶 maxCell、
 * 更宽的容器中网格整体居中不继续放大；末行空位用
 * 更淡的中性补位格补成完整矩形（不空着）。任务卡月卡专用。
 */
export function MonthHeatmapGrid({
  days,
  color,
  gap = 5,
  maxCell = 44,
  dayTitle,
  selected,
  onSelect,
}: {
  days: HeatmapDay[];
  color: string;
  gap?: number;
  /** 格边长上限（px）：容器过宽时网格封顶居中，不继续放大 */
  maxCell?: number;
  dayTitle: (d: HeatmapDay) => string;
  selected?: string | null;
  onSelect?: (d: HeatmapDay) => void;
}) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver((es) => {
      for (const e of es) setWidth(e.contentRect.width);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  // 固定一行 11 格，不随容器收窄减列
  const cols = 11;
  const cell = width > 0 ? Math.min((width - (cols - 1) * gap) / cols, maxCell) : maxCell;
  const gridW = cell * cols + (cols - 1) * gap;
  const radius = Math.min(cell * 0.24, 6);
  if (days.length === 0) return null;
  // 末行空位（不属于本月的格子）：淡色补位，不空着
  const pads = (cols - (days.length % cols)) % cols;
  const padStyle: CSSProperties = {
    width: cell,
    height: cell,
    borderRadius: radius,
    background: MONTH_PAD_BG,
  };

  return (
    <div ref={wrapRef} className="flex justify-center">
      <div
        className="grid"
        style={{ width: gridW, gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))`, gap }}
      >
        {days.map((d) => (
          <HeatmapCell
            key={d.logical_day}
            state={d.display_state}
            rate={d.capped_rate}
            color={color}
            size={cell}
            radius={radius}
            isToday={d.is_today}
            prevYear={isPrevYear(d.logical_day)}
            selected={selected === d.logical_day}
            title={dayTitle(d)}
            onClick={onSelect ? () => onSelect(d) : undefined}
          />
        ))}
        {Array.from({ length: pads }, (_, i) => (
          <div key={`pad${i}`} style={padStyle} />
        ))}
      </div>
    </div>
  );
}

/** 详情页月视图日历表头（与 weekday_index 0=周一 … 6=周日 对齐）。 */
const CAL_COLS = ["一", "二", "三", "四", "五", "六", "日"] as const;

/**
 * 详情页月视图专用：日历对齐热力网格（按真实星期位置排布），
 * 格内无文字、边长封顶（默认 28px）、网格居中；首/末周溢出本月的
 * 日体外补位格（更淡中性色）补成完整矩形；周末列表头着色区分。
 */
export function MonthCalendarGrid({
  days,
  color,
  gap = 5,
  maxCell = 28,
  dayTitle,
  selected,
  onSelect,
}: {
  days: HeatmapDay[];
  color: string;
  gap?: number;
  /** 格边长上限（px） */
  maxCell?: number;
  dayTitle: (d: HeatmapDay) => string;
  selected?: string | null;
  onSelect?: (d: HeatmapDay) => void;
}) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver((es) => {
      for (const e of es) setWidth(e.contentRect.width);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  const cell = width > 0 ? Math.min((width - 6 * gap) / 7, maxCell) : maxCell;
  const gridW = cell * 7 + gap * 6;
  const radius = Math.min(cell * 0.28, 8);
  if (days.length === 0) return null;
  const lead = days[0].weekday_index;
  const trail = 6 - days[days.length - 1].weekday_index;
  const padStyle: CSSProperties = {
    width: cell,
    height: cell,
    borderRadius: radius,
    background: MONTH_PAD_BG,
  };

  return (
    <div ref={wrapRef} className="flex justify-center">
      <div style={{ width: gridW }}>
        <div className="mb-1.5 grid grid-cols-7" style={{ columnGap: gap }}>
          {CAL_COLS.map((w, i) => (
            <div
              key={w}
              className={`text-center text-[10px] leading-none ${
                i >= 5 ? "text-warn/80" : "text-ink3"
              }`}
            >
              {w}
            </div>
          ))}
        </div>
        <div className="grid" style={{ gridTemplateColumns: "repeat(7, minmax(0, 1fr))", gap }}>
          {Array.from({ length: lead }, (_, i) => (
            <div key={`lead${i}`} style={padStyle} />
          ))}
          {days.map((d) => (
            <HeatmapCell
              key={d.logical_day}
              state={d.display_state}
              rate={d.capped_rate}
              color={color}
              size={cell}
              radius={radius}
              isToday={d.is_today}
              prevYear={isPrevYear(d.logical_day)}
              selected={selected === d.logical_day}
              title={dayTitle(d)}
              onClick={onSelect ? () => onSelect(d) : undefined}
            />
          ))}
          {Array.from({ length: trail }, (_, i) => (
            <div key={`trail${i}`} style={padStyle} />
          ))}
        </div>
      </div>
    </div>
  );
}

/** 年热力图（GitHub 式：7 行 × N 周列，格子恰好填满容器宽度，不滚动）。
 *  窗口为滚动 53 周（右端列=本周）；容器宽度不足时只显示最近 N 周、保证格宽下限；
 *  未来日/上一年度格子填充减淡。 */
export function YearHeatmap({
  days,
  leadingEmpty,
  color,
  unit,
  gap = 4,
  selected,
  onSelect,
}: {
  days: HeatmapDay[];
  leadingEmpty: number;
  color: string;
  unit: string;
  gap?: number;
  selected?: string | null;
  onSelect?: (d: HeatmapDay) => void;
}) {
  const cells: (HeatmapDay | null)[] = [
    ...Array<HeatmapDay | null>(leadingEmpty).fill(null),
    ...days,
  ];
  const weeks: (HeatmapDay | null)[][] = [];
  for (let i = 0; i < cells.length; i += 7) {
    weeks.push(cells.slice(i, i + 7));
  }
  const { wrapRef, cols, size } = useFitWeeks(weeks.length, gap, 14, 8);
  const shownWeeks = weeks.slice(-cols);
  return (
    <div ref={wrapRef} className="overflow-visible px-1">
      <div className="flex" style={{ gap }}>
        {shownWeeks.map((wk, wi) => (
          <div key={wi} className="flex flex-col" style={{ gap }}>
            {wk.map((d, di) =>
              d ? (
                <HeatmapCell
                  key={d.logical_day}
                  state={d.display_state}
                  rate={d.capped_rate}
                  color={color}
                  size={size}
                  isToday={d.is_today}
                  prevYear={isPrevYear(d.logical_day)}
                  selected={selected === d.logical_day}
                  title={dayDetailText(d, unit)}
                  onClick={onSelect ? () => onSelect(d) : undefined}
                />
              ) : (
                <div key={`e${di}`} style={{ width: size, height: size }} />
              ),
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
