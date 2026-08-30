import { useEffect, useState } from "react";
import { api } from "../api";
import type { CardStyle, LibraryListItem, ProjectWithStats, Task } from "../types";
import { TASK_COLORS } from "../taskVisual";
import { Button } from "./ui";
import { confirmDialog, toastError } from "./DialogHost";

const STYLES: { key: CardStyle; label: string }[] = [
  { key: "day", label: "日卡" },
  { key: "week", label: "周卡" },
  { key: "month", label: "月卡" },
  { key: "year", label: "年卡" },
];

/** 任务编辑器：新建/编辑共用。标题/图标/主题色/每日目标/单位/项目/主库/卡片样式。 */
export default function TaskEditor({
  task,
  onClose,
  onSaved,
}: {
  /** null = 新建 */
  task: Task | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [title, setTitle] = useState(task?.title ?? "");
  const [icon, setIcon] = useState(task?.icon ?? "");
  const [color, setColor] = useState(task?.color_hex ?? TASK_COLORS[0].hex);
  const [target, setTarget] = useState<number>(1);
  const [unit, setUnit] = useState(task?.unit ?? "");
  const [cardStyle, setCardStyle] = useState<CardStyle>(task?.card_style ?? "day");
  const [projectId, setProjectId] = useState<string>(task?.project_id ?? "");
  const [libraryId, setLibraryId] = useState<string>("");
  const [projects, setProjects] = useState<ProjectWithStats[]>([]);
  const [libraries, setLibraries] = useState<LibraryListItem[]>([]);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api.listProjects().then((ps) => setProjects(ps.filter((p) => p.status === "active"))).catch(console.error);
    api.listLibraries(false).then(setLibraries).catch(console.error);
  }, []);

  // 编辑态：目标读取自今日视图（新建默认 1）
  useEffect(() => {
    if (!task) return;
    api
      .getToday()
      .then((t) => {
        const v = t.today_tasks.find((x) => x.task.id === task.id);
        if (v?.target) setTarget(v.target);
        setLibraryId(v?.library_id ?? "");
      })
      .catch(console.error);
  }, [task]);

  const save = async () => {
    const t = title.trim();
    if (!t || busy) return;
    setBusy(true);
    try {
      if (task) {
        await api.updateTask({
          id: task.id,
          title: t,
          icon,
          color,
          target,
          unit,
          cardStyle,
          projectId: projectId || null,
        });
        // 归属单独走 move（按天生效）
        await api.moveTaskLibrary(task.id, libraryId || null);
      } else {
        await api.createTask({
          title: t,
          icon,
          color,
          target,
          unit,
          cardStyle,
          projectId: projectId || null,
          libraryId: libraryId || null,
        });
      }
      onSaved();
      onClose();
    } catch (e) {
      toastError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const archive = async () => {
    if (!task) return;
    if (!(await confirmDialog("归档任务", `「${task.title}」将停止打卡，历史记录保留`))) return;
    try {
      await api.archiveTask(task.id);
      onSaved();
      onClose();
    } catch (e) {
      toastError(String(e));
    }
  };

  const fieldCls =
    "w-full rounded-lg border border-line bg-surface2 px-2.5 py-1.5 text-sm outline-none focus:border-accent/50";
  const labelCls = "mb-1 block text-xs text-ink2";

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/30"
      onClick={onClose}
    >
      <div
        className="w-[440px] max-w-[92vw] rounded-2xl border border-line bg-surface p-5 shadow-lg"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 className="text-sm font-semibold text-ink">{task ? "编辑任务" : "新建任务"}</h2>

        <div className="mt-4 grid grid-cols-[72px_1fr] gap-3">
          <div>
            <label className={labelCls}>图标</label>
            <input
              value={icon}
              onChange={(e) => setIcon(e.target.value)}
              placeholder="✓"
              maxLength={4}
              className={`${fieldCls} text-center text-lg`}
            />
          </div>
          <div>
            <label className={labelCls}>标题</label>
            <input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="例如：喝水、健身、阅读…"
              autoFocus
              className={fieldCls}
              onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && save()}
            />
          </div>
        </div>

        <div className="mt-3">
          <label className={labelCls}>主题色</label>
          <div className="flex gap-2">
            {TASK_COLORS.map((c) => (
              <button
                key={c.hex}
                title={c.name}
                onClick={() => setColor(c.hex)}
                className="h-7 w-7 rounded-lg transition-transform hover:scale-110"
                style={{
                  background: c.hex,
                  boxShadow:
                    color === c.hex ? `0 0 0 2px var(--surface), 0 0 0 4px ${c.hex}` : undefined,
                }}
              />
            ))}
          </div>
        </div>

        <div className="mt-3 grid grid-cols-3 gap-3">
          <div>
            <label className={labelCls}>每日目标</label>
            <input
              type="number"
              min={1}
              max={999}
              value={target}
              onChange={(e) => setTarget(Math.max(1, Math.min(999, Number(e.target.value) || 1)))}
              className={fieldCls}
            />
          </div>
          <div>
            <label className={labelCls}>单位</label>
            <input
              value={unit}
              onChange={(e) => setUnit(e.target.value)}
              placeholder="次/杯/页…"
              className={fieldCls}
            />
          </div>
          <div>
            <label className={labelCls}>卡片样式</label>
            <select
              value={cardStyle}
              onChange={(e) => setCardStyle(e.target.value as CardStyle)}
              className={fieldCls}
            >
              {STYLES.map((s) => (
                <option key={s.key} value={s.key}>
                  {s.label}
                </option>
              ))}
            </select>
          </div>
        </div>

        <div className="mt-3 grid grid-cols-2 gap-3">
          <div>
            <label className={labelCls}>所属项目</label>
            <select
              value={projectId}
              onChange={(e) => setProjectId(e.target.value)}
              className={fieldCls}
            >
              <option value="">（无）</option>
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className={labelCls}>日期主库</label>
            <select
              value={libraryId}
              onChange={(e) => setLibraryId(e.target.value)}
              className={fieldCls}
            >
              <option value="">独立任务</option>
              {libraries.map((l) => (
                <option key={l.id} value={l.id}>
                  {l.title}
                </option>
              ))}
            </select>
          </div>
        </div>

        {task && (
          <p className="mt-2 text-[11px] text-ink3">
            修改目标从明天起生效，历史日期仍按当时目标统计。
          </p>
        )}

        <div className="mt-5 flex items-center justify-between">
          <div>
            {task && task.status === "active" && (
              <Button variant="ghost" onClick={archive}>
                归档
              </Button>
            )}
            {task && task.status === "archived" && (
              <Button
                variant="ghost"
                onClick={async () => {
                  try {
                    await api.restoreTask(task.id);
                    onSaved();
                    onClose();
                  } catch (e) {
                    toastError(String(e));
                  }
                }}
              >
                恢复
              </Button>
            )}
          </div>
          <div className="flex gap-2">
            <Button variant="ghost" onClick={onClose}>
              取消
            </Button>
            <Button onClick={save} disabled={!title.trim() || busy}>
              {task ? "保存" : "创建"}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
