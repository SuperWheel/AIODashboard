/**
 * 任务卡片视觉规格（移植自 PlanningDays，贴合本仓库语义 token 双主题）。
 *
 * 任务主题色 hex 存库，色阶用 color-mix 作用于透明底——双主题自动成立，不新增 token。
 * 「颜色不是唯一通道」：不适用=虚线框、今天=加粗边框、未来=小圆点、0 次=横杠、
 * 部分完成=单/双点、完成=✓。
 */
import type { HeatmapState } from "./types";

export const TASK_COLORS: { hex: string; name: string }[] = [
  { hex: "#4A90E2", name: "海蓝" },
  { hex: "#38A38A", name: "青绿" },
  { hex: "#69A84F", name: "草绿" },
  { hex: "#E49345", name: "暖橙" },
  { hex: "#D96767", name: "珊瑚" },
  { hex: "#8067C8", name: "紫罗兰" },
];

export function taskColor(hex: string): string {
  return /^#[0-9a-fA-F]{6}$/.test(hex) ? hex : TASK_COLORS[0].hex;
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

/** 格内符号标记：颜色之外的第二通道。 */
export function stateMarker(state: HeatmapState): "dot" | "dash" | "dots1" | "dots2" | "check" | null {
  switch (state) {
    case "future":
      return "dot";
    case "zero":
      return "dash";
    case "partial_low":
      return "dots1";
    case "partial_high":
      return "dots2";
    case "complete":
      return "check";
    default:
      return null;
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
