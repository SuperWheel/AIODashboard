import { useEffect, useState, type ReactNode } from "react";
import { api } from "../api";
import { localToday } from "../hooks";
import type { TaskDayView, TodayContext } from "../types";
import CheckinRow from "./CheckinRow";
import QuickCapture from "./QuickCapture";
import { Badge, Card, Empty, ProgressRing, StatCard } from "./ui";
import { toastError } from "./DialogHost";

const GROUPS: { key: string; label: string; match: (s: string) => boolean }[] = [
  { key: "in_progress", label: "进行中", match: (s) => s === "in_progress" },
  { key: "pending", label: "未完成", match: (s) => s === "pending" },
  { key: "completed", label: "已完成", match: (s) => s === "completed" },
];

/**
 * Today = Bento 总控台：今日任务为打卡式列表（进行中/未完成/已完成分组）。
 * ⌘Z 撤销本次会话最近一次打卡。
 */
export default function TodayView({
  data,
  onChanged,
  onNav,
  extraCards,
}: {
  data: TodayContext | null;
  onChanged: () => void;
  onNav: (v: string, param?: string) => void;
  /** 插件卡片槽位（ModuleRegistry 注入，已按 size 包好网格占位） */
  extraCards?: ReactNode;
}) {
  const hour = new Date().getHours();
  const greet = hour < 6 ? "夜深了" : hour < 12 ? "早上好" : hour < 18 ? "下午好" : "晚上好";

  const total = data?.stats.task_total ?? 0;
  const done = data?.stats.completed_today ?? 0;
  const rate = data?.stats.completion_rate ?? 0;
  const missed = data?.stats.missed_last_7d ?? 0;
  const inbox = data?.open_inbox_count ?? 0;
  const todayTasks = data?.today_tasks ?? [];

  // ⌘Z：撤销本次会话最近一次打卡的任务
  const [lastActionTask, setLastActionTask] = useState<string | null>(null);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "z" && lastActionTask) {
        e.preventDefault();
        api
          .taskUndo(lastActionTask)
          .then(() => {
            setLastActionTask(null);
            onChanged();
          })
          .catch((e) => toastError(String(e)));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [lastActionTask, onChanged]);

  const afterAction = (taskId: string) => {
    setLastActionTask(taskId);
    onChanged();
  };

  return (
    <div className="grid grid-cols-1 gap-4 lg:grid-cols-12">
      {/* 问候 + 今日完成率 */}
      <Card className="flex items-center gap-5 p-5 lg:col-span-4">
        <ProgressRing value={rate} size={64} stroke={6} />
        <div className="min-w-0">
          <h1 className="text-lg font-semibold text-ink">{greet}</h1>
          <div className="mt-0.5 text-xs text-ink3">{data?.date ?? localToday()}</div>
          <div className="mt-1.5 text-xs text-ink2">
            今日达标{" "}
            <span className="font-medium tabular-nums text-accent">{done}</span>
            <span className="tabular-nums"> / {total}</span>
            <span className="ml-2 tabular-nums text-ink3">{Math.round(rate * 100)}%</span>
          </div>
        </div>
      </Card>

      {/* 快速捕捉 */}
      <Card className="p-4 lg:col-span-4">
        <QuickCapture onChanged={onChanged} />
      </Card>

      {/* 统计 2×2 迷你卡 */}
      <div className="grid grid-cols-2 gap-3 lg:col-span-4">
        <StatCard label="今日任务" value={data ? total : "-"} tone="blue" onClick={() => onNav("tasks")} />
        <StatCard label="已达标" value={data ? done : "-"} tone="green" onClick={() => onNav("tasks")} />
        <StatCard label="本周错过" value={data ? missed : "-"} tone="red" onClick={() => onNav("tasks")} />
        <StatCard label="收件箱" value={data ? inbox : "-"} tone="violet" onClick={() => onNav("inbox")} />
      </div>

      {/* 今日任务主卡（打卡式，分组展示） */}
      <Card className="lg:col-span-8">
        <div className="flex items-center justify-between border-b border-line px-4 py-3">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-ink2">今日任务</h2>
          <span className="text-xs tabular-nums text-ink3">
            {done}/{total} 达标
          </span>
        </div>

        {!data ? (
          <div className="p-4 text-sm text-ink3">加载中…</div>
        ) : todayTasks.length === 0 ? (
          <div className="p-4">
            <Empty
              text="今天没有进行中的任务"
              glyph="☀"
              action={
                <button
                  className="text-xs text-accent hover:underline"
                  onClick={() => onNav("tasks")}
                >
                  去新建一个打卡任务 →
                </button>
              }
            />
          </div>
        ) : (
          GROUPS.map((g) => {
            const items = todayTasks.filter((v: TaskDayView) => g.match(v.state));
            if (items.length === 0) return null;
            return (
              <div key={g.key}>
                <div className="px-4 pt-2.5 text-[11px] font-medium text-ink3">
                  {g.label} {items.length}
                </div>
                {items.map((v) => (
                  <CheckinRow
                    key={v.task.id}
                    view={v}
                    onChanged={() => afterAction(v.task.id)}
                    onOpenDetail={(id) => onNav("tasks", id)}
                  />
                ))}
              </div>
            );
          })
        )}
      </Card>

      {/* 右栏：最近笔记 + 活跃项目 */}
      <div className="flex flex-col gap-4 lg:col-span-4">
        <Card hoverable>
          <div className="flex items-center justify-between border-b border-line px-4 py-3">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-ink2">最近笔记</h2>
            <button
              className="text-xs text-ink3 hover:text-ink"
              onClick={() => onNav("notes")}
            >
              全部 →
            </button>
          </div>
          <div className="p-3">
            {data && data.recent_notes.length > 0 ? (
              <ul className="space-y-0.5">
                {data.recent_notes.map((n) => (
                  <li key={n.id}>
                    <button
                      onClick={() => onNav("notes", n.id)}
                      className="w-full truncate rounded-md px-1.5 py-1 text-left text-xs text-ink2 transition-colors hover:bg-hover hover:text-ink"
                    >
                      · {n.title || "(无标题)"}
                    </button>
                  </li>
                ))}
              </ul>
            ) : (
              <span className="text-xs text-ink3">暂无笔记</span>
            )}
          </div>
        </Card>

        <Card hoverable>
          <div className="flex items-center justify-between border-b border-line px-4 py-3">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-ink2">活跃项目</h2>
            <button
              className="text-xs text-ink3 hover:text-ink"
              onClick={() => onNav("projects")}
            >
              全部 →
            </button>
          </div>
          <div className="p-3">
            {data && data.active_projects.length > 0 ? (
              <div className="flex flex-wrap gap-1.5">
                {data.active_projects.map((p) => (
                  <button key={p.id} onClick={() => onNav("projects")} title="查看项目">
                    <Badge tone="blue">{p.name}</Badge>
                  </button>
                ))}
              </div>
            ) : (
              <span className="text-xs text-ink3">暂无活跃项目</span>
            )}
          </div>
        </Card>
      </div>

      {/* 插件卡片舞台（Bento 一等格位，按 size 占位） */}
      {extraCards}
    </div>
  );
}
