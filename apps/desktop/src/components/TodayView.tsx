import type { ReactNode } from "react";
import { localToday } from "../hooks";
import type { TodayContext } from "../types";
import QuickCapture from "./QuickCapture";
import TaskRow from "./TaskRow";
import { Badge, Card, Empty, ProgressRing, StatCard } from "./ui";

/**
 * Today = Bento 总控台（docs/ui-redesign-2026-08.md §三）：
 * 12 列网格，今日任务为主卡，快速捕捉/统计/插件卡片平级格位；<lg 降级单列。
 */
export default function TodayView({
  data,
  onChanged,
  onNav,
  extraCards,
}: {
  data: TodayContext | null;
  onChanged: () => void;
  onNav: (v: string) => void;
  /** 插件卡片槽位（ModuleRegistry 注入，已按 size 包好网格占位） */
  extraCards?: ReactNode;
}) {
  const hour = new Date().getHours();
  const greet = hour < 6 ? "夜深了" : hour < 12 ? "早上好" : hour < 18 ? "下午好" : "晚上好";

  const total = data?.stats.today_total ?? 0;
  const done = data?.stats.completed_today ?? 0;
  const overdue = data?.stats.overdue_total ?? 0;
  const inbox = data?.open_inbox_count ?? 0;
  const overdueTasks = data?.overdue_tasks ?? [];
  const todayTasks = data?.today_tasks ?? [];

  return (
    <div className="grid grid-cols-1 gap-4 lg:grid-cols-12">
      {/* 问候 + 今日完成率 */}
      <Card className="flex items-center gap-5 p-5 lg:col-span-4">
        <ProgressRing value={total > 0 ? done / total : 0} size={64} stroke={6} />
        <div className="min-w-0">
          <h1 className="text-lg font-semibold text-ink">{greet}</h1>
          <div className="mt-0.5 text-xs text-ink3">{data?.date ?? localToday()}</div>
          <div className="mt-1.5 text-xs text-ink2">
            今日完成{" "}
            <span className="font-medium tabular-nums text-accent">{done}</span>
            <span className="tabular-nums"> / {total}</span>
            {overdue > 0 && <span className="ml-2 text-danger">逾期 {overdue}</span>}
          </div>
        </div>
      </Card>

      {/* 快速捕捉 */}
      <Card className="p-4 lg:col-span-4">
        <QuickCapture onChanged={onChanged} />
      </Card>

      {/* 统计 2×2 迷你卡 */}
      <div className="grid grid-cols-2 gap-3 lg:col-span-4">
        <StatCard label="今日待办" value={data ? total : "-"} tone="blue" />
        <StatCard label="已完成" value={data ? done : "-"} tone="green" />
        <StatCard label="已逾期" value={data ? overdue : "-"} tone="red" />
        <StatCard label="收件箱" value={data ? inbox : "-"} tone="violet" />
      </div>

      {/* 今日任务主卡（逾期置顶分组） */}
      <Card className="lg:col-span-8">
        <div className="flex items-center justify-between border-b border-line px-4 py-3">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-ink2">今日任务</h2>
          <span className="text-xs tabular-nums text-ink3">{todayTasks.length} 项</span>
        </div>

        {overdueTasks.length > 0 && (
          <div className="border-b border-danger/20 bg-danger/5">
            <div className="px-4 pt-2.5 text-[11px] font-medium text-danger">
              已逾期 {overdueTasks.length} 项
            </div>
            {overdueTasks.map((t) => (
              <TaskRow key={t.id} task={t} onChanged={onChanged} />
            ))}
          </div>
        )}

        {!data ? (
          <div className="p-4 text-sm text-ink3">加载中…</div>
        ) : todayTasks.length === 0 && overdueTasks.length === 0 ? (
          <div className="p-4">
            <Empty text="今天没有安排的任务" glyph="☀" />
          </div>
        ) : todayTasks.length === 0 ? (
          <div className="px-4 py-3 text-xs text-ink3">今日无其他任务</div>
        ) : (
          todayTasks.map((t) => <TaskRow key={t.id} task={t} onChanged={onChanged} />)
        )}
      </Card>

      {/* 右栏：最近笔记 + 活跃项目 */}
      <div className="flex flex-col gap-4 lg:col-span-4">
        <Card>
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
              <ul className="space-y-1.5">
                {data.recent_notes.map((n) => (
                  <li key={n.id} className="truncate text-xs text-ink2">
                    · {n.title || "(无标题)"}
                  </li>
                ))}
              </ul>
            ) : (
              <span className="text-xs text-ink3">暂无笔记</span>
            )}
          </div>
        </Card>

        <Card>
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
                  <Badge key={p.id} tone="blue">
                    {p.name}
                  </Badge>
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
