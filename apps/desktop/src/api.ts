import { invoke } from "@tauri-apps/api/core";
import type {
  ArchivedTask,
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
import type {
  PluginInfo,
  PluginManifest,
  PluginImportSourceKind,
  PluginImportCheck,
  PluginImportPreview,
  PluginInstallResult,
} from "./plugins/types";
import { pluginEvents } from "./plugins/events";

function pluginInvoke<T>(
  command: string,
  args: Record<string, unknown>,
): Promise<T> {
  return invoke<T>(command, args).catch((e: unknown) => {
    if (
      e &&
      typeof e === "object" &&
      "code" in e &&
      e.code === "permission_denied"
    )
      pluginEvents.emit("plugin.denied", e);
    if (e && typeof e === "object" && "message" in e)
      throw Object.assign(new Error(String(e.message)), e);
    throw e;
  });
}

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
  /** 重要性星级 0–5 */
  priority?: number;
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
  recurrence?: Recurrence; /** 重要性星级 0–5 */
  priority?: number;
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
  taskWallViews: (day?: string) =>
    invoke<TaskDayView[]>("task_wall_views", { day: day ?? null }),
  listTasks: (scope?: "all" | "active" | "archived") =>
    invoke<Task[]>("list_tasks", { scope }),
  /** 归档任务列表（含归档日，归档页用） */
  listArchivedTasks: () => invoke<ArchivedTask[]>("list_archived_tasks"),
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
        priority: params.priority ?? null,
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
        priority: params.priority ?? null,
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
    invoke<TaskDayView>("task_checkin", {
      id,
      operationId: null,
      actor: actor ?? null,
    }).then((v) => {
      if (v.state === "completed") {
        pluginEvents.emit("task.completed", {
          id: v.task.id,
          title: v.task.title,
        });
      }
      return v;
    }),
  taskDecrement: (id: string, actor?: string) =>
    invoke<TaskDayView>("task_decrement", {
      id,
      operationId: null,
      actor: actor ?? null,
    }),
  taskUndo: (id: string, actor?: string) =>
    invoke<TaskDayView>("task_undo", {
      id,
      operationId: null,
      actor: actor ?? null,
    }),
  taskOverview: (
    id: string,
    period: "week" | "month" | "year",
    anchor?: string,
  ) =>
    invoke<PeriodOverview>("task_overview", {
      id,
      period,
      anchor: anchor ?? null,
    }),

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
  updateLibrary: (
    id: string,
    params: {
      title?: string;
      note?: string;
      icon?: string;
      color?: string;
      anchorDay?: string;
    },
  ) =>
    invoke<DateLibrary>("update_library", {
      id,
      title: params.title ?? null,
      note: params.note ?? null,
      icon: params.icon ?? null,
      color: params.color ?? null,
      anchorDay: params.anchorDay ?? null,
    }),
  archiveLibrary: (
    id: string,
    mode: "keep" | "detach" | "move_to",
    moveTo?: string,
  ) =>
    invoke<DateLibrary>("archive_library", {
      id,
      mode,
      moveTo: moveTo ?? null,
    }),
  restoreLibrary: (id: string) =>
    invoke<DateLibrary>("restore_library", { id }),
  libraryTasks: (libraryId: string) =>
    invoke<Task[]>("library_tasks", { libraryId }),
  libraryHeatmap: (libraryId: string, anchor?: string) =>
    invoke<LibraryYearHeatmap>("library_heatmap", {
      libraryId,
      anchor: anchor ?? null,
    }),
  globalYearHeatmap: (anchor?: string) =>
    invoke<GlobalYearHeatmap>("global_year_heatmap", {
      anchor: anchor ?? null,
    }),
  moveTaskLibrary: (id: string, libraryId: string | null) =>
    invoke<Task>("move_task_library", { id, libraryId }),
  /** 拖拽落位：priority=null 同档内重排；传 0–5 跨档改级 */
  moveTaskPosition: (
    id: string,
    priority: number | null,
    beforeId: string | null,
    afterId: string | null,
  ) => invoke<Task>("move_task_position", { id, priority, beforeId, afterId }),

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
    invoke<Note>("create_note", { title, body, actor: actor ?? null }).then(
      (n) => {
        pluginEvents.emit("note.created", { id: n.id, title: n.title });
        return n;
      },
    ),
  updateNote: (id: string, title: string, body: string) =>
    invoke<Note>("update_note", { id, title, body }),
  deleteNote: (id: string) => invoke<void>("delete_note", { id }),

  // Inbox
  listInbox: (includeProcessed = false) =>
    invoke<InboxItem[]>("list_inbox", { includeProcessed }),
  addInboxItem: (content: string, actor?: string) =>
    invoke<InboxItem>("add_inbox_item", { content, actor: actor ?? null }).then(
      (item) => {
        pluginEvents.emit("inbox.added", {
          id: item.id,
          content: item.content,
        });
        return item;
      },
    ),
  inboxToTask: (id: string) => invoke<ProcessReport>("inbox_to_task", { id }),
  inboxToNote: (id: string) => invoke<ProcessReport>("inbox_to_note", { id }),
  deleteInboxItem: (id: string) => invoke<void>("delete_inbox_item", { id }),

  // Search
  searchAll: (query: string) => invoke<SearchResults>("search_all", { query }),

  // Plugins
  pluginPickImportSource: (kind: PluginImportSourceKind) =>
    invoke<string | null>("plugin_pick_import_source", { kind }),
  pluginPreviewImport: (source: string) =>
    invoke<PluginImportPreview>("plugin_preview_import", { source }),
  pluginImport: (source: string, check: PluginImportCheck) =>
    invoke<PluginInstallResult>("plugin_import", { source, check }),
  pluginList: () => invoke<PluginInfo[]>("plugin_list"),
  pluginReadManifest: (id: string) =>
    invoke<PluginManifest>("plugin_read_manifest", { id }),
  pluginSetEnabled: (id: string, enabled: boolean, fingerprint?: string) =>
    invoke<void>("plugin_set_enabled", { id, enabled, fingerprint }),
  pluginDisableAll: () => invoke<void>("plugin_disable_all"),
  pluginOpenContext: (id: string) =>
    pluginInvoke<{
      token: string;
      plugin_id: string;
      manifest: PluginManifest;
      fingerprint: string;
      revision: number;
    }>("plugin_open_context", { id }),
  pluginCloseContext: (token: string) =>
    invoke<void>("plugin_close_context", { token }),
  pluginLoadSource: (token: string) =>
    pluginInvoke<string>("plugin_load_source", { token }),
  pluginCall: <T>(
    token: string,
    method: string,
    params: Record<string, unknown> = {},
  ) => pluginInvoke<T>("plugin_call", { token, call: { ...params, method } }),
};
