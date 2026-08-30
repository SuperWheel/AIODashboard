import { useState } from "react";
import { api } from "../api";
import { usePolling } from "../hooks";
import type { Task } from "../types";
import TaskRow from "./TaskRow";
import { Card, Empty, SectionTitle } from "./ui";

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
}: {
  refreshKey: number;
  onChanged: () => void;
}) {
  const [tab, setTab] = useState<Tab>("open");
  const [tasks, setTasks] = useState<Task[]>([]);
  const [title, setTitle] = useState("");
  const [due, setDue] = useState("");
  const [busy, setBusy] = useState(false);

  const load = () => api.listTasks(tab).then(setTasks).catch(console.error);
  usePolling(load, 4000, [tab, refreshKey]);

  const add = async () => {
    const t = title.trim();
    if (!t || busy) return;
    setBusy(true);
    try {
      await api.createTask(t, due || undefined);
      setTitle("");
      setDue("");
      onChanged();
    } catch (e) {
      alert(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div>
      <h1 className="text-xl font-semibold text-ink">任务</h1>

      <div className="mt-5 flex items-center gap-2 rounded-xl border border-line bg-surface px-3 py-2">
        <input
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && add()}
          placeholder="新任务标题，回车创建…"
          className="flex-1 bg-transparent text-sm outline-none placeholder:text-ink3"
        />
        <input
          type="date"
          value={due}
          onChange={(e) => setDue(e.target.value)}
          className="rounded-md border border-line bg-transparent px-2 py-1 text-xs text-ink2 outline-none"
        />
        <button
          onClick={add}
          disabled={!title.trim() || busy}
          className="rounded-lg bg-accent px-3 py-1 text-xs font-medium text-onaccent hover:bg-accent/90 disabled:opacity-40"
        >
          添加
        </button>
      </div>

      <SectionTitle
        right={<span className="text-xs text-ink3">{tasks.length} 个</span>}
      >
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
        <Empty text="这里空空如也" />
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
