import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { api } from "../api";
import { localToday } from "../hooks";
import type { GlobalYearHeatmap, LibraryListItem, TaskDayView, TodayContext } from "../types";
import { taskColor } from "../taskVisual";
import { RateHeatmapGrid } from "./Heatmap";
import QuickCapture from "./QuickCapture";
import TodayTaskCard from "./TodayTaskCard";
import { Badge, Card, Empty, ProgressRing, StatCard } from "./ui";
import { toastError } from "./DialogHost";

/** 打卡操作后的位置冻结窗口：窗口内不重排（卡片留在原位，数据照常刷新），
 * 之后的轮询刷新再按最新状态归位——避免完成瞬间卡片跳来跳去。 */
const GROUP_SETTLE_MS = 3000;

/** 重要日天数短文案（首页右栏卡）。 */
function libraryDayText(it: LibraryListItem): { text: string; overdue: boolean } {
  switch (it.day_info.display_kind) {
    case "day_n":
      return { text: `第 ${it.day_info.day_count} 天`, overdue: false };
    case "remaining":
      return { text: `剩 ${it.day_info.day_count} 天`, overdue: false };
    case "today":
      return { text: "就是今天", overdue: false };
    default:
      return { text: `已过 ${it.day_info.day_count} 天`, overdue: true };
  }
}

