import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { api } from "../api";
import { localToday, useMinWidth, usePolling } from "../hooks";
import type { CardStyle, Task, TaskDayView } from "../types";
import TaskCard from "./TaskCard";
import TaskDetailView from "./TaskDetailView";
import TaskEditor from "./TaskEditor";
import { Button, DatePickerPanel, Empty, PageHeader } from "./ui";
import { toastError } from "./DialogHost";

type Tab = "active" | "archived";
/** 卡片墙视图：smart = 均衡发牌混排；grouped = 按卡片类型分区 */
type WallMode = "smart" | "grouped";

const WALL_MODE_KEY = "aiodashboard.tasks.wallMode";

const WEEKDAYS = ["日", "一", "二", "三", "四", "五", "六"] as const;

function shiftDay(day: string, n: number): string {
  const d = new Date(`${day}T00:00:00`);
  d.setDate(d.getDate() + n);
  const pad = (x: number) => String(x).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

function dayLabel(day: string): string {
  const d = new Date(`${day}T00:00:00`);
  return `${day} · 周${WEEKDAYS[d.getDay()]}`;
}

/** 日期导航：◀ 日期 ▶ 一体分段控件（圆角矩形，与快速捕捉切换器同规格）；
 *  日期按钮弹选择层跳任意日；非今天时高亮并显示「回到今天」。 */
function DayNavigator({ day, onChange }: { day: string; onChange: (d: string) => void }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [open]);
  const isToday = day === localToday();
  const segBtn =
    "flex h-6 items-center justify-center rounded-md text-xs text-ink2 transition-colors hover:bg-surface hover:text-ink";
  return (
    <div className="flex items-center gap-2" ref={ref}>
      <div className="flex items-center rounded-lg bg-hover p-0.5">
        <button className={`${segBtn} w-6`} onClick={() => onChange(shiftDay(day, -1))} title="前一天">
          ◀
        </button>
        <div className="relative">
          <button
            className={`${segBtn} px-2 tabular-nums ${
              isToday ? "" : "bg-surface font-medium text-accent shadow-sm"
            }`}
            onClick={() => setOpen((v) => !v)}
            title="选择日期"
          >
            {dayLabel(day)}
            {isToday ? " · 今天" : ""}
          </button>
          {open && (
            <div className="absolute left-1/2 top-8 z-30 -translate-x-1/2">
              <DatePickerPanel
                value={day}
                onSelect={(d) => {
                  onChange(d);
                  setOpen(false);
                }}
              />
            </div>
          )}
        </div>
        <button className={`${segBtn} w-6`} onClick={() => onChange(shiftDay(day, 1))} title="后一天">
          ▶
        </button>
      </div>
      {!isToday && (
        <button
          className="rounded-lg px-2 py-1 text-xs text-accent transition-colors hover:bg-accent/10"
          onClick={() => onChange(localToday())}
        >
          回到今天
        </button>
      )}
    </div>
  );
}

/** 各样式的兜底高度估算（px）：仅用于卡片首次渲染、实测值到达前的发牌；
 *  实测（ResizeObserver）才是均衡依据——副标题换行/月卡行数等都会让实际高度偏离估算。 */
const CARD_HEIGHT_EST: Record<CardStyle, number> = {
  day: 76,
  week: 118,
  month: 150,
  year: 670,
};

/** 实测高度容器：上报 offsetHeight，样式切换/文本换行/窗口缩放都会触发。
 *  卡片高度与所在列无关（两列等宽），重排不会引起高度变化，因此不会抖动。 */
function MeasuredCard({
  id,
  onHeight,
  children,
}: {
  id: string;
  onHeight: (id: string, h: number) => void;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const report = () => onHeight(id, el.offsetHeight);
    report(); // 首帧绘制前同步上报，避免估算布局闪现
    const ro = new ResizeObserver(report);
    ro.observe(el);
    return () => ro.disconnect();
  }, [id, onHeight]);
  return <div ref={ref}>{children}</div>;
}

/** 依次把卡片发进"当前最矮"的列（贪心均衡）：高卡先落一列后，
 *  后续短卡会连续补到另一列直到两边高度接近，再回到较矮的列。
 *  按序处理保证前缀稳定——新增任务不会让已有卡片换列。 */
function dealColumns<T>(items: T[], cols: number, heightFor: (t: T) => number): T[][] {
  const buckets = Array.from({ length: cols }, () => ({ h: 0, list: [] as T[] }));
  for (const it of items) {
    const b = buckets.reduce((a, c) => (c.h < a.h ? c : a));
    b.list.push(it);
    b.h += heightFor(it) + 16; // gap-4
  }
  return buckets.map((b) => b.list);
}

