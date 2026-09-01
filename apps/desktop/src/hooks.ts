import { useEffect, useRef, useState } from "react";

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

/** 视口是否不窄于 px（响应式列数等 JS 需要与 Tailwind 断点同步时用）。 */
export function useMinWidth(px: number): boolean {
  const [ok, setOk] = useState(() => window.matchMedia(`(min-width: ${px}px)`).matches);
  useEffect(() => {
    const mq = window.matchMedia(`(min-width: ${px}px)`);
    const on = () => setOk(mq.matches);
    mq.addEventListener("change", on);
    return () => mq.removeEventListener("change", on);
  }, [px]);
  return ok;
}

export function fmtDateTime(iso?: string | null): string {
  if (!iso) return "-";
  const d = new Date(iso);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getMonth() + 1}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** 本地时区的今天（YYYY-MM-DD），给 task due 等日期参数用；不用 UTC（跨午夜会偏一天）。 */
export function localToday(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

export function isOverdue(dueAt?: string | null, status?: string): boolean {
  if (!dueAt || status === "done") return false;
  return new Date(dueAt).getTime() < Date.now();
}
