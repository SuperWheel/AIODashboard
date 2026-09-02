/**
 * 任务卡片拖拽（006 第二版）：实时重排预览 + 淡色占位块 + FLIP 丝滑动画。
 *
 * 设计：
 * - 拖动过程中 onDragOver 实时算出预览序列（拖走插入目标前/后），其他卡片经
 *   FLIP 补间动画让位；被拖卡渲染为同形状淡色圆角矩形占位块 = 落点标记。
 * - 松手在 onDragEnd 提交（不依赖 drop 事件触发）：预览序列即落点，
 *   取被拖卡的上下邻居算 before/after；跨档弹确认框改级（见 006 spec）。
 *
 * 平台坑：WKWebView 的 HTML5 DnD 必须在 dragstart 调 dataTransfer.setData()，
 * 否则拖拽根本不启动（Chrome 不需要）。
 */
import { useLayoutEffect, useRef, useState, type DragEvent } from "react";
import { api } from "./api";
import type { Task } from "./types";
import { confirmDialog, toastError } from "./components/DialogHost";

/** 星级文案：0=未评级，N=N 星。 */
export function starText(n: number): string {
  return n > 0 ? `${n} 星` : "未评级";
}

/** FLIP：重排前 capture() 记录各卡位置，渲染后自动补间回放到新位置。 */
export function useFlip() {
  const refs = useRef(new Map<string, HTMLElement>());
  const snap = useRef<Map<string, DOMRect> | null>(null);
  const register = (id: string) => (el: HTMLElement | null) => {
    if (el) refs.current.set(id, el);
    else refs.current.delete(id);
  };
  /** 重排前调用：快照当前各卡位置。 */
  const capture = () => {
    snap.current = new Map(
      [...refs.current].map(([id, el]) => [id, el.getBoundingClientRect()]),
    );
  };
  useLayoutEffect(() => {
    const s = snap.current;
    if (!s) return;
    snap.current = null;
    for (const [id, el] of refs.current) {
      const old = s.get(id);
      if (!old) continue;
      const now = el.getBoundingClientRect();
      const dx = old.left - now.left;
      const dy = old.top - now.top;
      if (!dx && !dy) continue;
      el.style.transition = "none";
      el.style.transform = `translate(${dx}px, ${dy}px)`;
      requestAnimationFrame(() => {
        el.style.transition = "transform 0.18s ease";
        el.style.transform = "";
      });
    }
  });
  return { register, capture };
}

export function useTaskDnd(
  getSeq: () => Task[],
  onChanged: () => void,
  /** 提交成功后的新序列回调（首页用它更新会话级位置快照） */
  onReordered?: (ids: string[]) => void,
) {
  const [dragId, setDragId] = useState<string | null>(null);
  /** 拖拽中的预览序列（任务 id 顺序）；null = 未在拖动 */
  const [preview, setPreview] = useState<string[] | null>(null);
  /** 被拖卡的高度（占位块保持同形状） */
  const [ghostH, setGhostH] = useState(0);
  const flip = useFlip();

  /** 展示序列：预览优先（把 base 按预览 id 重排，缺漏项追加末尾）。 */
  const order = <T extends { task: Task }>(base: T[]): T[] => {
    if (!preview) return base;
    const byId = new Map(base.map((v) => [v.task.id, v]));
    const out: T[] = [];
    for (const id of preview) {
      const v = byId.get(id);
      if (v) {
        out.push(v);
        byId.delete(id);
      }
    }
    return [...out, ...byId.values()];
  };

  const commit = async () => {
    const id = dragId;
    const seq = preview;
    setDragId(null);
    setPreview(null);
    if (!id || !seq) return;
    const base = getSeq();
    const byId = new Map(base.map((t) => [t.id, t]));
    const ordered = seq
      .map((i) => byId.get(i))
      .filter((t): t is Task => t !== undefined);
    const idx = ordered.findIndex((t) => t.id === id);
    if (idx < 0) return;
    // 序列没变 = 原地松手
    if (ordered.map((t) => t.id).join() === base.map((t) => t.id).join()) return;
    const before = ordered[idx - 1];
    const after = ordered[idx + 1];
    const drag = byId.get(id);
    if (!drag) return;
    // 目标档 = 落点下方卡片的星级（无下方 = 上方卡片；均无 = 自身档不变）
    const band = after?.priority ?? before?.priority ?? drag.priority;
    try {
      if (band === drag.priority) {
        await api.moveTaskPosition(id, null, before?.id ?? null, after?.id ?? null);
      } else {
        const ok = await confirmDialog(
          "调整星级",
          `将「${drag.title}」从${starText(drag.priority)}调整为${starText(band)}并移动到此处？`,
        );
        if (!ok) return;
        await api.moveTaskPosition(id, band, before?.id ?? null, after?.id ?? null);
      }
      onReordered?.(seq);
      onChanged();
    } catch (e) {
      toastError(String(e));
    }
  };

  const wrapperProps = (id: string) => ({
    draggable: true,
    onDragStart: (e: DragEvent) => {
      setDragId(id);
      setGhostH((e.currentTarget as HTMLElement).offsetHeight);
      e.dataTransfer.setData("text/plain", id);
      e.dataTransfer.effectAllowed = "move";
    },
    onDragOver: (e: DragEvent) => {
      if (!dragId || dragId === id) return;
      e.preventDefault();
      const r = e.currentTarget.getBoundingClientRect();
      const before = e.clientY < r.top + r.height / 2;
      const cur = preview ?? getSeq().map((t) => t.id);
      const from = cur.indexOf(dragId);
      let to = cur.indexOf(id) + (before ? 0 : 1);
      if (from < 0) return;
      if (from < to) to -= 1; // 先移除自身，目标位要回退一格
      if (from === to) return;
      const next = [...cur];
      next.splice(from, 1);
      next.splice(to, 0, dragId);
      flip.capture();
      setPreview(next);
    },
    onDragEnd: () => {
      void commit();
    },
    onDrop: (e: DragEvent) => {
      e.preventDefault();
    },
  });

  const isDragging = (id: string) => dragId === id;

  return {
    dragId,
    preview,
    ghostH,
    order,
    wrapperProps,
    isDragging,
    flipRegister: flip.register,
  };
}
