import * as React from "react";
import {
  createApi,
  Lifecycle,
  type PluginApi,
  type PluginManifest,
} from "../../../../packages/plugin-sdk/src/index";
import { api as coreApi } from "../api";
import type { CronRegistry } from "./crons";
import type { EventBus } from "./events";
import type { ModuleRegistry } from "./registry";
export type { PluginApi } from "../../../../packages/plugin-sdk/src/index";
export interface PluginDeps {
  registry: ModuleRegistry;
  events: EventBus;
  crons: CronRegistry;
  onChanged: () => void;
}
export function createPluginApi(
  pluginId: string,
  token: string,
  manifest: PluginManifest,
  deps: PluginDeps,
  lifecycle = new Lifecycle(),
): PluginApi {
  if (pluginId !== manifest.id) throw new Error("插件身份不匹配");
  const owner = pluginId;
  const changed = (off: () => void) => {
    deps.onChanged();
    return () => {
      off();
      deps.onChanged();
    };
  };
  return createApi(
    manifest,
    {
      react: React,
      call: async <T>(method: string, params?: Record<string, unknown>) => {
        const result = await coreApi.pluginCall<T>(token, method, params);
        if (lifecycle.active) {
          const topics: Record<string, string> = {
            create_task: "task.created",
            create_note: "note.created",
            add_inbox_item: "inbox.added",
          };
          if (topics[method]) deps.events.emit(topics[method], result);
          if (
            method === "checkin_task" &&
            (result as { state?: string }).state === "completed"
          )
            deps.events.emit(
              "task.completed",
              (result as { task: unknown }).task,
            );
        }
        return result;
      },
      denied: (action) => {
        deps.events.emit("plugin.denied", { plugin_id: owner, action });
        void coreApi
          .pluginCall(token, "denied", { action })
          .catch(console.error);
      },
      on: (topic, handler) => deps.events.on(topic, owner, handler),
      emit: (topic, payload) => deps.events.emit(topic, payload),
      cron: (expr, handler) =>
        deps.crons.on(owner, expr, async () => {
          await handler();
        }),
      card: (card) =>
        changed(
          deps.registry.registerCard({
            ...card,
            id: `${owner}.${card.id}`,
            owner,
          }),
        ),
      view: (view) =>
        changed(
          deps.registry.registerView({
            ...view,
            key: `${owner}.${view.id}`,
            icon: view.icon ?? "▣",
            owner,
          }),
        ),
      command: (cmd) =>
        changed(
          deps.registry.registerCommand({
            ...cmd,
            id: `${owner}.${cmd.id}`,
            owner,
            handler: async () => {
              await cmd.handler();
            },
          }),
        ),
      settings: (definition) =>
        changed(deps.registry.registerSettings(owner, definition)),
    },
    lifecycle,
  );
}
