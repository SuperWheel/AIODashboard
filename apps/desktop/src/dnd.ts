/**
 * 任务卡片拖拽（006 v4.4）：Pointer Events + 独立浮层 + 实时落点预览。
 *
 * 设计：
 * - 指针移动超过阈值才进入拖拽；进入后立即从布局序列移除被拖卡，其他卡片通过
 *   FLIP 补位。起拖前先克隆卡片 DOM 到 body，浮层只跟随指针、不参与命中测试。
 * - 拖拽生命周期全部绑定在 document，不依赖会被 React 重排卸载的卡片节点。
 * - 指针进入有效落点后，才把被拖卡 id 插回预览序列；视图把它渲染为同形状的
 *   淡色圆角占位。pointerup 提交同一预览序列，Esc、pointercancel、失焦或墙外释放恢复。
 * - 命中使用排除 FLIP transform 的稳定布局坐标，并以 rAF、列优先、当前占位稳定区
 *   和目标切换距离抑制动画反向影响命中的反馈循环。
 * - FLIP 快照与预览更新强制在同一帧提交；让位使用可取消的 Web Animations，旧动画
 *   会从当前视觉位置无缝接续，且布局未变化的卡片不会被反复重启动画。
 * - 浮层清除克隆来的动画样式，以起拖卡位置为基准只做指针增量位移；命中外层与
 *   FLIP 视觉内层分离，确保抓取点始终贴住指针。
 */
import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { flushSync } from "react-dom";
import { api } from "./api";
import type { Task } from "./types";
import { confirmDialog, toastError } from "./components/DialogHost";

const DRAG_START_DISTANCE = 5;
const PREVIEW_REARM_DISTANCE = 10;
const PLACEHOLDER_HOLD_MARGIN = 12;
const DROP_BEFORE_RATIO = 0.68;
const DROP_SIDE_HYSTERESIS = 8;
const SAME_COLUMN_EPSILON = 1;
const FLIP_DURATION_MS = 240;
const FLIP_EPSILON = 0.5;
const DROP_ZONE_SELECTOR = '[data-task-dnd-zone="true"]';
const DROP_TARGET_SELECTOR = "[data-task-dnd-id]";

export interface DndRect {
  left: number;
  right: number;
  top: number;
  bottom: number;
}

interface PreviewPlacement {
  targetId: string;
  before: boolean;
  clientX: number;
  clientY: number;
}

interface PointerSession {
  id: string;
  pointerId: number;
  startX: number;
  startY: number;
  source: HTMLElement;
  sourceRect: DOMRect;
  sourceVacated: boolean;
  active: boolean;
  overDropZone: boolean;
  latestX: number;
  latestY: number;
  previewFrame: number | null;
  lastPlacement: PreviewPlacement | null;
}

/** 星级文案：0=未评级，N=N 星。 */
export function starText(n: number): string {
  return n > 0 ? `${n} 星` : "未评级";
}

/** 起拖后的布局序列：被拖卡立即离开原位，不保留原位占位。 */
export function liftFromSequence(ids: string[], dragId: string): string[] {
  return ids.filter((id) => id !== dragId);
}

/**
 * 把被拖卡插到目标卡之前/之后。输入既可以是刚起拖的“无被拖卡”序列，
 * 也可以是上一次已带占位的预览序列；结果始终只含一个 dragId。
 */
export function placeInSequence(
  ids: string[],
  dragId: string,
  targetId: string,
  before: boolean,
): string[] {
  const withoutDrag = liftFromSequence(ids, dragId);
  const targetIndex = withoutDrag.indexOf(targetId);
  if (targetIndex < 0) return ids;
  const next = [...withoutDrag];
  next.splice(targetIndex + (before ? 0 : 1), 0, dragId);
  return next;
}

/** 防止轻微手抖把普通点击误判成拖拽。 */
export function exceedsDragThreshold(
  startX: number,
  startY: number,
  clientX: number,
  clientY: number,
  threshold = DRAG_START_DISTANCE,
): boolean {
  const dx = clientX - startX;
  const dy = clientY - startY;
  return dx * dx + dy * dy >= threshold * threshold;
}

