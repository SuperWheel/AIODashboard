import { invoke } from "@tauri-apps/api/core";
import type {
  CardStyle,
  DateLibrary,
  GlobalYearHeatmap,
  Recurrence,
  InboxItem,
  LibraryListItem,
  LibraryYearHeatmap,
  Note,
  PeriodOverview,
  Project,
  ProjectWithStats,
  SearchResults,
  Task,
  TaskDayView,
  TodayContext,
} from "./types";
import type { PluginInfo, PluginManifest } from "./plugins/types";
import { pluginEvents } from "./plugins/events";

export interface ProcessReport {
  inbox_id: string;
  created_type: "task" | "note";
  created_id: string;
}

export interface PluginKvEntry {
  key: string;
  value: string;
}

export interface PluginFetchResult {
  status: number;
  text: string;
  json: unknown;
}

export interface CreateTaskParams {
  title: string;
  target?: number;
  unit?: string;
  icon?: string;
  color?: string;
  cardStyle?: CardStyle;
  recurrence?: Recurrence;
  projectId?: string | null;
  libraryId?: string | null;
  actor?: string;
}

export interface UpdateTaskParams {
  id: string;
  title?: string;
  target?: number;
  unit?: string;
  icon?: string;
  color?: string;
  cardStyle?: CardStyle;
  recurrence?: Recurrence;
  /** 显式 null = 移出项目；不传 = 不修改 */
  projectId?: string | null;
  actor?: string;
}

/** 循环规则的扁平序列化：后端 params 用 kind + weekdays 两个键。 */
function recurrencePayload(rec?: Recurrence): Record<string, unknown> {
  if (!rec) return { recurrence: null, weekdays: null };
  if (rec.kind === "weekly") {
    return { recurrence: "weekly", weekdays: rec.weekdays };
  }
  return { recurrence: rec.kind, weekdays: null };
}

