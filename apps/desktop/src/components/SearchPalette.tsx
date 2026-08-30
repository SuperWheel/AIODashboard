import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "../api";
import type { SearchHit, SearchKind } from "../types";
import type { RegisteredCommand } from "../plugins/registry";

const KIND_META: Record<SearchKind, { label: string; cls: string }> = {
  task: { label: "任务", cls: "bg-emerald-500/15 text-emerald-300" },
  project: { label: "项目", cls: "bg-sky-500/15 text-sky-300" },
  note: { label: "笔记", cls: "bg-violet-500/15 text-violet-300" },
  inbox: { label: "收件箱", cls: "bg-slate-500/15 text-slate-300" },
};

export default function SearchPalette({
  open,
  onClose,
  onNavigate,
  commands = [],
  onRunCommand,
}: {
  open: boolean;
  onClose: () => void;
  onNavigate: (kind: SearchKind) => void;
  /** 插件命令（ModuleRegistry 注入），空查询时全部展示、输入时按标题过滤 */
  commands?: RegisteredCommand[];
  onRunCommand: (cmd: RegisteredCommand) => void;
}) {
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const inputRef = useRef<HTMLInputElement>(null);
  const timer = useRef<number | null>(null);

  const matchedCommands = useMemo(() => {
    const query = q.trim().toLowerCase();
    if (!query) return commands;
    return commands.filter((c) => c.title.toLowerCase().includes(query));
  }, [q, commands]);

  useEffect(() => {
    if (open) {
      setQ("");
      setHits([]);
      setTimeout(() => inputRef.current?.focus(), 30);
    }
  }, [open]);

  useEffect(() => {
    if (timer.current) window.clearTimeout(timer.current);
    const query = q.trim();
    if (!query) {
      setHits([]);
      return;
    }
    timer.current = window.setTimeout(async () => {
      try {
        const r = await api.searchAll(query);
        setHits(r.hits);
      } catch (e) {
        console.error(e);
      }
    }, 200);
    return () => {
      if (timer.current) window.clearTimeout(timer.current);
    };
  }, [q]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/50 pt-[12vh]"
      onClick={onClose}
    >
      <div
        className="w-full max-w-xl overflow-hidden rounded-2xl border border-white/10 bg-[#161a22] shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <input
          ref={inputRef}
          value={q}
          onChange={(e) => setQ(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") onClose();
          }}
          placeholder="搜索任务、项目、笔记、收件箱…"
          className="w-full border-b border-white/10 bg-transparent px-5 py-4 text-sm outline-none placeholder:text-slate-600"
        />
        <div className="max-h-[50vh] overflow-y-auto p-2">
          {matchedCommands.length > 0 && (
            <div className="mb-1">
              <p className="px-3 pb-1 pt-2 text-[10px] font-medium uppercase tracking-wider text-slate-600">
                插件命令
              </p>
              {matchedCommands.map((c) => (
                <button
                  key={c.id}
                  onClick={() => onRunCommand(c)}
                  className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left hover:bg-white/[0.05]"
                >
                  <span className="shrink-0 rounded-md bg-amber-500/15 px-1.5 py-0.5 text-[10px] text-amber-300">
                    命令
                  </span>
                  <span className="min-w-0 flex-1 truncate text-sm text-slate-200">{c.title}</span>
                </button>
              ))}
            </div>
          )}
          {!q.trim() ? (
            matchedCommands.length > 0 ? null : (
              <p className="px-3 py-6 text-center text-xs text-slate-600">
                输入关键词，回车或点击结果跳转 · Esc 关闭
              </p>
            )
          ) : hits.length === 0 && matchedCommands.length === 0 ? (
            <p className="px-3 py-6 text-center text-xs text-slate-600">无匹配结果</p>
          ) : (
            hits.map((h) => (
              <button
                key={`${h.kind}-${h.id}`}
                onClick={() => onNavigate(h.kind)}
                className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left hover:bg-white/[0.05]"
              >
                <span
                  className={`shrink-0 rounded-md px-1.5 py-0.5 text-[10px] ${KIND_META[h.kind].cls}`}
                >
                  {KIND_META[h.kind].label}
                </span>
                <span className="min-w-0 flex-1 truncate text-sm text-slate-200">{h.title}</span>
                {h.subtitle && (
                  <span className="hidden max-w-[40%] truncate text-xs text-slate-500 sm:block">
                    {h.subtitle}
                  </span>
                )}
              </button>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
