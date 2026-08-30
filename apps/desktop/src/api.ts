import { invoke } from "@tauri-apps/api/core";
import type {
  InboxItem,
  Note,
  Project,
  ProjectWithStats,
  SearchResults,
  Task,
  TodayContext,
} from "./types";
import type { PluginInfo, PluginManifest } from "./plugins/types";

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

export const api = {
  // Today / Tasks
  getToday: () => invoke<TodayContext>("get_today"),
  listTasks: (scope?: string) => invoke<Task[]>("list_tasks", { scope }),
  createTask: (title: string, dueAt?: string, projectId?: string) =>
    invoke<Task>("create_task", {
      title,
      dueAt: dueAt || null,
      projectId: projectId || null,
    }),
  setTaskStatus: (id: string, status: string) =>
    invoke<Task>("set_task_status", { id, status }),
  deleteTask: (id: string) => invoke<void>("delete_task", { id }),

  // 带 actor 的写入（插件桥使用；审计 actor=plugin:<id>）
  createTaskAs: (actor: string, title: string, dueAt?: string, projectId?: string) =>
    invoke<Task>("create_task", {
      title,
      dueAt: dueAt || null,
      projectId: projectId || null,
      actor,
    }),
  setTaskStatusAs: (actor: string, id: string, status: string) =>
    invoke<Task>("set_task_status", { id, status, actor }),
  deleteTaskAs: (actor: string, id: string) =>
    invoke<void>("delete_task", { id, actor }),
  createNoteAs: (actor: string, title: string, body: string) =>
    invoke<Note>("create_note", { title, body, actor }),
  addInboxItemAs: (actor: string, content: string) =>
    invoke<InboxItem>("add_inbox_item", { content, actor }),

  // Projects
  listProjects: () => invoke<ProjectWithStats[]>("list_projects"),
  createProject: (name: string, description: string) =>
    invoke<Project>("create_project", { name, description: description ?? "" }),
  archiveProject: (id: string) => invoke<void>("archive_project", { id }),
  deleteProject: (id: string) => invoke<void>("delete_project", { id }),

  // Notes
  listNotes: (limit = 200) => invoke<Note[]>("list_notes", { limit }),
  createNote: (title: string, body: string) =>
    invoke<Note>("create_note", { title, body }),
  updateNote: (id: string, title: string, body: string) =>
    invoke<Note>("update_note", { id, title, body }),
  deleteNote: (id: string) => invoke<void>("delete_note", { id }),

  // Inbox
  listInbox: (includeProcessed = false) =>
    invoke<InboxItem[]>("list_inbox", { includeProcessed }),
  addInboxItem: (content: string) => invoke<InboxItem>("add_inbox_item", { content }),
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
