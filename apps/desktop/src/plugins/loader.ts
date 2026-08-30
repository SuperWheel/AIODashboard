// 平台粘合层：扫描 → 校验 → blob 动态 import → onload。Tauri 环境专用（不进单测）。
// 首次发现的插件自动登记（默认启用）；单个插件加载失败不影响其余插件。

import { api } from "../api";
import { createPluginApi, type PluginApi } from "./bridge";
import type { CronRegistry } from "./crons";
import { validateManifest } from "./manifest";
import type { EventBus } from "./events";
import type { ModuleRegistry } from "./registry";
import type { PluginInfo } from "./types";

export interface LoadedPlugin {
  id: string;
  api: PluginApi;
  dispose: () => Promise<void>;
}

export interface PluginHostOptions {
  registry: ModuleRegistry;
  events: EventBus;
  crons: CronRegistry;
  onChanged: () => void;
}

export interface LoadAllResult {
  plugins: LoadedPlugin[];
  /** 磁盘上首次发现、等待用户确认权限的插件 */
  pending: PluginInfo[];
}

/** onload 看门狗：超时视为坏插件 */
const ONLOAD_TIMEOUT_MS = 8000;

export async function loadAllPlugins(opts: PluginHostOptions): Promise<LoadAllResult> {
  const plugins: LoadedPlugin[] = [];
  const pending: PluginInfo[] = [];
  let infos;
  try {
    infos = await api.pluginList();
  } catch (e) {
    console.error("[plugins] 扫描插件目录失败:", e);
    return { plugins, pending };
  }

  for (const info of infos) {
    if (info.error) {
      console.warn(`[plugins] 跳过非法插件 ${info.id}: ${info.error}`);
      continue;
    }
    if (info.enabled === null) {
      // 首次发现：交由用户确认权限后启用
      pending.push(info);
      continue;
    }
    if (!info.enabled) continue;
    try {
      plugins.push(await loadPlugin(info.id, opts));
    } catch (e) {
      console.error(`[plugins] 加载 ${info.id} 失败，自动停用:`, e);
      await api.pluginSetEnabled(info.id, false).catch(() => {});
    }
  }
  return { plugins, pending };
}

function withTimeout<T>(p: Promise<T>, ms: number, msg: string): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const t = setTimeout(() => reject(new Error(msg)), ms);
    p.then(
      (v) => {
        clearTimeout(t);
        resolve(v);
      },
      (e) => {
        clearTimeout(t);
        reject(e);
      },
    );
  });
}

export async function loadPlugin(id: string, opts: PluginHostOptions): Promise<LoadedPlugin> {
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
  // 看门狗：onload 挂死（超时或抛错）都会让上层禁用该插件
  await withTimeout(
    Promise.resolve(
      (mod.onload as (api: PluginApi) => Promise<void> | void)(apiObj),
    ),
    ONLOAD_TIMEOUT_MS,
    `插件 ${id} 的 onload 超过 ${ONLOAD_TIMEOUT_MS}ms 未完成（看门狗）`,
  );

  return {
    id,
    api: apiObj,
    dispose: async () => {
      try {
        if (typeof mod.onunload === "function") {
          await withTimeout(
            Promise.resolve((mod.onunload as () => Promise<void> | void)()),
            ONLOAD_TIMEOUT_MS,
            `插件 ${id} 的 onunload 超时（忽略）`,
          );
        }
      } finally {
        // 无论 onunload 是否抛错，注册项与订阅必须清理干净（T8）
        opts.registry.unregisterOwner(id);
        opts.events.offOwner(id);
        opts.crons.offOwner(id);
        opts.onChanged();
      }
    },
  };
}
