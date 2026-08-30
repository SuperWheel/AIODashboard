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

export interface ProcessReport {
  inbox_id: string;
  created_type: "task" | "note";
  created_id: string;
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
};