/**
 * 卡片的大部分区域代表“占据这个位置”，只有靠近底边时才代表放在其后；
 * 分界线带迟滞，避免光标在边界附近来回翻转。
 */
export function resolveDropBefore(
  clientY: number,
  rect: Pick<DndRect, "top" | "bottom">,
  previous: boolean | null,
  hysteresis = DROP_SIDE_HYSTERESIS,
): boolean {
  const height = rect.bottom - rect.top;
  const split = rect.top + height * DROP_BEFORE_RATIO;
  const deadZone = Math.min(hysteresis, Math.max(0, height / 6));
  if (previous === true && clientY <= split + deadZone) return true;
  if (previous === false && clientY >= split - deadZone) return false;
  return clientY < split;
}

/** 当前占位周围保留小幅稳定区，指针擦过边缘时不立即触发下一轮重排。 */
export function pointInExpandedRect(
  clientX: number,
  clientY: number,
  rect: DndRect,
  margin = PLACEHOLDER_HOLD_MARGIN,
): boolean {
  return (
    clientX >= rect.left - margin &&
    clientX <= rect.right + margin &&
    clientY >= rect.top - margin &&
    clientY <= rect.bottom + margin
  );
}

function axisDistance(value: number, start: number, end: number): number {
  if (value < start) return start - value;
  if (value > end) return value - end;
  return 0;
}

/**
 * 双列/多列列表先选水平方向最近的一列，再在该列按纵向选卡片。
 * 这样重排改变卡片的纵向位置时，不会把命中点突然吸到另一列。
 */
export function nearestRectIndex(
  rects: DndRect[],
  clientX: number,
  clientY: number,
): number {
  if (rects.length === 0) return -1;
  const horizontal = rects.map((rect) => axisDistance(clientX, rect.left, rect.right));
  const minHorizontal = Math.min(...horizontal);
  let bestIndex = -1;
  let bestVertical = Number.POSITIVE_INFINITY;
  for (let index = 0; index < rects.length; index += 1) {
    if (horizontal[index] > minHorizontal + SAME_COLUMN_EPSILON) continue;
    const rect = rects[index];
    const vertical = axisDistance(clientY, rect.top, rect.bottom);
    if (vertical < bestVertical) {
      bestIndex = index;
      bestVertical = vertical;
    }
  }
  return bestIndex;
}

/** 去掉元素自身 FLIP translate 后的真实布局坐标。 */
function stableElementRect(
  element: HTMLElement,
  visual = element.getBoundingClientRect(),
): DndRect {
  let translateX = 0;
  let translateY = 0;
  const transform = window.getComputedStyle(element).transform;
  if (transform && transform !== "none") {
    try {
      const matrix = new DOMMatrixReadOnly(transform);
      translateX = matrix.m41;
      translateY = matrix.m42;
    } catch {
      // 无法解析非标准 transform 时退回视觉坐标，拖拽仍可用。
    }
  }
  return {
    left: visual.left - translateX,
    right: visual.right - translateX,
    top: visual.top - translateY,
    bottom: visual.bottom - translateY,
  };
}

/** 提交只引用目标星级档内的邻居，不能拿其他档的 sort_order 算中点。 */
export function neighborsInBand<T extends { priority: number }>(
  ordered: T[],
  index: number,
  band: number,
): { before: T | null; after: T | null } {
  let before: T | null = null;
  let after: T | null = null;
  for (let i = index - 1; i >= 0; i -= 1) {
    if (ordered[i].priority === band) {
      before = ordered[i];
      break;
    }
  }
  for (let i = index + 1; i < ordered.length; i += 1) {
    if (ordered[i].priority === band) {
      after = ordered[i];
      break;
    }
  }
  return { before, after };
}

function sameSequence(a: string[], b: string[]): boolean {
  return a.length === b.length && a.every((id, i) => id === b[i]);
}

interface FlipSnapshot {
  /** 浏览器此刻真正绘制的位置，含尚未结束的动画 transform。 */
  visual: DOMRect;
  /** 扣除 transform 后的布局位置，用于判断这一轮是否真的换位。 */
  layout: DndRect;
}

