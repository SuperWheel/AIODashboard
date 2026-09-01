import { useState } from "react";
import { api } from "../api";
import { usePolling } from "../hooks";
import type { LibraryYearHeatmap, TaskDayView } from "../types";
import { taskColor } from "../taskVisual";
import CheckinRow from "./CheckinRow";
import { RateHeatmapGrid } from "./Heatmap";
import { Button, Card, Empty } from "./ui";
import { toastError } from "./DialogHost";

type ArchiveMode = "keep" | "detach" | "move_to";

/** 重要日详情：日期概览 + 综合热力图 + 直属任务 + 归档三选一。 */
export default function LibraryDetailView({
  libraryId,
  refreshKey,
  onChanged,
  onBack,
  onNav,
}: {
  libraryId: string;
  refreshKey: number;
  onChanged: () => void;
  onBack: () => void;
  onNav: (view: string, param?: string) => void;
}) {
  const [lib, setLib] = useState<{
    library: import("../types").LibraryListItem | null;
  }>({ library: null });
  const [tasks, setTasks] = useState<TaskDayView[]>([]);
  const [heatmap, setHeatmap] = useState<LibraryYearHeatmap | null>(null);
  const [archiveOpen, setArchiveOpen] = useState(false);
  const [mode, setMode] = useState<ArchiveMode>("detach");
  const [moveTo, setMoveTo] = useState<string>("");
  const [allLibs, setAllLibs] = useState<import("../types").LibraryListItem[]>([]);

  const load = () => {
    api
      .listLibraries(true)
      .then((items) => {
        setAllLibs(items.filter((i) => i.status === "active" && i.id !== libraryId));
        setLib({ library: items.find((i) => i.id === libraryId) ?? null });
      })
      .catch(console.error);
    api
      .getToday()
      .then((t) => setTasks(t.today_tasks.filter((v) => v.library_id === libraryId)))
      .catch(console.error);
    api.libraryHeatmap(libraryId).then(setHeatmap).catch(console.error);
  };
  usePolling(load, 4000, [refreshKey, libraryId]);

  const it = lib.library;
  if (!it) return <div className="py-20 text-center text-sm text-ink3">加载中…</div>;
  const accent = taskColor(it.color_hex);
  const daysText =
    it.day_info.display_kind === "day_n"
      ? `第 ${it.day_info.day_count} 天`
      : it.day_info.display_kind === "remaining"
        ? `还剩 ${it.day_info.day_count} 天`
        : it.day_info.display_kind === "today"
          ? "就是今天"
          : `已逾期 ${it.day_info.day_count} 天`;

  const doArchive = async () => {
    try {
      await api.archiveLibrary(it.id, mode, mode === "move_to" ? moveTo : undefined);
      setArchiveOpen(false);
      onChanged();
      onBack();
    } catch (e) {
      toastError(String(e));
    }
  };

  return (
    <div>
      <div className="flex items-center gap-3">
        <Button variant="ghost" onClick={onBack}>← 返回</Button>
        <div
          className="flex h-12 w-12 items-center justify-center rounded-xl text-xl"
          style={{ background: `color-mix(in srgb, ${accent} 16%, transparent)` }}
        >
          {it.icon || "📅"}
        </div>
        <div className="min-w-0 flex-1">
          <h1 className="truncate text-lg font-semibold text-ink">{it.title}</h1>
          <div className="text-xs text-ink3">
            {it.kind === "anniversary" ? "纪念日" : "倒计时日"} · 锚点 {it.anchor_day}
            {it.status === "archived" && " · 已归档"}
          </div>
        </div>
        <div className="text-2xl font-semibold tabular-nums" style={{ color: accent }}>
          {daysText}
        </div>
      </div>

      {/* 综合热力图 */}
      <Card className="mt-5 p-4">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-ink2">
          综合热力图 · 近一年
        </h2>
        {heatmap && (
          <div className="mt-3">
            <RateHeatmapGrid
              days={heatmap.days}
              leadingEmpty={heatmap.leading_empty_count}
              color={accent}
            />
          </div>
        )}
        <p className="mt-2 text-[11px] text-ink3">
          逐日读取当时真实生效的归属与目标；当日完成率 = 有效任务贡献均值（单任务封顶 100%）。
        </p>
      </Card>

      {/* 直属任务 */}
      <Card className="mt-4">
        <div className="flex items-center justify-between border-b border-line px-4 py-3">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-ink2">直属任务</h2>
          <span className="text-xs tabular-nums text-ink3">{tasks.length} 项</span>
        </div>
        {tasks.length === 0 ? (
          <div className="p-4">
            <Empty text="暂无直属任务——在任务编辑器里把任务归入本重要日" glyph="☑" />
          </div>
        ) : (
          tasks.map((v) => (
            <CheckinRow key={v.task.id} view={v} onChanged={onChanged} onOpenDetail={(id) => onNav("tasks", id)} />
          ))
        )}
      </Card>

      {/* 归档 */}
      {it.status === "active" ? (
        <div className="mt-4">
          <Button variant="ghost" onClick={() => setArchiveOpen(true)}>
            归档重要日…
          </Button>
        </div>
      ) : (
        <div className="mt-4">
          <Button
            variant="ghost"
            onClick={() => api.restoreLibrary(it.id).then(() => { onChanged(); onBack(); }).catch((e) => toastError(String(e)))}
          >
            恢复重要日
          </Button>
        </div>
      )}

      {archiveOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30" onClick={() => setArchiveOpen(false)}>
          <div className="w-[400px] max-w-[92vw] rounded-2xl border border-line bg-surface p-5 shadow-lg" onClick={(e) => e.stopPropagation()}>
            <h2 className="text-sm font-semibold text-ink">归档「{it.title}」</h2>
            <p className="mt-1 text-xs text-ink3">直属 {tasks.length} 个任务如何处理？</p>
            <div className="mt-3 space-y-2">
              {(
                [
                  { key: "keep", label: "保留归属（任务仍显示在重要日历史中）" },
                  { key: "detach", label: "转为独立任务" },
                  { key: "move_to", label: "移动到另一个重要日" },
                ] as { key: ArchiveMode; label: string }[]
              ).map((m) => (
                <label key={m.key} className="flex cursor-pointer items-center gap-2 rounded-lg border border-line px-3 py-2 text-xs hover:bg-hover">
                  <input type="radio" checked={mode === m.key} onChange={() => setMode(m.key)} />
                  {m.label}
                </label>
              ))}
              {mode === "move_to" && (
                <select
                  value={moveTo}
                  onChange={(e) => setMoveTo(e.target.value)}
                  className="w-full rounded-lg border border-line bg-surface2 px-2.5 py-1.5 text-sm outline-none"
                >
                  <option value="">选择目标重要日…</option>
                  {allLibs.map((l) => (
                    <option key={l.id} value={l.id}>{l.title}</option>
                  ))}
                </select>
              )}
            </div>
            <div className="mt-4 flex justify-end gap-2">
              <Button variant="ghost" onClick={() => setArchiveOpen(false)}>取消</Button>
              <Button onClick={doArchive} disabled={mode === "move_to" && !moveTo}>确认归档</Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
