export type TaskStatus = "active" | "archived";

export type CardStyle = "day" | "week" | "month" | "year";

export interface Task {
  id: string;
  title: string;
  status: TaskStatus;
  icon: string;
  color_hex: string;
  unit: string;
  card_style: CardStyle;
  project_id?: string | null;
  created_at: string;
  updated_at: string;
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

export interface LibraryHeatmapDay {
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
  days: LibraryHeatmapDay[];
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