export const api = {
  // Today / Tasks
  getToday: () => invoke<TodayContext>("get_today"),
  /** 任务墙：全部启用任务在指定逻辑日的视图（含当日不适用）；day 缺省 = 今天 */
  taskWallViews: (day?: string) => invoke<TaskDayView[]>("task_wall_views", { day: day ?? null }),
  listTasks: (scope?: "all" | "active" | "archived") =>
    invoke<Task[]>("list_tasks", { scope }),
  createTask: (params: CreateTaskParams) =>
    invoke<Task>("create_task", {
      params: {
        title: params.title,
        target: params.target ?? null,
        unit: params.unit ?? null,
        icon: params.icon ?? null,
        color: params.color ?? null,
        cardStyle: params.cardStyle ?? null,
        ...recurrencePayload(params.recurrence),
        projectId: params.projectId ?? null,
        libraryId: params.libraryId ?? null,
        actor: params.actor ?? null,
      },
    }).then((t) => {
      pluginEvents.emit("task.created", { id: t.id, title: t.title });
      return t;
    }),
  updateTask: (params: UpdateTaskParams) =>
    invoke<Task>("update_task", {
      params: {
        id: params.id,
        title: params.title ?? null,
        target: params.target ?? null,
        unit: params.unit ?? null,
        icon: params.icon ?? null,
        color: params.color ?? null,
        cardStyle: params.cardStyle ?? null,
        ...recurrencePayload(params.recurrence),
        projectId: params.projectId ?? null,
        // JSON null 无法区分"不变"与"清空"，后端靠这个标志判断移出项目
        clearProject: params.projectId === null,
        actor: params.actor ?? null,
      },
    }),
  archiveTask: (id: string, actor?: string) =>
    invoke<Task>("archive_task", { id, actor: actor ?? null }),
  restoreTask: (id: string, actor?: string) =>
    invoke<Task>("restore_task", { id, actor: actor ?? null }),
  deleteTask: (id: string) => invoke<void>("delete_task", { id }),

  // Check-in
  taskCheckin: (id: string, actor?: string) =>
    invoke<TaskDayView>("task_checkin", { id, operationId: null, actor: actor ?? null }).then(
      (v) => {
        if (v.state === "completed") {
          pluginEvents.emit("task.completed", { id: v.task.id, title: v.task.title });
        }
        return v;
      },
    ),
  taskDecrement: (id: string, actor?: string) =>
    invoke<TaskDayView>("task_decrement", { id, operationId: null, actor: actor ?? null }),
  taskUndo: (id: string, actor?: string) =>
    invoke<TaskDayView>("task_undo", { id, operationId: null, actor: actor ?? null }),
  taskOverview: (id: string, period: "week" | "month" | "year", anchor?: string) =>
    invoke<PeriodOverview>("task_overview", { id, period, anchor: anchor ?? null }),

  // Date Libraries
  listLibraries: (includeArchived = false) =>
    invoke<LibraryListItem[]>("list_libraries", { includeArchived }),
  createLibrary: (params: {
    title: string;
    kind: "anniversary" | "countdown";
    anchorDay: string;
    note?: string;
    icon?: string;
    color?: string;
  }) =>
    invoke<DateLibrary>("create_library", {
      title: params.title,
      kind: params.kind,
      anchorDay: params.anchorDay,
      note: params.note ?? null,
      icon: params.icon ?? null,
      color: params.color ?? null,
    }),
  updateLibrary: (id: string, params: { title?: string; note?: string; icon?: string; color?: string; anchorDay?: string }) =>
    invoke<DateLibrary>("update_library", {
      id,
      title: params.title ?? null,
      note: params.note ?? null,
      icon: params.icon ?? null,
      color: params.color ?? null,
      anchorDay: params.anchorDay ?? null,
    }),
  archiveLibrary: (id: string, mode: "keep" | "detach" | "move_to", moveTo?: string) =>
    invoke<DateLibrary>("archive_library", { id, mode, moveTo: moveTo ?? null }),
  restoreLibrary: (id: string) => invoke<DateLibrary>("restore_library", { id }),
  libraryTasks: (libraryId: string) => invoke<Task[]>("library_tasks", { libraryId }),
  libraryHeatmap: (libraryId: string, anchor?: string) =>
    invoke<LibraryYearHeatmap>("library_heatmap", { libraryId, anchor: anchor ?? null }),
  globalYearHeatmap: (anchor?: string) =>
    invoke<GlobalYearHeatmap>("global_year_heatmap", { anchor: anchor ?? null }),
  moveTaskLibrary: (id: string, libraryId: string | null) =>
    invoke<Task>("move_task_library", { id, libraryId }),

  // 带 actor 的写入（插件桥使用；审计 actor=plugin:<id>）
  createTaskAs: (actor: string, title: string, target?: number) =>
    api.createTask({ title, target, actor }),
  checkinAs: (actor: string, id: string) => api.taskCheckin(id, actor),
  deleteTaskAs: (actor: string, id: string) =>
    invoke<void>("delete_task", { id, actor }),
  createNoteAs: (actor: string, title: string, body: string) =>
    api.createNote(title, body, actor),
  addInboxItemAs: (actor: string, content: string) => api.addInboxItem(content, actor),

  // Projects
  listProjects: () => invoke<ProjectWithStats[]>("list_projects"),
  createProject: (name: string, description: string) =>
    invoke<Project>("create_project", { name, description: description ?? "" }),
  archiveProject: (id: string) => invoke<void>("archive_project", { id }),
  updateProject: (id: string, name?: string, description?: string) =>
    invoke<Project>("update_project", {
      id,
      name: name ?? null,
      description: description ?? null,
    }),
  deleteProject: (id: string) => invoke<void>("delete_project", { id }),

  // Notes
  listNotes: (limit = 200) => invoke<Note[]>("list_notes", { limit }),
  createNote: (title: string, body: string, actor?: string) =>
    invoke<Note>("create_note", { title, body, actor: actor ?? null }).then((n) => {
      pluginEvents.emit("note.created", { id: n.id, title: n.title });
      return n;
    }),
  updateNote: (id: string, title: string, body: string) =>
    invoke<Note>("update_note", { id, title, body }),
  deleteNote: (id: string) => invoke<void>("delete_note", { id }),

  // Inbox
  listInbox: (includeProcessed = false) =>
    invoke<InboxItem[]>("list_inbox", { includeProcessed }),
  addInboxItem: (content: string, actor?: string) =>
    invoke<InboxItem>("add_inbox_item", { content, actor: actor ?? null }).then((item) => {
      pluginEvents.emit("inbox.added", { id: item.id, content: item.content });
      return item;
    }),
  inboxToTask: (id: string) => invoke<ProcessReport>("inbox_to_task", { id }),
  inboxToNote: (id: string) => invoke<ProcessReport>("inbox_to_note", { id }),
  deleteInboxItem: (id: string) => invoke<void>("delete_inbox_item", { id }),

  // Search
  searchAll: (query: string) => invoke<SearchResults>("search_all", { query }),

  // Plugins
  pluginList: () => invoke<PluginInfo[]>("plugin_list"),
  pluginReadManifest: (id: string) => invoke<PluginManifest>("plugin_read_manifest", { id }),
  pluginSetEnabled: (id: string, enabled: boolean) =>
    invoke<void>("plugin_set_enabled", { id, enabled }),
  pluginLoadSource: (id: string) => invoke<string>("plugin_load_source", { id }),
  pluginKvGet: (pluginId: string, key: string) =>
    invoke<string | null>("plugin_kv_get", { pluginId, key }),
  pluginKvSet: (pluginId: string, key: string, value: string) =>
    invoke<void>("plugin_kv_set", { pluginId, key, value }),
  pluginKvDelete: (pluginId: string, key: string) =>
    invoke<boolean>("plugin_kv_delete", { pluginId, key }),
  pluginKvList: (pluginId: string, keyPrefix?: string) =>
    invoke<PluginKvEntry[]>("plugin_kv_list", { pluginId, keyPrefix: keyPrefix ?? null }),
  pluginHttpFetch: (pluginId: string, url: string) =>
    invoke<PluginFetchResult>("plugin_http_fetch", { pluginId, url }),
};
