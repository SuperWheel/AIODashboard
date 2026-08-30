// 插件 API 桥：插件能 touch 到的唯一面。所有越权调用在此拒绝（T10），
// 服务端 Tauri command 再做第二道校验（白名单 / 命名空间 / 审计）。
// 依赖注入便于单测：coreApi / registry / events 均可替换为测试桩。

import * as React from "react";
import { api as coreApi } from "../api";
import { extractHost } from "./manifest";
import type { CronRegistry } from "./crons";
import type { EventBus } from "./events";
import type { ModuleRegistry } from "./registry";
import type { PluginManifest } from "./types";

export interface PluginDeps {
  registry: ModuleRegistry;
  events: EventBus;
  crons: CronRegistry;
  /** 注册贡献点后通知宿主重渲染 */
  onChanged: () => void;
}

export interface PluginApi {
  pluginId: string;
  /** 宿主共享的 React（createElement / hooks），保证单实例 */
  react: typeof React;
  core: {
    today: () => ReturnType<typeof coreApi.getToday>;
    listTasks: (scope?: string) => ReturnType<typeof coreApi.listTasks>;
    createTask: (title: string, dueAt?: string) => ReturnType<typeof coreApi.createTaskAs>;
    setTaskStatus: (id: string, status: string) => ReturnType<typeof coreApi.setTaskStatusAs>;
    deleteTask: (id: string) => ReturnType<typeof coreApi.deleteTaskAs>;
    search: (query: string) => ReturnType<typeof coreApi.searchAll>;
    addInboxItem: (content: string) => ReturnType<typeof coreApi.addInboxItemAs>;
    createNote: (title: string, body: string) => ReturnType<typeof coreApi.createNoteAs>;
  };
  storage: {
    get: (key: string) => Promise<string | null>;
    set: (key: string, value: string) => Promise<void>;
    delete: (key: string) => Promise<boolean>;
    list: (keyPrefix?: string) => Promise<{ key: string; value: string }[]>;
  };
  fetch: (url: string) => Promise<{ status: number; text: string; json: unknown }>;
  events: {
    on: (topic: string, handler: (payload: unknown) => void) => () => void;
    emit: (topic: string, payload: unknown) => void;
  };
  /** 注册 cron（Rust 侧驱动，后台不被定时器节流影响）；expr 必须在 manifest 声明 */
  registerCron: (expr: string, handler: () => void | Promise<void>) => void;
  ui: {
    registerTodayCard: (card: {
      id: string;
      title: string;
      /** Bento 占位：sm/md/lg（缺省 md）；非法值视为插件错误 */
      size?: import("./registry").CardSize;
      component: React.ComponentType<import("./registry").PluginCardProps>;
    }) => void;
    registerView: (view: {
      id: string;
      title: string;
      icon?: string;
      component: React.ComponentType<import("./registry").PluginViewProps>;
    }) => void;
    registerCommand: (cmd: {
      id: string;
      title: string;
      handler: () => void | Promise<void>;
    }) => void;
  };
  log: {
    info: (msg: string) => void;
    warn: (msg: string) => void;
    error: (msg: string) => void;
  };
}

export function createPluginApi(
  pluginId: string,
  manifest: PluginManifest,
  deps: PluginDeps,
): PluginApi {
  const actor = `plugin:${pluginId}`;
  const perms = manifest.permissions ?? {};

  return {
    pluginId,
    react: React,

    core: {
      today: () => coreApi.getToday(),
      listTasks: (scope?: string) => coreApi.listTasks(scope),
      createTask: (title: string, dueAt?: string) =>
        coreApi.createTaskAs(actor, title, dueAt),
      setTaskStatus: (id: string, status: string) =>
        coreApi.setTaskStatusAs(actor, id, status),
      deleteTask: (id: string) => coreApi.deleteTaskAs(actor, id),
      search: (query: string) => coreApi.searchAll(query),
      addInboxItem: (content: string) => coreApi.addInboxItemAs(actor, content),
      createNote: (title: string, body: string) =>
        coreApi.createNoteAs(actor, title, body),
    },

    storage: {
      get: (key: string) => coreApi.pluginKvGet(pluginId, key),
      set: async (key: string, value: string) => {
        await coreApi.pluginKvSet(pluginId, key, value);
      },
      delete: (key: string) => coreApi.pluginKvDelete(pluginId, key),
      list: async (keyPrefix?: string) => {
        const entries = await coreApi.pluginKvList(pluginId, keyPrefix);
        return entries.map((e) => ({ key: e.key, value: e.value }));
      },
    },

    fetch: async (url: string) => {
      const host = extractHost(url);
      if (!host || !(perms.network ?? []).includes(host)) {
        throw new Error(`network 权限未包含 host '${host ?? "?"}'`);
      }
      return coreApi.pluginHttpFetch(pluginId, url);
    },

    events: {
      on: (topic: string, handler: (payload: unknown) => void) => {
        // 领域事件须在 manifest 声明；panel.* 面板事件无需声明
        if (!topic.startsWith("panel.") && !(perms.events ?? []).includes(topic)) {
          throw new Error(`events 权限未包含 '${topic}'`);
        }
        return deps.events.on(topic, pluginId, handler);
      },
      emit: (topic: string, payload: unknown) => deps.events.emit(topic, payload),
    },

    registerCron: (expr: string, handler: () => void | Promise<void>) => {
      if (!(perms.cron ?? []).includes(expr)) {
        throw new Error(`cron 权限未包含 '${expr}'（需在 manifest permissions.cron 声明）`);
      }
      deps.crons.on(pluginId, expr, handler);
    },

    ui: {
      registerTodayCard: (card) => {
        if (card.size !== undefined && !["sm", "md", "lg"].includes(card.size)) {
          throw new Error(`卡片 size 非法: '${card.size}'（可选 "sm" | "md" | "lg"）`);
        }
        deps.registry.registerCard({
          owner: pluginId,
          id: `${pluginId}.${card.id}`,
          title: card.title,
          component: card.component,
          size: card.size,
        });
        deps.onChanged();
      },
      registerView: (view) => {
        deps.registry.registerView({
          owner: pluginId,
          key: `${pluginId}.${view.id}`,
          title: view.title,
          icon: view.icon ?? "▣",
          component: view.component,
        });
        deps.onChanged();
      },
      registerCommand: (cmd) => {
        deps.registry.registerCommand({
          owner: pluginId,
          id: `${pluginId}.${cmd.id}`,
          title: cmd.title,
          handler: cmd.handler,
        });
        deps.onChanged();
      },
    },

    log: {
      info: (msg: string) => console.info(`[${pluginId}] ${msg}`),
      warn: (msg: string) => console.warn(`[${pluginId}] ${msg}`),
      error: (msg: string) => console.error(`[${pluginId}] ${msg}`),
    },
  };
}