const STYLE_SECTIONS: { key: CardStyle; label: string }[] = [
  { key: "day", label: "日卡" },
  { key: "week", label: "周卡" },
  { key: "month", label: "月卡" },
  { key: "year", label: "年卡" },
];

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
  // 日期翻页：任务墙查看的逻辑日（默认今天；过去日只读回看）
  const [day, setDay] = useState(localToday());
  const isToday = day === localToday();
  const [wallMode, setWallMode] = useState<WallMode>(() => {
    try {
      return localStorage.getItem(WALL_MODE_KEY) === "grouped" ? "grouped" : "smart";
    } catch {
      return "smart";
    }
  });
  const twoCols = useMinWidth(1280); // 与 Tailwind xl: 断点一致

  const changeWallMode = (m: WallMode) => {
    setWallMode(m);
    try {
      localStorage.setItem(WALL_MODE_KEY, m);
    } catch {
      /* localStorage 不可用时仅本次会话生效 */
    }
  };

  // 墙序 = 创建时间升序，与打卡状态解耦：完成与否不改变卡片位置，
  // 新任务固定排在最末（发牌时落到当前较矮一列的底部）。
  // created_at 为 UTC RFC3339，字典序即时间序。
  const wallViews = useMemo(
    () => [...views].sort((a, b) => a.task.created_at.localeCompare(b.task.created_at)),
    [views],
  );

  // 各卡实测高度（task id → px）；相等值不更新，避免重排循环
  const [measured, setMeasured] = useState<Record<string, number>>({});
  const reportHeight = useCallback((id: string, h: number) => {
    setMeasured((m) => (m[id] === h ? m : { ...m, [id]: h }));
  }, []);
  const heightFor = useCallback(
    (v: TaskDayView) => measured[v.task.id] ?? CARD_HEIGHT_EST[v.task.card_style],
    [measured],
  );

  // 混排列分配：实测高度到达/任务集/列数变化时重算（发牌前缀稳定，不跳动）
  const columns = useMemo(
    () => dealColumns(wallViews, twoCols ? 2 : 1, heightFor),
    [wallViews, twoCols, heightFor],
  );

  // navParam 以 tsk_ 开头 → 任务详情
  const detailId = navParam?.startsWith("tsk_") ? navParam : null;

  const load = () => {
    // 任务墙 = 全部启用任务在所选日的视图（含当日不适用者，004）；
    // Today 页仍走 getToday 只看今日适用
    api
      .taskWallViews(day)
      .then(setViews)
      .catch(console.error);
    api
      .listTasks("archived")
      .then(setArchived)
      .catch(console.error);
  };
  usePolling(load, 4000, [refreshKey, day]);

  useEffect(() => {
    if (navParam === "archived") setTab("archived");
    if (navParam === "active") setTab("active");
  }, [navParam]);

  const openEditor = (task: Task | null) => {
    setEditing(task);
    setEditorOpen(true);
  };

  const renderCard = (v: TaskDayView) => (
    <MeasuredCard key={v.task.id} id={v.task.id} onHeight={reportHeight}>
      <TaskCard
        view={v}
        refreshKey={refreshKey}
        onChanged={onChanged}
        onOpenDetail={(id) => onNav("tasks", id)}
        onEdit={openEditor}
        anchorDay={day}
        interactive={isToday}
      />
    </MeasuredCard>
  );

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

      <div className="mt-4 flex flex-wrap items-center justify-between gap-x-3 gap-y-2">
        <div className="flex flex-wrap items-center gap-3">
          <div className="flex gap-1">
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
          {/* 日期翻页：仅进行中墙；过去/未来日只读回看 */}
          {tab === "active" && <DayNavigator day={day} onChange={setDay} />}
        </div>
        {tab === "active" && views.length > 0 && (
          <div className="flex gap-1">
            {(
              [
                { key: "smart", label: "均衡混排" },
                { key: "grouped", label: "类型分区" },
              ] as { key: WallMode; label: string }[]
            ).map((m) => (
              <button
                key={m.key}
                onClick={() => changeWallMode(m.key)}
                title={
                  m.key === "smart"
                    ? "按高度均衡分两列，卡片互不拉伸"
                    : "按日/周/月/年分区展示"
                }
                className={`rounded-lg px-2.5 py-1 text-xs transition-colors ${
                  wallMode === m.key
                    ? "bg-accent/10 font-medium text-accent"
                    : "text-ink3 hover:text-ink"
                }`}
              >
                {m.label}
              </button>
            ))}
          </div>
        )}
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
        ) : wallMode === "smart" ? (
          <div className="mt-4 grid grid-cols-1 items-start gap-4 xl:grid-cols-2">
            {columns.map((col, i) => (
              <div key={i} className="flex flex-col gap-4">
                {col.map((v) => renderCard(v))}
              </div>
            ))}
          </div>
        ) : (
          STYLE_SECTIONS.map((s) => {
            const items = wallViews.filter((v) => v.task.card_style === s.key);
            if (items.length === 0) return null;
            return (
              <div key={s.key} className="mt-5 first:mt-4">
                <div className="mb-2 text-[11px] font-medium uppercase tracking-wider text-ink3">
                  {s.label} · {items.length}
                </div>
                <div className="grid grid-cols-1 items-start gap-4 xl:grid-cols-2">
                  {items.map((v) => renderCard(v))}
                </div>
              </div>
            );
          })
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