/** FLIP：重排前记录视觉/布局位置，渲染后从当前视觉位置无缝接到新布局。 */
export function useFlip() {
  const refs = useRef(new Map<string, HTMLElement>());
  const snap = useRef<Map<string, FlipSnapshot> | null>(null);
  const animations = useRef(new Map<string, Animation>());
  const registerCallbacks = useRef(
    new Map<string, (el: HTMLElement | null) => void>(),
  );
  const register = (id: string) => {
    let callback = registerCallbacks.current.get(id);
    if (!callback) {
      callback = (el: HTMLElement | null) => {
        if (el) {
          el.dataset.taskDndFlip = "true";
          refs.current.set(id, el);
          return;
        }
        refs.current.delete(id);
        const animation = animations.current.get(id);
        if (animation) {
          animation.cancel();
          animations.current.delete(id);
        }
      };
      registerCallbacks.current.set(id, callback);
    }
    return callback;
  };
  /** 重排前调用：快照当前各卡位置。 */
  const capture = () => {
    snap.current = new Map(
      [...refs.current].map(([id, el]) => {
        const visual = el.getBoundingClientRect();
        return [id, { visual, layout: stableElementRect(el, visual) }];
      }),
    );
  };
  useLayoutEffect(() => {
    const s = snap.current;
    if (!s) return;
    snap.current = null;
    for (const [id, el] of refs.current) {
      const old = s.get(id);
      if (!old) continue;
      // 先在旧动画仍生效时算出稳定布局；随后取消旧动画，不会暴露中间跳帧。
      const now = stableElementRect(el);
      const layoutChanged =
        Math.abs(old.layout.left - now.left) >= FLIP_EPSILON ||
        Math.abs(old.layout.top - now.top) >= FLIP_EPSILON;
      // 本轮没有换位：让原动画自然完成，避免每次预览都把无关卡片重新计时。
      if (!layoutChanged) continue;

      const dx = old.visual.left - now.left;
      const dy = old.visual.top - now.top;
      const previous = animations.current.get(id);
      if (previous) {
        previous.cancel();
        animations.current.delete(id);
      }
      // 清掉 v4.3 可能遗留的 inline transition/transform；后续只有 WAAPI 写 transform。
      el.style.transition = "";
      el.style.transform = "";
      if (Math.abs(dx) < FLIP_EPSILON && Math.abs(dy) < FLIP_EPSILON) {
        el.style.willChange = "";
        continue;
      }

      el.style.willChange = "transform";
      const animation = el.animate(
        [
          { transform: `translate3d(${dx}px, ${dy}px, 0)` },
          { transform: "translate3d(0, 0, 0)" },
        ],
        {
          duration: FLIP_DURATION_MS,
          easing: "cubic-bezier(0.22, 1, 0.36, 1)",
        },
      );
      animations.current.set(id, animation);
      const finish = () => {
        if (animations.current.get(id) !== animation) return;
        animations.current.delete(id);
        if (refs.current.get(id) === el) el.style.willChange = "";
      };
      animation.onfinish = finish;
      animation.oncancel = finish;
    }
  });
  useEffect(
    () => () => {
      for (const animation of animations.current.values()) animation.cancel();
      animations.current.clear();
      refs.current.clear();
      registerCallbacks.current.clear();
    },
    [],
  );
  return { register, capture };
}

