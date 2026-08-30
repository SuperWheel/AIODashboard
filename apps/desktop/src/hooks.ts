import { useEffect, useRef } from "react";

/**
 * 轮询 + 窗口聚焦时自动刷新。
 * CLI / AI 修改数据库后，GUI 会在下一个轮询周期自动同步（SQLite WAL 共享）。
 */
export function usePolling(fn: () => void, intervalMs = 4000, deps: unknown[] = []) {
  const saved = useRef(fn);
  saved.current = fn;

  useEffect(() => {
    const run = () => {
      if (document.visibilityState === "visible") saved.current();
    };
    run();
    const t = setInterval(run, intervalMs);
    window.addEventListener("focus", run);
    return () => {
      clearInterval(t);
      window.removeEventListener("focus", run);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, intervalMs]);
}

export function fmtDateTime(iso?: string | null): string {
  if (!iso) return "-";
  const d = new Date(iso);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getMonth() + 1}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export function isOverdue(dueAt?: string | null, status?: string): boolean {
  if (!dueAt || status === "done") return false;
  return new Date(dueAt).getTime() < Date.now();
}
