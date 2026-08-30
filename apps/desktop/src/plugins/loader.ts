// 平台粘合层：扫描 → 校验 → blob 动态 import → onload。Tauri 环境专用（不进单测）。
// 首次发现的插件自动登记（默认启用）；单个插件加载失败不影响其余插件。

import { api } from "../api";
import { createPluginApi, type PluginApi } from "./bridge";
import { validateManifest } from "./manifest";
import type { EventBus } from "./events";
import type { ModuleRegistry } from "./registry";

export interface LoadedPlugin {
  id: string;
  api: PluginApi;
  dispose: () => Promise<void>;
}

export interface PluginHostOptions {
  registry: ModuleRegistry;
  events: EventBus;
  onChanged: () => void;
}

export async function loadAllPlugins(opts: PluginHostOptions): Promise<LoadedPlugin[]> {
  const loaded: LoadedPlugin[] = [];
  let infos;
  try {
    infos = await api.pluginList();
  } catch (e) {
    console.error("[plugins] 扫描插件目录失败:", e);
    return loaded;
  }

  for (const info of infos) {
    if (info.error) {
      console.warn(`[plugins] 跳过非法插件 ${info.id}: ${info.error}`);
      continue;
    }
    try {
      if (info.enabled === null) {
        // 首次发现：登记并默认启用
        await api.pluginSetEnabled(info.id, true);
      } else if (!info.enabled) {
        continue;
      }
      loaded.push(await loadPlugin(info.id, opts));
    } catch (e) {
      console.error(`[plugins] 加载 ${info.id} 失败:`, e);
    }
  }
  return loaded;
}

async function loadPlugin(id: string, opts: PluginHostOptions): Promise<LoadedPlugin> {
  const manifest = await api.pluginReadManifest(id);
  const errors = validateManifest(manifest);
  if (errors.length > 0) {
    throw new Error(`manifest 校验失败: ${errors.join("; ")}`);
  }
  const source = await api.pluginLoadSource(id);

  const url = URL.createObjectURL(new Blob([source], { type: "text/javascript" }));
  let mod: Record<string, unknown>;
  try {
    mod = (await import(/* @vite-ignore */ url)) as Record<string, unknown>;
  } finally {
    URL.revokeObjectURL(url);
  }

  if (typeof mod.onload !== "function") {
    throw new Error("插件缺少 onload 导出");
  }
  const apiObj = createPluginApi(id, manifest, opts);
  await (mod.onload as (api: PluginApi) => Promise<void> | void)(apiObj);

  return {
    id,
    api: apiObj,
    dispose: async () => {
      try {
        if (typeof mod.onunload === "function") {
          await (mod.onunload as () => Promise<void> | void)();
        }
      } finally {
        // 无论 onunload 是否抛错，注册项与订阅必须清理干净（T8）
        opts.registry.unregisterOwner(id);
        opts.events.offOwner(id);
        opts.onChanged();
      }
    },
  };
}