export function useTaskDnd(
  getSeq: () => Task[],
  onChanged: () => void,
  /** 提交成功后的新序列回调（用于先乐观落位，再刷新服务端数据）。 */
  onReordered?: (ids: string[]) => void,
) {
  const [dragId, setDragId] = useState<string | null>(null);
  /**
   * 拖拽中的布局序列：起拖时不含 dragId（原位腾空）；经过有效目标后在落点
   * 含 dragId（视图将它渲染为占位块）。null = 未在拖动/提交预览已结束。
   */
  const [preview, setPreview] = useState<string[] | null>(null);
  const [ghostH, setGhostH] = useState(0);
  const dragIdRef = useRef<string | null>(null);
  const previewRef = useRef<string[] | null>(null);
  const pointerSessionRef = useRef<PointerSession | null>(null);
  const pointerCleanupRef = useRef<(() => void) | null>(null);
  const overlayRef = useRef<HTMLElement | null>(null);
  const bodyStyleRef = useRef<{ userSelect: string; cursor: string } | null>(null);
  const finishingRef = useRef(false);
  const flip = useFlip();

  /** 展示序列：严格按预览重排；尚无落点时不会把被拖卡追加回原位。 */
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
    for (const [id, v] of byId) {
      if (id !== dragId) out.push(v);
    }
    return out;
  };

  const clearPreview = () => {
    dragIdRef.current = null;
    previewRef.current = null;
    setDragId(null);
    setPreview(null);
    setGhostH(0);
  };

  const restoreBodyStyle = () => {
    const previous = bodyStyleRef.current;
    if (!previous) return;
    document.body.style.userSelect = previous.userSelect;
    document.body.style.cursor = previous.cursor;
    bodyStyleRef.current = null;
  };

  const removeOverlay = (defer = false) => {
    const overlay = overlayRef.current;
    overlayRef.current = null;
    if (!overlay) return;
    if (!defer) {
      overlay.remove();
      return;
    }
    overlay.style.transition = "opacity 80ms ease";
    requestAnimationFrame(() => {
      overlay.style.opacity = "0";
      window.setTimeout(() => overlay.remove(), 80);
    });
  };

  const cleanupPointerTracking = (deferOverlay = false) => {
    const session = pointerSessionRef.current;
    if (session?.previewFrame !== null && session?.previewFrame !== undefined) {
      cancelAnimationFrame(session.previewFrame);
      session.previewFrame = null;
    }
    pointerCleanupRef.current?.();
    pointerCleanupRef.current = null;
    pointerSessionRef.current = null;
    removeOverlay(deferOverlay);
    restoreBodyStyle();
  };

  /** 松手后的点击是这次拖拽的尾事件，不能误触打卡/打开详情。 */
  const suppressTrailingClick = () => {
    const suppress = (e: MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      e.stopImmediatePropagation();
    };
    document.addEventListener("click", suppress, { capture: true, once: true });
    window.setTimeout(() => document.removeEventListener("click", suppress, true), 0);
  };

  const cancelDrag = () => {
    const active = pointerSessionRef.current?.active === true;
    if (active) flip.capture();
    cleanupPointerTracking(active);
    if (active) clearPreview();
  };

  /** pointerup 后提交；ref 保证最后一次 pointermove 与释放同帧时仍取最新落点。 */
  const commit = async () => {
    if (finishingRef.current) return;
    finishingRef.current = true;

    const id = dragIdRef.current;
    const seq = previewRef.current;
    // 占位块立即变为真实卡片，并保持在预览落点等待持久化。
    dragIdRef.current = null;
    setDragId(null);

    if (!id || !seq || !seq.includes(id)) {
      clearPreview();
      finishingRef.current = false;
      return;
    }

    const base = getSeq();
    const byId = new Map(base.map((t) => [t.id, t]));
    const ordered = seq
      .map((i) => byId.get(i))
      .filter((t): t is Task => t !== undefined);
    const idx = ordered.findIndex((t) => t.id === id);
    if (idx < 0 || sameSequence(ordered.map((t) => t.id), base.map((t) => t.id))) {
      clearPreview();
      finishingRef.current = false;
      return;
    }

    const drag = byId.get(id);
    if (!drag) {
      clearPreview();
      finishingRef.current = false;
      return;
    }
    const immediateBefore = ordered[idx - 1];
    const immediateAfter = ordered[idx + 1];
    // 目标档 = 落点下方卡片的星级（无下方 = 上方卡片；均无 = 自身档不变）。
    const band = immediateAfter?.priority ?? immediateBefore?.priority ?? drag.priority;
    const { before, after } = neighborsInBand(ordered, idx, band);

    try {
      if (band === drag.priority) {
        await api.moveTaskPosition(id, null, before?.id ?? null, after?.id ?? null);
      } else {
        const ok = await confirmDialog(
          "调整星级",
          `将「${drag.title}」从${starText(drag.priority)}调整为${starText(band)}并移动到此处？`,
        );
        if (!ok) {
          flip.capture();
          clearPreview();
          return;
        }
        await api.moveTaskPosition(id, band, before?.id ?? null, after?.id ?? null);
      }

      // 先把同一套预览序列写回当前页面，再撤掉 preview，避免释放后闪回旧序列。
      onReordered?.(seq);
      previewRef.current = null;
      setPreview(null);
      setGhostH(0);
      onChanged();
    } catch (e) {
      flip.capture();
      clearPreview();
      toastError(String(e));
    } finally {
      finishingRef.current = false;
    }
  };

  const previewAtTarget = (targetId: string, before: boolean): boolean => {
    const activeId = dragIdRef.current;
    if (!activeId || activeId === targetId) return false;
    const next = placeInSequence(
      previewRef.current ?? liftFromSequence(getSeq().map((t) => t.id), activeId),
      activeId,
      targetId,
      before,
    );
    if (previewRef.current && sameSequence(next, previewRef.current)) return false;
    flip.capture();
    previewRef.current = next;
    // 原生 pointer/rAF 回调里的普通 setState 仍可能进入并发调度；FLIP 快照只对紧随
    // 其后的这次 DOM 布局有效，因此在实际换位（不是每次 pointermove）时强制同帧提交。
    flushSync(() => setPreview(next));
    return true;
  };

  const nearestTarget = (
    zone: HTMLElement,
    clientX: number,
    clientY: number,
    activeId: string,
  ): { element: HTMLElement; rect: DndRect } | null => {
    const candidates = [...zone.querySelectorAll<HTMLElement>(DROP_TARGET_SELECTOR)]
      .filter((candidate) => candidate.dataset.taskDndId !== activeId)
      .map((element) => ({ element, rect: stableElementRect(element) }));
    const index = nearestRectIndex(
      candidates.map(({ rect }) => rect),
      clientX,
      clientY,
    );
    return index >= 0 ? candidates[index] : null;
  };

  const updatePreviewAtPoint = (clientX: number, clientY: number) => {
    const session = pointerSessionRef.current;
    if (!session?.active) return;
    // 起拖阈值刚满足时指针通常仍在原卡范围内；先保持原位真正腾空，离开后才开始落点预览。
    if (!session.sourceVacated) {
      if (pointInExpandedRect(clientX, clientY, session.sourceRect, 0)) {
        session.overDropZone = false;
        return;
      }
      session.sourceVacated = true;
    }
    const hit = document.elementFromPoint(clientX, clientY) as HTMLElement | null;
    const zone = hit?.closest<HTMLElement>(DROP_ZONE_SELECTOR) ?? null;
    session.overDropZone = zone !== null;
    if (!zone) {
      session.lastPlacement = null;
      return;
    }

    // 占位已落下时，以它为当前稳定槽；指针仍在槽内或擦过边缘都不重新命中其他卡片。
    if (previewRef.current?.includes(session.id)) {
      const placeholder = [...zone.querySelectorAll<HTMLElement>(DROP_TARGET_SELECTOR)].find(
        (candidate) => candidate.dataset.taskDndId === session.id,
      );
      if (
        placeholder &&
        pointInExpandedRect(clientX, clientY, stableElementRect(placeholder))
      ) {
        return;
      }
    }
    // elementFromPoint 命中的可能是仍在 FLIP 动画途中的卡片；统一按稳定布局坐标选目标。
    const target = nearestTarget(zone, clientX, clientY, session.id);
    const targetId = target?.element.dataset.taskDndId;
    if (!target || !targetId) return;

    const previous = session.lastPlacement;
    // 卡片因动画从光标下经过时 targetId 可能改变；光标未主动移动足够距离就不重排。
    if (
      previous &&
      previous.targetId !== targetId &&
      !exceedsDragThreshold(
        previous.clientX,
        previous.clientY,
        clientX,
        clientY,
        PREVIEW_REARM_DISTANCE,
      )
    ) {
      return;
    }
    const before = resolveDropBefore(
      clientY,
      target.rect,
      previous?.targetId === targetId ? previous.before : null,
    );
    const changed = previewAtTarget(targetId, before);
    if (
      changed ||
      !previous ||
      previous.targetId !== targetId ||
      previous.before !== before
    ) {
      session.lastPlacement = { targetId, before, clientX, clientY };
    }
  };

  const schedulePreviewAtPoint = (clientX: number, clientY: number) => {
    const session = pointerSessionRef.current;
    if (!session?.active) return;
    session.latestX = clientX;
    session.latestY = clientY;
    if (session.previewFrame !== null) return;
    session.previewFrame = requestAnimationFrame(() => {
      const current = pointerSessionRef.current;
      if (current !== session || !current.active) return;
      current.previewFrame = null;
      updatePreviewAtPoint(current.latestX, current.latestY);
    });
  };

  const moveOverlay = (clientX: number, clientY: number) => {
    const session = pointerSessionRef.current;
    const overlay = overlayRef.current;
    if (!session || !overlay) return;
    // 浮层基准点就是起拖时的卡片位置，只移动指针增量；避免从页面原点做大位移。
    const dx = clientX - session.startX;
    const dy = clientY - session.startY;
    overlay.style.transform = `translate3d(${dx}px, ${dy}px, 0)`;
  };

  const beginDrag = (session: PointerSession, clientX: number, clientY: number) => {
    const ids = getSeq().map((t) => t.id);
    if (!ids.includes(session.id)) {
      cleanupPointerTracking();
      return;
    }

    // 必须在 React 移除源卡之前创建视觉浮层；cloneNode 不复制事件监听器。
    const overlay = session.source.cloneNode(true) as HTMLElement;
    // 克隆会复制 FLIP 的 inline transform/transition；必须全部清掉，否则浮层会追赶鼠标。
    const inheritedFlipNodes = [
      ...(overlay.matches('[data-task-dnd-flip="true"]') ? [overlay] : []),
      ...overlay.querySelectorAll<HTMLElement>('[data-task-dnd-flip="true"]'),
    ];
    for (const node of inheritedFlipNodes) {
      node.removeAttribute("data-task-dnd-flip");
      node.style.transition = "none";
      node.style.transform = "none";
      node.style.willChange = "auto";
    }
    overlay.removeAttribute("data-task-dnd-id");
    overlay.setAttribute("data-task-dnd-overlay", "true");
    overlay.setAttribute("aria-hidden", "true");
    overlay.setAttribute("inert", "");
    Object.assign(overlay.style, {
      position: "fixed",
      left: `${session.sourceRect.left}px`,
      top: `${session.sourceRect.top}px`,
      width: `${session.sourceRect.width}px`,
      height: `${session.sourceRect.height}px`,
      margin: "0",
      zIndex: "9999",
      pointerEvents: "none",
      opacity: "0.96",
      transition: "none",
      animation: "none",
      transform: "translate3d(0, 0, 0)",
      transformOrigin: "0 0",
      filter: "none",
      borderRadius: "16px",
      boxShadow: "0 16px 28px rgb(15 23 42 / 0.22)",
      backfaceVisibility: "hidden",
      willChange: "transform",
    });
    document.body.appendChild(overlay);
    overlayRef.current = overlay;

    bodyStyleRef.current = {
      userSelect: document.body.style.userSelect,
      cursor: document.body.style.cursor,
    };
    document.body.style.userSelect = "none";
    document.body.style.cursor = "grabbing";

    const lifted = liftFromSequence(ids, session.id);
    session.active = true;
    session.sourceVacated = false;
    session.overDropZone = false;
    session.latestX = clientX;
    session.latestY = clientY;
    session.lastPlacement = null;
    flip.capture();
    dragIdRef.current = session.id;
    previewRef.current = lifted;
    moveOverlay(clientX, clientY);
    // 起拖也必须先完成“原位腾空”，再用新 DOM 计算首个落点；否则会命中即将卸载的 source。
    flushSync(() => {
      setGhostH(session.sourceRect.height);
      setDragId(session.id);
      setPreview(lifted);
    });
  };

  const startPointerTracking = (id: string, e: ReactPointerEvent<HTMLElement>) => {
    if (finishingRef.current || e.button !== 0 || !e.isPrimary) return;
    const ids = getSeq().map((t) => t.id);
    if (!ids.includes(id)) return;

    cleanupPointerTracking();
    const source = e.currentTarget;
    const rect = source.getBoundingClientRect();
    const session: PointerSession = {
      id,
      pointerId: e.pointerId,
      startX: e.clientX,
      startY: e.clientY,
      source,
      sourceRect: rect,
      sourceVacated: false,
      active: false,
      overDropZone: false,
      latestX: e.clientX,
      latestY: e.clientY,
      previewFrame: null,
      lastPlacement: null,
    };
    pointerSessionRef.current = session;

    const onPointerMove = (event: PointerEvent) => {
      const current = pointerSessionRef.current;
      if (!current || event.pointerId !== current.pointerId) return;
      if (!current.active) {
        if (
          !exceedsDragThreshold(
            current.startX,
            current.startY,
            event.clientX,
            event.clientY,
          )
        ) {
          return;
        }
        beginDrag(current, event.clientX, event.clientY);
      }
      if (!pointerSessionRef.current?.active) return;
      event.preventDefault();
      moveOverlay(event.clientX, event.clientY);
      schedulePreviewAtPoint(event.clientX, event.clientY);
    };

    const onPointerUp = (event: PointerEvent) => {
      const current = pointerSessionRef.current;
      if (!current || event.pointerId !== current.pointerId) return;
      if (!current.active) {
        cleanupPointerTracking();
        return;
      }
      event.preventDefault();
      suppressTrailingClick();
      if (current.previewFrame !== null) {
        cancelAnimationFrame(current.previewFrame);
        current.previewFrame = null;
      }
      updatePreviewAtPoint(event.clientX, event.clientY);
      const shouldCommit =
        current.overDropZone && previewRef.current?.includes(current.id) === true;
      if (!shouldCommit) {
        cancelDrag();
        return;
      }
      cleanupPointerTracking(true);
      void commit();
    };

    const onPointerCancel = (event: PointerEvent) => {
      if (event.pointerId === pointerSessionRef.current?.pointerId) cancelDrag();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      cancelDrag();
    };
    const onWindowBlur = () => cancelDrag();

    document.addEventListener("pointermove", onPointerMove, { passive: false });
    document.addEventListener("pointerup", onPointerUp);
    document.addEventListener("pointercancel", onPointerCancel);
    document.addEventListener("keydown", onKeyDown);
    window.addEventListener("blur", onWindowBlur);
    pointerCleanupRef.current = () => {
      document.removeEventListener("pointermove", onPointerMove);
      document.removeEventListener("pointerup", onPointerUp);
      document.removeEventListener("pointercancel", onPointerCancel);
      document.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("blur", onWindowBlur);
    };
  };

  useEffect(
    () => () => {
      const session = pointerSessionRef.current;
      if (session?.previewFrame !== null && session?.previewFrame !== undefined) {
        cancelAnimationFrame(session.previewFrame);
      }
      pointerCleanupRef.current?.();
      pointerCleanupRef.current = null;
      pointerSessionRef.current = null;
      removeOverlay();
      restoreBodyStyle();
    },
    [],
  );

  const wrapperProps = (id: string) => ({
    "data-task-dnd-id": id,
    onPointerDown: (e: ReactPointerEvent<HTMLElement>) => startPointerTracking(id, e),
  });

  const placeholderProps = (id: string) => ({
    "data-task-dnd-id": id,
    "aria-hidden": true,
  });

  const dropZoneProps = {
    "data-task-dnd-zone": "true",
  } as const;

  const isDragging = (id: string) => dragId === id;

  return {
    preview,
    ghostH,
    dragging: dragId !== null,
    order,
    wrapperProps,
    placeholderProps,
    dropZoneProps,
    isDragging,
    flipRegister: flip.register,
  };
}
