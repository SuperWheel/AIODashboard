import { useState, type ReactNode } from "react";
import { api } from "../api";
import type { TodayContext } from "../types";
import TaskRow from "./TaskRow";
import { Badge, Card, Empty, SectionTitle, StatCard } from "./ui";

export default function TodayView({
  data,
  onChanged,
  onNav,
  extraCards,
}: {
  data: TodayContext | null;
  onChanged: () => void;
  onNav: (v: string) => void;
  /** 插件卡片槽位（ModuleRegistry 注入） */
  extraCards?: ReactNode;
}) {
  const [title, setTitle] = useState("");
  const [dueToday, setDueToday] = useState(true);
  const [busy, setBusy] = useState(false);

  const todayStr = new Date().toISOString().slice(0, 10);

  const add = async () => {
    const t = title.trim();
    if (!t || busy) return;
    setBusy(true);
    try {
      await api.createTask(t, dueToday ? todayStr : undefined);
      setTitle("");
      onChanged();
    } catch (e) {
      alert(String(e));
    } finally {
      setBusy(false);
    }
  };

  const hour = new Date().getHours();
  const greet = hour < 6 ? "夜深了" : hour < 12 ? "早上好" : hour < 18 ? "下午好" : "晚上好";

  return (
    <div>
      <div className="flex items-baseline justify-between">
        <h1 className="text-xl font-semibold text-white">
          {greet}，今天 {data?.date ?? ""}
        </h1>
        <span className="text-xs text-slate-500">GUI · CLI · AI 共用同一 Rust Core</span>
      </div>

      {/* 统计 */}
      <div className="mt-5 grid grid-cols-4 gap-3">
        <StatCard label="今日待办" value={data?.stats.today_total ?? "-"} tone="blue" />
        <StatCard label="已完成" value={data?.stats.completed_today ?? "-"} tone="green" />
        <StatCard label="已逾期" value={data?.stats.overdue_total ?? "-"} tone="red" />
        <StatCard label="收件箱" value={data?.open_inbox_count ?? "-"} tone="violet" />
      </div>

      {/* 插件卡片槽位 */}
      {extraCards}

      {/* 快速添加 */}
      <SectionTitle>快速添加</SectionTitle>
      <div className="flex items-center gap-2 rounded-xl border border-white/10 bg-[#161a22] px-3 py-2">
        <input
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && add()}
          placeholder="输入任务标题，回车创建…"
          className="flex-1 bg-transparent text-sm outline-none placeholder:text-slate-600"
        />
        <label className="flex cursor-pointer items-center gap-1.5 text-xs text-slate-400">
          <input
            type="checkbox"
            checked={dueToday}
            onChange={(e) => setDueToday(e.target.checked)}
            className="accent-emerald-400"
          />
          今天到期
        </label>
        <button
          onClick={add}
          disabled={!title.trim() || busy}
          className="rounded-lg bg-emerald-500/90 px-3 py-1 text-xs font-medium text-[#0f1115] transition-colors hover:bg-emerald-400 disabled:opacity-40"
        >
          添加
        </button>
      </div>

      {/* 已逾期 */}
      {(data?.overdue_tasks.length ?? 0) > 0 && (
        <>
          <SectionTitle>已逾期</SectionTitle>
          <Card className="border-rose-400/20">
            {data!.overdue_tasks.map((t) => (
              <TaskRow key={t.id} task={t} onChanged={onChanged} />
            ))}
          </Card>
        </>
      )}

      {/* 今日任务 */}
      <SectionTitle>今日任务</SectionTitle>
      {!data ? (
        <Empty text="加载中…" />
      ) : data.today_tasks.length === 0 ? (
        <Empty text="今天没有安排的任务" />
      ) : (
        <Card>
          {data.today_tasks.map((t) => (
            <TaskRow key={t.id} task={t} onChanged={onChanged} />
          ))}
        </Card>
      )}

      {/* 底部：项目 / 笔记 / 收件箱预览 */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <SectionTitle
            right={
              <button className="text-xs text-slate-500 hover:text-slate-300" onClick={() => onNav("projects")}>
                全部 →
              </button>
            }
          >
            活跃项目
          </SectionTitle>
          <Card className="p-3">
            {data && data.active_projects.length > 0 ? (
              <div className="flex flex-wrap gap-1.5">
                {data.active_projects.map((p) => (
                  <Badge key={p.id} tone="blue">
                    {p.name}
                  </Badge>
                ))}
              </div>
            ) : (
              <span className="text-xs text-slate-600">暂无活跃项目</span>
            )}
          </Card>
        </div>
        <div>
          <SectionTitle
            right={
              <button className="text-xs text-slate-500 hover:text-slate-300" onClick={() => onNav("notes")}>
                全部 →
              </button>
            }
          >
            最近笔记
          </SectionTitle>
          <Card className="p-3">
            {data && data.recent_notes.length > 0 ? (
              <ul className="space-y-1.5">
                {data.recent_notes.map((n) => (
                  <li key={n.id} className="truncate text-xs text-slate-300">
                    · {n.title || "(无标题)"}
                  </li>
                ))}
              </ul>
            ) : (
              <span className="text-xs text-slate-600">暂无笔记</span>
            )}
          </Card>
        </div>
      </div>
    </div>
  );
}
