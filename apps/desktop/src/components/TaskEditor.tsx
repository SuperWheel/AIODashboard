import { useEffect, useState } from "react";
import { api } from "../api";
import type { CardStyle, LibraryListItem, ProjectWithStats, Recurrence, Task } from "../types";
import { defaultRecurrence } from "../types";
import { TASK_COLORS } from "../taskVisual";
import { Button, ColorSwatches, EmojiPicker, FieldSelect, inputCls } from "./ui";
import { confirmDialog, toastError } from "./DialogHost";

const STYLES: { key: CardStyle; label: string }[] = [
  { key: "day", label: "日卡" },
  { key: "week", label: "周卡" },
  { key: "month", label: "月卡" },
  { key: "year", label: "年卡" },
];

/** 任务编辑器：新建/编辑共用。标题/图标/主题色/每日目标/单位/项目/重要日/卡片样式。 */
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
  const [recurrence, setRecurrence] = useState<Recurrence>(task?.recurrence ?? defaultRecurrence);
  const [projectId, setProjectId] = useState<string>(task?.project_id ?? "");
  const [libraryId, setLibraryId] = useState<string>("");
  // 目标/归属只能从今日视图读出真实值。归档或今天不适用的任务不在
  // today_tasks 里，若允许用默认值(1/空)写回，会静默重置目标并移出重要日。
  const [dayLoaded, setDayLoaded] = useState(!task);
  const [projects, setProjects] = useState<ProjectWithStats[]>([]);
  const [libraries, setLibraries] = useState<LibraryListItem[]>([]);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api.listProjects().then((ps) => setProjects(ps.filter((p) => p.status === "active"))).catch(console.error);
    api.listLibraries(false).then(setLibraries).catch(console.error);
  }, []);

  const isoWeekdayToday = (() => {
    const d = new Date().getDay(); // 0=周日
    return d === 0 ? 7 : d;
  })();
  const weeklyDays =
    recurrence.kind === "weekly" && recurrence.weekdays.length > 0
      ? recurrence.weekdays
      : [isoWeekdayToday];
  const toggleWeekday = (w: number) => {
    setRecurrence((r) => {
      const cur = r.kind === "weekly" && r.weekdays.length > 0 ? r.weekdays : [isoWeekdayToday];
      const next = cur.includes(w) ? cur.filter((x) => x !== w) : [...cur, w];
      return { kind: "weekly", weekdays: next.length ? next : [w] }; // 至少保留一天
    });
  };

  // 编辑态：目标读取自今日视图（新建默认 1）
  useEffect(() => {
    if (!task) return;
    api
      .getToday()
      .then((t) => {
        const v = t.today_tasks.find((x) => x.task.id === task.id);
        setDayLoaded(!!v);
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
          ...(dayLoaded ? { target, recurrence } : {}),
          unit,
          cardStyle,
          projectId: projectId || null,
        });
        // 归属单独走 move（按天生效）；读不到今日视图时跳过，避免误移出
        if (dayLoaded) {
          await api.moveTaskLibrary(task.id, libraryId || null);
        }
      } else {
        await api.createTask({
          title: t,
          icon,
          color,
          target,
          unit,
          cardStyle,
          recurrence,
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

  const del = async () => {
    if (!task) return;
    const ok = await confirmDialog(
      "删除任务",
      `「${task.title}」及其全部打卡历史、目标区间将被永久删除，不可恢复。确定删除？`,
    );
    if (!ok) return;
    try {
      await api.deleteTask(task.id);
      onSaved();
      onClose();
    } catch (e) {
      toastError(String(e));
    }
  };

  const fieldCls = inputCls;
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

        <div className="mt-4 grid grid-cols-[132px_1fr] gap-3">
          <div>
            <label className={labelCls}>图标</label>
            <div className="flex gap-1.5">
              <input
                value={icon}
                onChange={(e) => setIcon(e.target.value)}
                placeholder="✓"
                maxLength={4}
                className={`${fieldCls} w-0 flex-1 px-1 text-center text-lg`}
              />
              <EmojiPicker onPick={setIcon} />
            </div>
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
          <ColorSwatches value={color} onChange={setColor} />
        </div>

        <div className="mt-3 grid grid-cols-3 gap-3">
          <div>
            <label className={labelCls}>每日目标</label>
            <input
              type="number"
              min={1}
              max={999}
              value={target}
              disabled={!dayLoaded}
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
            <FieldSelect value={cardStyle} onChange={(v) => setCardStyle(v as CardStyle)}>
              {STYLES.map((s) => (
                <option key={s.key} value={s.key}>
                  {s.label}
                </option>
              ))}
            </FieldSelect>
          </div>
        </div>

        <div className="mt-3">
          <label className={labelCls}>循环</label>
          <div className="flex items-center gap-2">
            <FieldSelect
              value={recurrence.kind}
              disabled={!dayLoaded}
              className="w-28 shrink-0"
              onChange={(kind) =>
                setRecurrence(
                  kind === "weekly"
                    ? { kind: "weekly", weekdays: [isoWeekdayToday] }
                    : ({ kind } as Recurrence),
                )
              }
            >
              <option value="daily">每天</option>
              <option value="weekly">每周</option>
              <option value="monthly">每月</option>
              <option value="yearly">每年</option>
              <option value="once">一次性</option>
            </FieldSelect>
            {recurrence.kind === "weekly" && (
              <div className="flex gap-1">
                {["一", "二", "三", "四", "五", "六", "日"].map((label, i) => {
                  const w = i + 1;
                  const on = weeklyDays.includes(w);
                  return (
                    <button
                      key={w}
                      type="button"
                      title={`每周${label}`}
                      onClick={() => toggleWeekday(w)}
                      className={`h-7 w-7 rounded-lg border text-xs transition-colors ${
                        on
                          ? "border-accent/40 bg-accent/10 font-medium text-accent"
                          : "border-line text-ink3 hover:bg-hover hover:text-ink"
                      }`}
                    >
                      {label}
                    </button>
                  );
                })}
              </div>
            )}
          </div>
        </div>

        <div className="mt-3 grid grid-cols-2 gap-3">
          <div>
            <label className={labelCls}>所属项目</label>
            <FieldSelect value={projectId} onChange={setProjectId}>
              <option value="">（无）</option>
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </FieldSelect>
          </div>
          <div>
            <label className={labelCls}>重要日</label>
            <FieldSelect value={libraryId} disabled={!dayLoaded} onChange={setLibraryId}>
              <option value="">独立任务</option>
              {libraries.map((l) => (
                <option key={l.id} value={l.id}>
                  {l.title}
                </option>
              ))}
            </FieldSelect>
          </div>
        </div>

        {task && !dayLoaded && (
          <p className="mt-2 text-[11px] text-ink3">
            该任务今天不在打卡列表（已归档或今天不适用），目标与归属暂不修改，恢复后可调整。
          </p>
        )}

        {task && (
          <p className="mt-2 text-[11px] text-ink3">
            修改目标/循环从明天起生效，历史日期仍按当时口径统计；一次性任务完成打卡后自动归档。
          </p>
        )}

        <div className="mt-5 flex items-center justify-between">
          <div className="flex gap-2">
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
            {task && (
              <Button variant="danger" onClick={del}>
                删除
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
