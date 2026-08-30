import { api } from "../api";
import { fmtDateTime, isOverdue } from "../hooks";
import type { Task } from "../types";
import { Badge } from "./ui";

export default function TaskRow({
  task,
  onChanged,
}: {
  task: Task;
  onChanged: () => void;
}) {
  const overdue = isOverdue(task.due_at, task.status);
  const done = task.status === "done";

  const toggle = async () => {
    try {
      await api.setTaskStatus(task.id, done ? "todo" : "done");
      onChanged();
    } catch (e) {
      alert(String(e));
    }
  };

  const remove = async () => {
    if (!confirm(`删除任务「${task.title}」？`)) return;
    try {
      await api.deleteTask(task.id);
      onChanged();
    } catch (e) {
      alert(String(e));
    }
  };

  return (
    <div className="group flex items-center gap-3 border-b border-white/5 px-3 py-2.5 last:border-b-0 hover:bg-white/[0.03]">
      <button
        onClick={toggle}
        className={`flex h-[18px] w-[18px] shrink-0 items-center justify-center rounded-full border text-[10px] transition-colors ${
          done
            ? "border-emerald-400 bg-emerald-400 text-[#0f1115]"
            : "border-slate-500 hover:border-emerald-300"
        }`}
        title={done ? "重新打开" : "标记完成"}
      >
        {done ? "✓" : ""}
      </button>

      <span className="flex-1 truncate text-sm">
        <span className={done ? "text-slate-500 line-through" : ""}>{task.title}</span>
      </span>

      {overdue && <Badge tone="red">逾期</Badge>}
      {!done && task.status === "doing" && <Badge tone="blue">进行中</Badge>}
      {task.due_at && !overdue && !done && (
        <span className="shrink-0 text-xs text-slate-500">{fmtDateTime(task.due_at)}</span>
      )}
      {task.project_id && (
        <Badge tone="violet">{shortId(task.project_id)}</Badge>
      )}

      <button
        onClick={remove}
        className="invisible shrink-0 text-slate-600 transition-colors hover:text-rose-400 group-hover:visible"
        title="删除"
      >
        ×
      </button>
    </div>
  );
}

function shortId(id: string): string {
  return id.length > 16 ? id.slice(0, 16) + "…" : id;
}
