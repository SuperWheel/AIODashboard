import { useEffect, useState } from "react";
import { api } from "../api";
import { usePolling } from "../hooks";
import type { Task, TaskDayView } from "../types";
import TaskCard from "./TaskCard";
import TaskDetailView from "./TaskDetailView";
import TaskEditor from "./TaskEditor";
import { Button, Empty, PageHeader } from "./ui";
import { toastError } from "./DialogHost";

type Tab = "active" | "archived";

/**
 * 任务页 = 卡片墙：每个任务一张卡（日/周/月/年样式可切换）。
 * navParam = 任务 id 时进入任务详情。
 */
export default function TasksView({
  refreshKey,
  onChanged,
  navParam,
  onNav,
}: {
  refreshKey: number;
  onChanged: () => void;
  navParam?: string;
  onNav: (view: string, param?: string) => void;
}) {
  const [tab, setTab] = useState<Tab>("active");
  const [views, setViews] = useState<TaskDayView[]>([]);
  const [archived, setArchived] = useState<Task[]>([]);
  const [editorOpen, setEditorOpen] = useState(false);
  const [editing, setEditing] = useState<Task | null>(null);

  // navParam 以 tsk_ 开头 → 任务详情
  const detailId = navParam?.startsWith("tsk_") ? navParam : null;

  const load = () => {
    api
      .getToday()
      .then((t) => setViews(t.today_tasks))
      .catch(console.error);
    api
      .listTasks("archived")
      .then(setArchived)
      .catch(console.error);
  };
  usePolling(load, 4000, [refreshKey]);

  useEffect(() => {
    if (navParam === "archived") setTab("archived");
    if (navParam === "active") setTab("active");
  }, [navParam]);

  const openEditor = (task: Task | null) => {
    setEditing(task);
    setEditorOpen(true);
  };

  if (detailId) {
    const task = views.find((v) => v.task.id === detailId)?.task ?? archived.find((t) => t.id === detailId);
    return (
      <TaskDetailView
        taskId={detailId}
        refreshKey={refreshKey}
        onChanged={onChanged}
        onBack={() => onNav("tasks")}
        onEdit={() => task && openEditor(task)}
      />
    );
  }

  return (
    <div>
      <PageHeader
        title="任务"
        count={views.length}
        desc="长期打卡对象：每个自然日都可以重新打卡"
        actions={<Button onClick={() => openEditor(null)}>＋ 新建任务</Button>}
      />

      <div className="mt-4 flex gap-1">
        {(
          [
            { key: "active", label: "进行中" },
            { key: "archived", label: `已归档（${archived.length}）` },
          ] as { key: Tab; label: string }[]
        ).map((t) => (
          <button
            key={t.key}
            onClick={() => setTab(t.key)}
            className={`rounded-lg px-2.5 py-1 text-xs transition-colors ${
              tab === t.key ? "bg-accent/10 font-medium text-accent" : "text-ink3 hover:text-ink"
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === "active" &&
        (views.length === 0 ? (
          <div className="mt-4">
            <Empty
              text="还没有进行中的任务，新建一个开始打卡"
              glyph="☑"
              action={<Button onClick={() => openEditor(null)}>新建任务</Button>}
            />
          </div>
        ) : (
          <div className="mt-4 grid grid-cols-1 gap-4 xl:grid-cols-2">
            {views.map((v) => (
              <TaskCard
                key={v.task.id}
                view={v}
                refreshKey={refreshKey}
                onChanged={onChanged}
                onOpenDetail={(id) => onNav("tasks", id)}
                onEdit={openEditor}
              />
            ))}
          </div>
        ))}

      {tab === "archived" &&
        (archived.length === 0 ? (
          <div className="mt-4">
            <Empty text="没有已归档的任务" glyph="🗃" />
          </div>
        ) : (
          <div className="mt-4 rounded-2xl border border-line bg-surface">
            {archived.map((t) => (
              <div
                key={t.id}
                className="flex items-center gap-3 border-b border-line/60 px-4 py-2.5 last:border-b-0"
              >
                <span className="text-sm text-ink3">{t.icon || "✓"}</span>
                <span className="flex-1 truncate text-sm text-ink2">{t.title}</span>
                <Button variant="ghost" onClick={() => openEditor(t)}>
                  查看
                </Button>
                <Button
                  variant="ghost"
                  onClick={async () => {
                    try {
                      await api.restoreTask(t.id);
                      onChanged();
                    } catch (e) {
                      toastError(String(e));
                    }
                  }}
                >
                  恢复
                </Button>
              </div>
            ))}
          </div>
        ))}

      {editorOpen && (
        <TaskEditor
          task={editing}
          onClose={() => setEditorOpen(false)}
          onSaved={onChanged}
        />
      )}
    </div>
  );
}
