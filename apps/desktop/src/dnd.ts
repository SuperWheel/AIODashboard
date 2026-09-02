/**
 * 任务卡片拖拽（006）：HTML5 DnD 共享逻辑（任务墙 / 首页今日卡共用）。
 *
 * 关键坑：Tauri 的 WKWebView（WebKit）要求 dragstart 时必须
 * `dataTransfer.setData()`，否则拖拽根本不会开始（Chrome 不需要）。
 */
import { useState, type DragEvent } from "react";
import { api } from "./api";
import type { Task } from "./types";
import { confirmDialog, toastError } from "./components/DialogHost";

/** 星级文案：0=未评级，N=N 星。 */
export function starText(n: number): string {
  return n > 0 ? `${n} 星` : "未评级";
}

export type DropTarget = { id: string; where: "before" | "after" };

/** 落点 → 服务端落位：同档直接调 move_task_position；跨档先弹确认框改级。 */
async function applyDragMove(
  seq: Task[],
  dragId: string,
  target: DropTarget,
  onChanged: () => void,
): Promise<void> {
  if (dragId === target.id) return;
  const drag = seq.find((t) => t.id === dragId);
  if (!drag) return;
  const others = seq.filter((t) => t.id !== dragId);
  const idx = others.findIndex((t) => t.id === target.id);
  if (idx < 0) return;
  const beforeId = target.where === "before" ? (others[idx - 1]?.id ?? null) : others[idx].id;
  const afterId = target.where === "before" ? others[idx].id : (others[idx + 1]?.id ?? null);
  // 目标档 = 落点下方卡片的星级（无下方 = 上方卡片；均无 = 自身档不变）
  const below = afterId ? others.find((t) => t.id === afterId) : undefined;
  const above = beforeId ? others.find((t) => t.id === beforeId) : undefined;
  const band = below?.priority ?? above?.priority ?? drag.priority;
  try {
    if (band === drag.priority) {
      await api.moveTaskPosition(dragId, null, beforeId, afterId);
    } else {
      const ok = await confirmDialog(
        "调整星级",
        `将「${drag.title}」从${starText(drag.priority)}调整为${starText(band)}并移动到此处？`,
      );
      if (!ok) return;
      await api.moveTaskPosition(dragId, band, beforeId, afterId);
    }
    onChanged();
  } catch (e) {
    toastError(String(e));
  }
}

/** 拖拽 hook：getSeq 在 drop 时取当前序列（避免闭包拿到旧数据）。 */
export function useTaskDnd(getSeq: () => Task[], onChanged: () => void) {
  const [dragId, setDragId] = useState<string | null>(null);
  const [dropMark, setDropMark] = useState<DropTarget | null>(null);

  /** 插入指示线位置（挂在目标卡片容器上）。 */
  const indicator = (id: string): "before" | "after" | null =>
    dropMark?.id === id ? dropMark.where : null;

  const wrapperProps = (id: string) => ({
    draggable: true,
    onDragStart: (e: DragEvent) => {
      setDragId(id);
      // WKWebView 必须 setData，否则拖拽不会开始
      e.dataTransfer.setData("text/plain", id);
      e.dataTransfer.effectAllowed = "move";
    },
    onDragEnd: () => {
      setDragId(null);
      setDropMark(null);
    },
    onDragOver: (e: DragEvent) => {
      if (!dragId || dragId === id) return;
      e.preventDefault();
      const r = e.currentTarget.getBoundingClientRect();
      setDropMark({ id, where: e.clientY < r.top + r.height / 2 ? "before" : "after" });
    },
    onDrop: (e: DragEvent) => {
      e.preventDefault();
      const mark = dropMark;
      const dragged = dragId;
      setDragId(null);
      setDropMark(null);
      if (dragged && mark) void applyDragMove(getSeq(), dragged, mark, onChanged);
    },
  });

  return { dragId, indicator, wrapperProps };
}
