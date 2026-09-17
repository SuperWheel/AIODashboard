import * as React from "react";
import {
  createApi,
  Lifecycle,
  PluginPermissionError,
  type PluginManifest,
  type PluginApi,
  type SettingsDefinition,
  type Task,
} from "@aiodashboard/plugin-sdk";
export interface TestOptions {
  kv?: Map<string, string>;
  responses?: Record<string, unknown>;
  core?: Record<string, (params: Record<string, unknown>) => unknown>;
}
export function createPluginTestContext(
  manifest: PluginManifest,
  options: TestOptions = {},
) {
  const life = new Lifecycle();
  const commands = new Map<string, () => unknown>();
  const cards = new Set<string>();
  const views = new Set<string>();
  const settings = new Map<string, SettingsDefinition>();
  const subscriptions = new Map<string, Set<(p: unknown) => void>>();
  const crons = new Map<string, Set<() => unknown>>();
  const emitted: Array<{ topic: string; payload: unknown }> = [];
  const denied: string[] = [];
  const kv = options.kv ?? new Map<string, string>();
  const tasks: Task[] = [];
  const emit = (topic: string, payload: unknown) => {
    emitted.push({ topic, payload });
    for (const f of subscriptions.get(topic) ?? []) f(payload);
  };
  const add = <T>(map: Map<string, Set<T>>, key: string, value: T) => {
    const set = map.get(key) ?? new Set<T>();
    map.set(key, set);
    set.add(value);
    return () => {
      set.delete(value);
      if (!set.size) map.delete(key);
    };
  };
  const register = (set: Set<string>, id: string) => {
    if (set.has(id)) throw new Error("注册冲突");
    set.add(id);
    return () => {
      set.delete(id);
    };
  };
  const call = async <T>(
    method: string,
    params: Record<string, unknown> = {},
  ): Promise<T> => {
    const key = String(params.key ?? "");
    const value = String(params.value ?? "");
    let out: unknown;
    if (options.core?.[method]) out = await options.core[method](params);
    else
      switch (method) {
        case "kv_get":
          out = kv.get(key) ?? null;
          break;
        case "kv_set": {
          const bytes = (s: string) => new TextEncoder().encode(s).length;
          if (
            !key ||
            bytes(key) > 200 ||
            /[\u0000-\u001f\u007f]/.test(key) ||
            bytes(value) > 1_000_000
          )
            throw new Error("KV 输入非法");
          const next = new Map(kv);
          next.set(key, value);
          const used = [...next].reduce(
            (n, [k, v]) => n + bytes(k) + bytes(v),
            0,
          );
          if (used > (manifest.permissions?.storage_quota_bytes ?? 0)) {
            denied.push("storage");
            throw new PluginPermissionError("KV quota 超限");
          }
          kv.set(key, value);
          break;
        }
        case "kv_delete":
          out = kv.delete(key);
          break;
        case "kv_list":
          out = [...kv]
            .filter(([k]) => k.startsWith(String(params.prefix ?? "")))
            .sort(([a], [b]) => a.localeCompare(b))
            .map(([key, value]) => ({ key, value }));
          break;
        case "fetch":
          if (!(String(params.url) in (options.responses ?? {})))
            throw new Error("测试必须显式提供网络响应");
          out = options.responses![String(params.url)];
          break;
        case "list_tasks":
          out = tasks.filter(
            (t) =>
              !params.scope ||
              params.scope === "all" ||
              t.status === params.scope,
          );
          break;
        case "create_task": {
          const t: Task = {
            id: `tsk_test_${tasks.length + 1}`,
            title: String(params.title),
            status: "active",
            icon: "",
            color_hex: "#4A90E2",
            unit: "",
            card_style: "day",
            priority: 0,
            sort_order: 0,
            created_at: "2000-01-01T00:00:00Z",
            updated_at: "2000-01-01T00:00:00Z",
          };
          tasks.push(t);
          out = t;
          break;
        }
        default:
          throw new Error(`测试桩未提供 ${method}，请传入 core fixture`);
      }
    return out as T;
  };
  const api: PluginApi = createApi(
    manifest,
    {
      react: React,
      call,
      denied: (a) => denied.push(a),
      emit,
      on: (topic, f) => add(subscriptions, topic, f),
      cron: (expr, f) => add(crons, expr, f),
      card: (c) => register(cards, c.id),
      view: (v) => register(views, v.id),
      command: (c) => {
        if (commands.has(c.id)) throw new Error("命令冲突");
        commands.set(c.id, c.handler);
        return () => {
          commands.delete(c.id);
        };
      },
      settings: (def) => {
        if (settings.has(def.id)) throw new Error("设置冲突");
        settings.set(def.id, def);
        return () => {
          settings.delete(def.id);
        };
      },
    },
    life,
  );
  return {
    api,
    commands,
    cards,
    views,
    settings,
    subscriptions,
    crons,
    emitted,
    denied,
    kv,
    dispose: () => life.dispose(),
    emitHost: emit,
    fireCron: async (expr: string) => {
      for (const f of crons.get(expr) ?? []) await f();
    },
  };
}
