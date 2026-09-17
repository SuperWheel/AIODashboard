import type * as React from "react";
import type {
  Task,
  TaskDayView,
  TodayContext,
  SearchResults,
  InboxItem,
  Note,
} from "./data.js";
export type {
  Task as PluginTask,
  Task,
  TaskDayView,
  TodayContext,
  SearchResults,
  InboxItem,
  Note,
} from "./data.js";
export type Disposer = () => void;
export type TaskScope = "all" | "active" | "archived";
export type CardSize = "sm" | "md" | "lg";
export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  entry: string;
  api_version?: string;
  min_host_version?: string;
  description?: string;
  author?: string;
  license?: string;
  homepage?: string;
  permissions?: {
    core?: string[];
    ui?: string[];
    network?: string[];
    events?: string[];
    cron?: string[];
    storage_quota_bytes?: number | null;
  };
  contributions?: {
    views?: { id: string; title: string }[];
    today_cards?: { id: string }[];
    commands?: { id: string; title: string }[];
    settings?: { id: string; title: string }[];
  };
}
export interface PluginCardProps {
  api: PluginApi | null;
  onChanged: () => void;
  today: TodayContext | null;
}
export interface PluginViewProps {
  api?: PluginApi;
  onChanged: () => void;
  onNav: (v: string, param?: string) => void;
  refreshKey: number;
  today: TodayContext | null;
  navParam?: string;
  cards?: React.ReactNode;
}
export interface TodayCard {
  id: string;
  title: string;
  size?: CardSize;
  component: React.ComponentType<PluginCardProps>;
}
export interface PluginView {
  id: string;
  title: string;
  icon?: string;
  component: React.ComponentType<PluginViewProps>;
}
export interface PluginCommand {
  id: string;
  title: string;
  handler: () => unknown | Promise<unknown>;
}
export type SettingValue = string | number | boolean;
export type SettingField = { key: string; label: string } & (
  | { type: "text"; default: string }
  | { type: "number"; default: number; min?: number; max?: number }
  | { type: "boolean"; default: boolean }
  | { type: "select"; default: string; options: string[] }
);
export interface SettingsDefinition {
  id: string;
  title: string;
  fields: SettingField[];
}
export interface FetchResult {
  status: number;
  text: string;
  json: unknown;
}
export class PluginPermissionError extends Error {
  readonly code = "permission_denied";
  constructor(message: string) {
    super(message);
    this.name = "PluginPermissionError";
  }
}
export function ownsTopic(id: string, topic: string): boolean {
  const prefix = `plugin.${id}:`;
  return (
    topic.startsWith(prefix) &&
    topic.length > prefix.length &&
    new TextEncoder().encode(topic.slice(prefix.length)).length <= 100
  );
}
export function allowed(m: PluginManifest, action: string): boolean {
  const p = m.permissions ?? {};
  if (action.startsWith("core."))
    return p.core?.includes(action.slice(5)) ?? false;
  if (action === "storage") return p.storage_quota_bytes != null;
  if (action === "network") return (p.network?.length ?? 0) > 0;
  if (action.startsWith("ui.")) {
    const [kind, id] = action.slice(3).split(":");
    const groups: Record<string, string> = {
      view: "views",
      today_card: "today_cards",
      command: "commands",
      settings: "settings",
    };
    const group = groups[kind] as keyof NonNullable<
      PluginManifest["contributions"]
    >;
    return (
      !!p.ui?.includes(kind) &&
      !!m.contributions?.[group]?.some((v) => v.id === id)
    );
  }
  if (action.startsWith("cron:"))
    return p.cron?.includes(action.slice(5)) ?? false;
  if (action.startsWith("event.on:")) {
    const topic = action.slice(9);
    return (
      ["panel.refresh", "panel.show", "panel.hide"].includes(topic) ||
      !!p.events?.includes(topic) ||
      ownsTopic(m.id, topic)
    );
  }
  if (action.startsWith("event.emit:"))
    return ownsTopic(m.id, action.slice(11));
  return false;
}
export class Lifecycle {
  active = true;
  private disposers = new Set<Disposer>();
  assert(): void {
    if (!this.active) throw new PluginPermissionError("插件会话已关闭");
  }
  add(dispose: Disposer): Disposer {
    this.assert();
    let done = false;
    const off = () => {
      if (done) return;
      done = true;
      this.disposers.delete(off);
      dispose();
    };
    this.disposers.add(off);
    return off;
  }
  dispose(): void {
    if (!this.active) return;
    this.active = false;
    for (const off of [...this.disposers]) {
      try {
        off();
      } catch (e) {
        console.error(e);
      }
    }
    this.disposers.clear();
  }
}
export interface PluginAdapter {
  react: typeof React;
  call: <T>(method: string, params?: Record<string, unknown>) => Promise<T>;
  denied: (action: string) => void;
  on: (topic: string, handler: (payload: unknown) => void) => Disposer;
  emit: (topic: string, payload: unknown) => void;
  cron: (expr: string, handler: () => unknown) => Disposer;
  card: (card: TodayCard) => Disposer;
  view: (view: PluginView) => Disposer;
  command: (cmd: PluginCommand) => Disposer;
  settings: (definition: SettingsDefinition) => Disposer;
}
export interface PluginApi {
  readonly pluginId: string;
  readonly react: typeof React;
  readonly core: {
    today(): Promise<TodayContext>;
    listTasks(scope?: TaskScope): Promise<Task[]>;
    createTask(title: string, target?: number): Promise<Task>;
    checkinTask(id: string, operationId?: string): Promise<TaskDayView>;
    archiveTask(id: string): Promise<Task>;
    deleteTask(id: string): Promise<void>;
    search(query: string): Promise<SearchResults>;
    addInboxItem(content: string): Promise<InboxItem>;
    createNote(title: string, body: string): Promise<Note>;
  };
  readonly storage: {
    kv: {
      get(key: string): Promise<string | null>;
      set(key: string, value: string): Promise<void>;
      delete(key: string): Promise<boolean>;
      list(prefix?: string): Promise<{ key: string; value: string }[]>;
    };
  };
  fetch(url: string): Promise<FetchResult>;
  readonly events: {
    on(topic: string, handler: (payload: unknown) => unknown): Disposer;
    emit(topic: string, payload: unknown): void;
  };
  registerCron(expr: string, handler: () => unknown): Disposer;
  registerInterval(handler: () => unknown, ms: number): Disposer;
  readonly ui: {
    registerTodayCard(card: TodayCard): Disposer;
    registerView(view: PluginView): Disposer;
    registerCommand(command: PluginCommand): Disposer;
    registerSettings(definition: SettingsDefinition): Disposer;
  };
  readonly settings: {
    get(id: string): Promise<Record<string, SettingValue>>;
    set(id: string, values: Record<string, SettingValue>): Promise<void>;
  };
  readonly log: {
    info(message: string): void;
    warn(message: string): void;
    error(message: string): void;
  };
}
export function validateSettings(
  def: SettingsDefinition,
  values: Record<string, SettingValue>,
): void {
  if (
    !Array.isArray(def.fields) ||
    def.fields.length > 30 ||
    new Set(def.fields.map((f) => f.key)).size !== def.fields.length
  )
    throw new Error("设置字段非法或重复");
  if (Object.keys(values).some((key) => !def.fields.some((f) => f.key === key)))
    throw new Error("未知设置字段");
  for (const f of def.fields) {
    if (!/^[a-zA-Z0-9_-]{1,64}$/.test(f.key) || !f.label.trim())
      throw new Error("设置字段名称非法");
    const v = values[f.key] ?? f.default;
    if (f.type === "boolean" && typeof v !== "boolean")
      throw new Error("设置须为布尔值");
    if (
      f.type === "number" &&
      (typeof v !== "number" ||
        !Number.isFinite(v) ||
        (f.min !== undefined && v < f.min) ||
        (f.max !== undefined && v > f.max))
    )
      throw new Error("设置数值越界");
    if (
      (f.type === "text" || f.type === "select") &&
      (typeof v !== "string" || v.length > 4096)
    )
      throw new Error("设置须为有限长度文本");
    if (f.type === "select" && !f.options.includes(v as string))
      throw new Error("设置选项非法");
  }
}
export function createApi(
  manifest: PluginManifest,
  adapter: PluginAdapter,
  lifecycle = new Lifecycle(),
): PluginApi {
  const denied = (action: string): never => {
    adapter.denied(action);
    throw new PluginPermissionError(`未声明能力 ${action}`);
  };
  const check = (action: string) => {
    lifecycle.assert();
    if (!allowed(manifest, action)) denied(action);
  };
  const request = async <T>(
    cap: string,
    method: string,
    params?: Record<string, unknown>,
  ): Promise<T> => {
    check(cap);
    const value = await adapter.call<T>(method, params);
    lifecycle.assert();
    return value;
  };
  const callback =
    (f: (...args: any[]) => unknown) =>
    (...args: any[]) => {
      if (lifecycle.active) return f(...args);
    };
  const definitions = new Map<string, SettingsDefinition>();
  const api: PluginApi = {
    pluginId: manifest.id,
    react: adapter.react,
    core: {
      today: () => request("core.context.read", "today"),
      listTasks: (scope) => request("core.task.read", "list_tasks", { scope }),
      createTask: (title, target) =>
        request("core.task.write", "create_task", { title, target }),
      checkinTask: (id, operationId) =>
        request("core.task.write", "checkin_task", {
          id,
          operation_id: operationId,
        }),
      archiveTask: (id) => request("core.task.write", "archive_task", { id }),
      deleteTask: (id) => request("core.task.write", "delete_task", { id }),
      search: (query) => request("core.search.read", "search", { query }),
      addInboxItem: (content) =>
        request("core.inbox.write", "add_inbox_item", { content }),
      createNote: (title, body) =>
        request("core.note.write", "create_note", { title, body }),
    },
    storage: {
      kv: {
        get: (key) => request("storage", "kv_get", { key }),
        set: (key, value) => request("storage", "kv_set", { key, value }),
        delete: (key) => request("storage", "kv_delete", { key }),
        list: (prefix) => request("storage", "kv_list", { prefix }),
      },
    },
    fetch: (raw) => {
      check("network");
      let url: URL;
      try {
        url = new URL(raw);
      } catch {
        denied("network.url");
      }
      if (
        !["https:", "http:"].includes(url!.protocol) ||
        url!.username ||
        url!.password ||
        !manifest.permissions?.network?.includes(url!.hostname)
      )
        denied("network.host");
      return request("network", "fetch", { url: raw });
    },
    events: {
      on: (topic, handler) => {
        check(`event.on:${topic}`);
        return lifecycle.add(adapter.on(topic, callback(handler)));
      },
      emit: (topic, payload) => {
        check(`event.emit:${topic}`);
        adapter.emit(topic, payload);
      },
    },
    registerCron: (expr, handler) => {
      check(`cron:${expr}`);
      return lifecycle.add(adapter.cron(expr, callback(handler)));
    },
    registerInterval: (handler, ms) => {
      lifecycle.assert();
      if (!Number.isFinite(ms) || ms < 100)
        throw new Error("定时器间隔至少 100ms");
      const t = setInterval(() => {
        try {
          Promise.resolve(callback(handler)()).catch(console.error);
        } catch (e) {
          console.error(e);
        }
      }, ms);
      return lifecycle.add(() => clearInterval(t));
    },
    ui: {
      registerTodayCard: (card) => {
        check(`ui.today_card:${card.id}`);
        if (card.size && !["sm", "md", "lg"].includes(card.size))
          throw new Error("size 非法");
        return lifecycle.add(adapter.card(card));
      },
      registerView: (view) => {
        check(`ui.view:${view.id}`);
        return lifecycle.add(adapter.view(view));
      },
      registerCommand: (cmd) => {
        check(`ui.command:${cmd.id}`);
        return lifecycle.add(
          adapter.command({ ...cmd, handler: callback(cmd.handler) }),
        );
      },
      registerSettings: (def) => {
        check(`ui.settings:${def.id}`);
        check("storage");
        validateSettings(def, {});
        if (definitions.has(def.id)) throw new Error("设置重复注册");
        definitions.set(def.id, def);
        const off = adapter.settings(def);
        return lifecycle.add(() => {
          off();
          definitions.delete(def.id);
        });
      },
    },
    settings: {
      get: async (id) => {
        check(`ui.settings:${id}`);
        const def = definitions.get(id);
        if (!def) throw new Error("设置未注册");
        const raw = await api.storage.kv.get(`@settings:${id}`);
        const values = raw ? JSON.parse(raw) : {};
        validateSettings(def, values);
        return Object.fromEntries(
          def.fields.map((f) => [f.key, values[f.key] ?? f.default]),
        );
      },
      set: async (id, values) => {
        check(`ui.settings:${id}`);
        const def = definitions.get(id);
        if (!def) throw new Error("设置未注册");
        validateSettings(def, values);
        await api.storage.kv.set(`@settings:${id}`, JSON.stringify(values));
      },
    },
    log: {
      info: (m) => console.info(`[${manifest.id}] ${m}`),
      warn: (m) => console.warn(`[${manifest.id}] ${m}`),
      error: (m) => console.error(`[${manifest.id}] ${m}`),
    },
  };
  return Object.freeze(api);
}
export type PluginOnload = (api: PluginApi) => void | Promise<void>;
export type PluginOnunload = () => void | Promise<void>;
export function definePlugin<T extends PluginOnload>(onload: T): T {
  return onload;
}
