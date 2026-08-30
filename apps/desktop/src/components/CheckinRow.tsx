import type { TaskDayView } from "../types";
import { taskColor } from "../taskVisual";
import { CheckinControls } from "./TaskCard";

/**
 * Today 页打卡行：圆环进度 + 圆角矩形 +/-（同侧），按状态分组使用。
 * 行点击进任务详情。
 */
export default function CheckinRow({
  view,
  onChanged,
  onOpenDetail,
}: {
  view: TaskDayView;
  onChanged: () => void;
  onOpenDetail: (taskId: string) => void;
}) {
  const accent = taskColor(view.task.color_hex);
  return (
    <div className="flex items-center gap-3 border-b border-line/60 px-4 py-2.5 last:border-b-0 hover:bg-hover">
      <div
        className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-sm"
        style={{ background: `color-mix(in srgb, ${accent} 16%, transparent)` }}
      >
        {view.task.icon || "✓"}
      </div>
      <button
        className="min-w-0 flex-1 text-left"
        onClick={() => onOpenDetail(view.task.id)}
        title="查看详情（Enter）"
      >
        <div className="truncate text-sm text-ink">{view.task.title}</div>
        <div className="text-[11px] text-ink3">
          {view.target
            ? `${view.count} / ${view.target} ${view.task.unit}`
            : "今日不适用"}
        </div>
      </button>
      <CheckinControls view={view} onChanged={onChanged} size="sm" />
    </div>
  );
}
