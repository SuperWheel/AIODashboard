/**
 * 任务卡片视觉规格（移植自 PlanningDays，贴合本仓库语义 token 双主题）。
 *
 * 任务主题色 hex 存库，色阶用 color-mix 作用于透明底——双主题自动成立，不新增 token。
 * 「颜色不是唯一通道」：不适用=虚线框、今天=加粗边框；格内一律纯色填充，不放符号标记。
 */
import type { HeatmapState } from "./types";

export const TASK_COLORS: { hex: string; name: string }[] = [
  { hex: "#4A90E2", name: "海蓝" },
  { hex: "#38BDF8", name: "天青" },
  { hex: "#6366F1", name: "靛蓝" },
  { hex: "#8067C8", name: "紫罗兰" },
  { hex: "#A855F7", name: "亮紫" },
  { hex: "#EC4899", name: "玫粉" },
  { hex: "#D96767", name: "珊瑚" },
  { hex: "#EF4444", name: "朱红" },
  { hex: "#E49345", name: "暖橙" },
  { hex: "#F5B942", name: "杏黄" },
  { hex: "#69A84F", name: "草绿" },
  { hex: "#10B981", name: "翠绿" },
  { hex: "#38A38A", name: "青绿" },
  { hex: "#92400E", name: "咖啡" },
  { hex: "#64748B", name: "岩灰" },
];

export function taskColor(hex: string): string {
  return /^#[0-9a-fA-F]{6}$/.test(hex) ? hex : TASK_COLORS[0].hex;
}

/** 热力图边框规则：以前年度的格子用虚线（与未来日同属"窗口边缘"视觉）。 */
export function isPrevYear(day: string): boolean {
  return day.slice(0, 4) < String(new Date().getFullYear());
}

/** 填充强度（色阶百分比），作用于 color-mix 的主题色占比。 */
export function fillIntensity(state: HeatmapState, rate: number | null): number {
  switch (state) {
    case "not_applicable":
      return 0;
    case "future":
      return 0;
    case "zero":
      return 14;
    case "partial_low":
    case "partial_high":
      return Math.round(18 + Math.min(1, rate ?? 0) * 74);
    case "complete":
      return 94;
  }
}

export const DAY_STATE_TEXT: Record<string, string> = {
  not_applicable: "不适用",
  pending: "尚未完成",
  in_progress: "进行中",
  completed: "已完成",
  missed: "已错过",
};

export const HEATMAP_LEGEND: { state: HeatmapState; label: string }[] = [
  { state: "not_applicable", label: "不适用" },
  { state: "future", label: "未到达" },
  { state: "zero", label: "0%" },
  { state: "partial_low", label: "1–49%" },
  { state: "partial_high", label: "50–99%" },
  { state: "complete", label: "100%" },
];

/** 打卡操作幂等键。 */
export function genOpId(): string {
  return `op_${crypto.randomUUID()}`;
}

const WEEKDAY_LABELS = ["一", "二", "三", "四", "五", "六", "日"] as const;

/** 循环规则的人类可读标签（004）：每天/每周一、三、五/每月/每年/一次性；daily 为空串。 */
export function recurrenceLabel(rec: { kind: string; weekdays?: number[] } | undefined): string {
  switch (rec?.kind) {
    case "weekly": {
      const ws = (rec.weekdays ?? []).slice().sort((a, b) => a - b);
      return `每周${ws.map((w) => WEEKDAY_LABELS[w - 1] ?? "?").join("、")}`;
    }
    case "monthly":
      return "每月";
    case "yearly":
      return "每年";
    case "once":
      return "一次性";
    default:
      return "";
  }
}