/**
 * Today = Bento 总控台：今日任务为双列渐进填充卡片网格。
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

  // 年度热力 + 重要日：打卡信号（今日各任务次数之和）或日期变化时刷新，
  // 不跟 4s 轮询空转
  const [heatmap, setHeatmap] = useState<GlobalYearHeatmap | null>(null);
  const [libraries, setLibraries] = useState<LibraryListItem[]>([]);
  const checkinSignal = useMemo(
    () => todayTasks.reduce((s, v) => s + v.count, 0),
    [todayTasks],
  );
  useEffect(() => {
    api.globalYearHeatmap().then(setHeatmap).catch(console.error);
    api
      .listLibraries(false)
      .then((items) => setLibraries(items.filter((i) => i.status === "active")))
      .catch(console.error);
  }, [checkinSignal, data?.date]);

  // ⌘Z：撤销本次会话最近一次打卡的任务
  const [lastActionTask, setLastActionTask] = useState<string | null>(null);
  const lastActionAt = useRef(0);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.key.toLowerCase() !== "z" || !lastActionTask) return;
      // 输入框里的 ⌘Z 是文本编辑撤销，不劫持
      const el = e.target as HTMLElement | null;
      if (
        el &&
        (["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName) || el.isContentEditable)
      ) {
        return;
      }
      e.preventDefault();
      const taskId = lastActionTask;
      // 同步清掉：请求返回前连按 ⌘Z 不会撤销两次
      setLastActionTask(null);
      lastActionAt.current = Date.now();
      api
        .taskUndo(taskId)
        .then(onChanged)
        .catch((e) => toastError(String(e)));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [lastActionTask, onChanged]);

  // 位置快照：数据每次刷新（打卡回写 + 4s 轮询）都会到达；冻结窗口内
  // 沿用旧顺序、只吸收新任务（追加末尾），窗口过后重建（未完成在前、已完成沉底）。
  const [pin, setPin] = useState<string[] | null>(null);
  useEffect(() => {
    if (!data) return;
    const tasks = data.today_tasks;
    setPin((prev) => {
      if (!prev || Date.now() - lastActionAt.current >= GROUP_SETTLE_MS) {
        const idx = new Map(tasks.map((v, i) => [v.task.id, i] as const));
        return [...tasks]
          .sort((a, b) => {
            const da = a.state === "completed" ? 1 : 0;
            const db = b.state === "completed" ? 1 : 0;
            return da - db || (idx.get(a.task.id) ?? 0) - (idx.get(b.task.id) ?? 0);
          })
          .map((v) => v.task.id);
      }
      const seen = new Set<string>();
      const out: string[] = [];
      for (const id of prev) {
        if (tasks.some((v) => v.task.id === id)) {
          out.push(id);
          seen.add(id);
        }
      }
      for (const v of tasks) {
        if (!seen.has(v.task.id)) out.push(v.task.id);
      }
      return out;
    });
  }, [data]);

  // 展示列表 = 快照顺序 + 最新数据
  const displayTasks = useMemo<TaskDayView[]>(() => {
    if (!pin) return todayTasks;
    const byId = new Map(todayTasks.map((v) => [v.task.id, v]));
    const out: TaskDayView[] = [];
    for (const id of pin) {
      const v = byId.get(id);
      if (v) {
        out.push(v);
        byId.delete(id);
      }
    }
    return [...out, ...byId.values()];
  }, [todayTasks, pin]);

  const afterAction = (taskId: string) => {
    lastActionAt.current = Date.now();
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

      {/* 年度热力图（全局所有任务聚合，GitHub 式） */}
      <Card className="lg:col-span-12">
        <div className="flex items-center justify-between border-b border-line px-4 py-3">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-ink2">
            年度热力{heatmap ? ` · ${heatmap.year}` : ""}
          </h2>
          <div className="flex items-center gap-1 text-[10px] text-ink3">
            <span className="mr-0.5">少</span>
            {[4.5, 18, 40, 65, 94].map((p, i) => (
              <span
                key={i}
                className="inline-block h-[10px] w-[10px] rounded-[3px]"
                style={{
                  background:
                    i === 0
                      ? "color-mix(in srgb, var(--ink) 4.5%, transparent)"
                      : `color-mix(in srgb, var(--accent) ${p}%, transparent)`,
                }}
              />
            ))}
            <span className="ml-0.5">多</span>
          </div>
        </div>
        <div className="p-4">
          {heatmap ? (
            <RateHeatmapGrid
              days={heatmap.days}
              leadingEmpty={heatmap.leading_empty_count}
              color="var(--accent)"
              onDayClick={() => onNav("tasks")}
            />
          ) : (
            <div className="text-sm text-ink3">加载中…</div>
          )}
        </div>
      </Card>

      {/* 今日任务主卡（双列渐进填充卡片网格） */}
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
          <div className="grid grid-cols-1 gap-2 p-3 sm:grid-cols-2">
            {displayTasks.map((v) => (
              <TodayTaskCard
                key={v.task.id}
                view={v}
                onChanged={() => afterAction(v.task.id)}
                onOpenDetail={(id) => onNav("tasks", id)}
              />
            ))}
          </div>
        )}
      </Card>

      {/* 右栏：重要日 + 最近笔记 + 活跃项目 */}
      <div className="flex flex-col gap-4 lg:col-span-4">
        <Card hoverable>
          <div className="flex items-center justify-between border-b border-line px-4 py-3">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-ink2">重要日</h2>
            <button
              className="text-xs text-ink3 hover:text-ink"
              onClick={() => onNav("libraries")}
            >
              全部 →
            </button>
          </div>
          <div className="p-3">
            {libraries.length > 0 ? (
              <ul className="space-y-0.5">
                {libraries.slice(0, 4).map((it) => {
                  const accent = taskColor(it.color_hex);
                  const day = libraryDayText(it);
                  return (
                    <li key={it.id}>
                      <button
                        onClick={() => onNav("libraries", it.id)}
                        className="flex w-full items-center gap-2.5 rounded-lg px-1.5 py-1.5 text-left transition-colors hover:bg-hover"
                        title={`${it.title} · 锚点 ${it.anchor_day}`}
                      >
                        <span
                          className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg text-sm"
                          style={{ background: `color-mix(in srgb, ${accent} 16%, transparent)` }}
                        >
                          {it.icon || "📅"}
                        </span>
                        <span className="min-w-0 flex-1 truncate text-xs text-ink2">
                          {it.title}
                        </span>
                        <span
                          className={`shrink-0 text-xs font-medium tabular-nums ${
                            day.overdue ? "text-danger" : ""
                          }`}
                          style={day.overdue ? undefined : { color: accent }}
                        >
                          {day.text}
                        </span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            ) : (
              <button
                className="text-xs text-ink3 hover:text-accent"
                onClick={() => onNav("libraries")}
              >
                还没有重要日，建一个倒数日 →
              </button>
            )}
          </div>
        </Card>

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
