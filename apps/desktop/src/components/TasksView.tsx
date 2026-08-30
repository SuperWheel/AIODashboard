import { useEffect, useState } from "react";
import { api } from "../api";
import { localToday, usePolling } from "../hooks";
import type { Task } from "../types";
import TaskRow from "./TaskRow";
import { Button, Card, Empty, PageHeader, SectionTitle } from "./ui";
import { toastError } from "./DialogHost";

type Tab = "open" | "today" | "overdue" | "done" | "all";

const TABS: { key: Tab; label: string }[] = [
  { key: "open", label: "未完成" },
  { key: "today", label: "今天" },
  { key: "overdue", label: "已逾期" },
  { key: "done", label: "已完成" },
  { key: "all", label: "全部" },
];

export default function TasksView({
  refreshKey,
  onChanged,
  navParam,
}: {
  refreshKey: number;
  onChanged: () => void;
  /** 跨视图导航参数：Today 统计卡跳转时指定 tab */
  navParam?: string;
}) {
  const [tab, setTab] = useState<Tab>("open");
  const [tasks, setTasks] = useState<Task[]>([]);
  const [title, setTitle] = useState("");
  // 默认今天到期：让「新建的任务」立即可见于 Today 页（清空日期则不排期）
  const [due, setDue] = useState(localToday());
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (navParam && TABS.some((t) => t.key === navParam)) setTab(navParam as Tab);
  }, [navParam]);

  const load = () => api.listTasks(tab).then(setTasks).catch(console.error);
  usePolling(load, 4000, [tab, refreshKey]);

  const add = async () => {
    const t = title.trim();
    if (!t || busy) return;
    setBusy(true);
    try {
      await api.createTask(t, due || undefined);
      setTitle("");
      setDue(localToday());
      onChanged();
    } catch (e) {
      toastError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div>
      <PageHeader title="任务" count={tasks.length} desc="创建、追踪、完成；⌘K 可全局搜索" />

      <div className="mt-5 flex items-center gap-2 rounded-xl border border-line bg-surface px-3 py-2">
        <input
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && add()}
          placeholder="新任务标题，回车创建…"
          className="flex-1 bg-transparent text-sm outline-none placeholder:text-ink3"
        />
        <input
          type="date"
          value={due}
          onChange={(e) => setDue(e.target.value)}
          className="rounded-md border border-line bg-transparent px-2 py-1 text-xs text-ink2 outline-none"
        />
        <Button onClick={add} disabled={!title.trim() || busy}>
          添加
        </Button>
      </div>
      <p className="mt-1.5 text-[11px] text-ink3">默认今天到期；清空日期则不排期（不出现在「今天」页）</p>

      <SectionTitle>
        <div className="flex gap-1">
          {TABS.map((t) => (
            <button
              key={t.key}
              onClick={() => setTab(t.key)}
              className={`rounded-lg px-2.5 py-1 text-xs transition-colors ${
                tab === t.key
                  ? "bg-accent/10 font-medium text-accent"
                  : "text-ink3 hover:text-ink"
              }`}
            >
              {t.label}
            </button>
          ))}
        </div>
      </SectionTitle>

      {tasks.length === 0 ? (
        <Empty text="这里空空如也" glyph="☑" />
      ) : (
        <Card>
          {tasks.map((t) => (
            <TaskRow key={t.id} task={t} onChanged={onChanged} />
          ))}
        </Card>
      )}
    </div>
  );
}
