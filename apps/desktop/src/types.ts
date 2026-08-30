export type TaskStatus = "todo" | "doing" | "done";

export interface Task {
  id: string;
  title: string;
  status: TaskStatus;
  due_at?: string | null;
  project_id?: string | null;
  completed_at?: string | null;
  created_at: string;
  updated_at: string;
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
  today_total: number;
  overdue_total: number;
  completed_today: number;
}

export interface ProjectSummary {
  id: string;
  name: string;
}

export interface TodayContext {
  generated_at: string;
  date: string;
  stats: TodayStats;
  today_tasks: Task[];
  overdue_tasks: Task[];
  active_projects: ProjectSummary[];
  open_inbox_count: number;
  recent_notes: Note[];
}
