export type TaskStatus = "active" | "archived";

export type CardStyle = "day" | "week" | "month" | "year";

/** 循环规则（004）：与卡片样式正交——样式管统计展示，循环管哪天适用。
 *  weekly.weekdays 为 ISO 星期（1=周一…7=周日）；once = 完成即自动归档。 */
export type Recurrence =
  | { kind: "daily" }
  | { kind: "weekly"; weekdays: number[] }
  | { kind: "monthly" }
  | { kind: "yearly" }
  | { kind: "once" };

export const defaultRecurrence: Recurrence = { kind: "daily" };

export interface Task {
  id: string;
  title: string;
  status: TaskStatus;
  icon: string;
  color_hex: string;
  unit: string;
  card_style: CardStyle;
  /** 当前生效循环规则（后端按当日目标区间回填；旧数据缺省视为 daily） */
  recurrence?: Recurrence;
  project_id?: string | null;
  created_at: string;
  updated_at: string;
}

/** 归档任务（归档页用）：任务本体 + 归档日（活动区间最后关闭日）。 */
export interface ArchivedTask extends Task {
  archived_day: string | null;
}

export type TaskDayState =
  | "not_applicable"
  | "pending"
  | "in_progress"
  | "completed"
  | "missed";

export type HeatmapState =
  | "not_applicable"
  | "future"
  | "zero"
  | "partial_low"
  | "partial_high"
  | "complete";

export interface TaskDayView {
  task: Task;
  target: number | null;
  count: number;
  state: TaskDayState | string;
  library_id: string | null;
  can_undo: boolean;
}

export interface HeatmapDay {
  logical_day: string;
  display_state: HeatmapState;
  day_state: TaskDayState | null;
  actual_count: number;
  target_count: number | null;
  capped_rate: number | null;
  is_overachieved: boolean;
  is_today: boolean;
  weekday_index: number;
  week_index: number;
  month: number;
}

export interface PeriodSummary {
  actual_count: number;
  applicable_day_count: number;
  complete_day_count: number;
  complete_day_rate: number;
  current_streak: number;
  longest_streak: number;
}

export interface PeriodBucket {
  label: string;
  start_day: string;
  end_day: string;
  actual_count: number;
  applicable_day_count: number;
  complete_day_count: number;
  complete_day_rate: number;
}

export interface PeriodOverview {
  kind: "week" | "month" | "year" | string;
  anchor_day: string;
  start_day: string;
  end_day: string;
  leading_empty_count: number;
  days: HeatmapDay[];
  summary: PeriodSummary;
  buckets: PeriodBucket[];
}

export type LibraryKind = "anniversary" | "countdown";
export type LibraryStatus = "active" | "archived";

export interface DateLibrary {
  id: string;
  title: string;
  note: string;
  icon: string;
  color_hex: string;
  kind: LibraryKind;
  anchor_day: string;
  sort_order: number;
  status: LibraryStatus;
  created_at: string;
  updated_at: string;
}

export interface LibraryDayInfo {
  day_count: number;
  /** anniversary: "day_n"；countdown: "remaining" | "today" | "overdue" */
  display_kind: "day_n" | "remaining" | "today" | "overdue" | string;
}

export interface LibraryListItem extends DateLibrary {
  day_info: LibraryDayInfo;
  task_count: number;
}

/** 聚合热力图单日（重要日综合 / 全局共用形状）。 */
export interface AggregateHeatmapDay {
  logical_day: string;
  display_state: "not_applicable" | "future" | "rate" | string;
  rate: number | null;
  active_task_count: number;
  is_today: boolean;
  weekday_index: number;
  week_index: number;
  month: number;
}

export interface LibraryYearHeatmap {
  library_id: string;
  year: number;
  start_day: string;
  end_day: string;
  leading_empty_count: number;
  days: AggregateHeatmapDay[];
}

/** 全局年度综合热力图（所有任务聚合，首页用）。 */
export interface GlobalYearHeatmap {
  year: number;
  start_day: string;
  end_day: string;
  leading_empty_count: number;
  days: AggregateHeatmapDay[];
}

export type ProjectStatus = "active" | "archived";

export interface Project {
  id: string;
  name: string;
  description: string;
  status: ProjectStatus;
  created_at: string;
  updated_at: string;
}

export interface ProjectWithStats extends Project {
  open_tasks: number;
}

export interface Note {
  id: string;
  title: string;
  body: string;
  project_id?: string | null;
  created_at: string;
  updated_at: string;
}

export type InboxStatus = "open" | "processed";

export interface InboxItem {
  id: string;
  content: string;
  source: string;
  status: InboxStatus;
  created_at: string;
}

export type SearchKind = "task" | "project" | "note" | "inbox";

export interface SearchHit {
  kind: SearchKind;
  id: string;
  title: string;
  subtitle?: string | null;
}

export interface SearchResults {
  query: string;
  total: number;
  hits: SearchHit[];
}

export interface TodayStats {
  task_total: number;
  completed_today: number;
  completion_rate: number;
  missed_last_7d: number;
}

export interface ProjectSummary {
  id: string;
  name: string;
}

export interface TodayContext {
  generated_at: string;
  date: string;
  stats: TodayStats;
  today_tasks: TaskDayView[];
  active_projects: ProjectSummary[];
  open_inbox_count: number;
  recent_notes: Note[];
}
