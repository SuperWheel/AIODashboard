import { api } from "../api";
import { createPluginApi, type PluginApi, type PluginDeps } from "./bridge";
import { Lifecycle } from "../../../../packages/plugin-sdk/src/index";
import { validateManifest } from "./manifest";
import type { PluginInfo, PluginManifest } from "./types";
export interface LoadedPlugin {
  id: string;
  api: PluginApi;
  manifest: PluginManifest;
  dispose: () => Promise<void>;
}
export type PluginHostOptions = PluginDeps & {
  onLoadError?: (id: string, error: unknown) => void;
};
export interface LoadAllResult {
  plugins: LoadedPlugin[];
  pending: PluginInfo[];
}
type PluginModule = {
  onload?: (api: PluginApi) => unknown;
  onunload?: () => unknown;
};
export async function bounded<T>(
  work: () => Promise<T> | T,
  signal: AbortSignal,
  ms = 8000,
): Promise<T> {
  if (signal.aborted) throw new Error("插件加载已取消");
  return new Promise<T>((resolve, reject) => {
    const abort = () => {
      clean();
      reject(new Error("插件加载已取消"));
    };
    const timer = setTimeout(() => {
      clean();
      reject(new Error("插件生命周期超过 8 秒"));
    }, ms);
    const clean = () => {
      clearTimeout(timer);
      signal.removeEventListener("abort", abort);
    };
    signal.addEventListener("abort", abort, { once: true });
    Promise.resolve()
      .then(work)
      .then(
        (v) => {
          clean();
          resolve(v);
        },
        (e) => {
          clean();
          reject(e);
        },
      );
  });
}
export async function activateModule(
  id: string,
  manifest: PluginManifest,
  token: string,
  mod: PluginModule,
  opts: PluginHostOptions,
  signal: AbortSignal,
): Promise<LoadedPlugin> {
  const life = new Lifecycle();
  const apiObj = createPluginApi(id, token, manifest, opts, life);
  const clean = () => {
    life.dispose();
    opts.onChanged();
  };
  signal.addEventListener("abort", clean, { once: true });
  try {
    if (typeof mod.onload !== "function")
      throw new Error("插件缺少 onload 导出");
    await bounded(() => mod.onload!(apiObj), signal);
    life.assert();
  } catch (e) {
    clean();
    signal.removeEventListener("abort", clean);
    throw e;
  }
  let disposed = false;
  return {
    id,
    api: apiObj,
    manifest,
    dispose: async () => {
      if (disposed) return;
      disposed = true;
      clean();
      signal.removeEventListener("abort", clean);
      await api.pluginCloseContext(token).catch(console.error);
      if (typeof mod.onunload === "function")
        await bounded(
          () => mod.onunload!(),
          new AbortController().signal,
        ).catch(console.error);
    },
  };
}
export async function loadPlugin(
  id: string,
  opts: PluginHostOptions,
  signal = new AbortController().signal,
): Promise<LoadedPlugin> {
  let token: string | undefined;
  let url: string | undefined;
  let expired = false;
  try {
    const opening = api.pluginOpenContext(id);
    void opening
      .then((s) => {
        if (signal.aborted || expired) void api.pluginCloseContext(s.token);
      })
      .catch(() => {});
    const session = await bounded(() => opening, signal);
    token = session.token;
    const errors = validateManifest(session.manifest);
    if (errors.length) throw new Error(errors.join("; "));
    const source = await bounded(
      () => api.pluginLoadSource(session.token),
      signal,
    );
    url = URL.createObjectURL(new Blob([source], { type: "text/javascript" }));
    const importUrl = url;
    const mod = await bounded(
      () => import(/* @vite-ignore */ importUrl) as Promise<PluginModule>,
      signal,
    );
    return await activateModule(
      id,
      session.manifest,
      session.token,
      mod,
      opts,
      signal,
    );
  } catch (e) {
    expired = true;
    if (token) await api.pluginCloseContext(token).catch(console.error);
    throw e;
  } finally {
    if (url) URL.revokeObjectURL(url);
  }
}
export async function loadAllPlugins(
  opts: PluginHostOptions,
  signal = new AbortController().signal,
): Promise<LoadAllResult> {
  const plugins: LoadedPlugin[] = [];
  const pending: PluginInfo[] = [];
  for (const info of await api.pluginList()) {
    if (signal.aborted) break;
    if (info.error || info.legacy) continue;
    if (info.pending_approval) {
      pending.push(info);
      continue;
    }
    if (!info.enabled) continue;
    try {
      plugins.push(await loadPlugin(info.id, opts, signal));
    } catch (e) {
      if (!signal.aborted) {
        console.error(e);
        opts.onLoadError?.(info.id, e);
        await api.pluginSetEnabled(info.id, false).catch(console.error);
      }
    }
  }
  if (signal.aborted) {
    await Promise.all(plugins.map((p) => p.dispose()));
    return { plugins: [], pending: [] };
  }
  return { plugins, pending };
}
/** 单一队列合并重载请求；旧代在卸载完成前不会启动下一代。 */
export class PluginHost {
  private stopped = false;
  private controller = new AbortController();
  private loaded: LoadedPlugin[] = [];
  private queued = false;
  private running: Promise<void> | null = null;
  private fingerprint = "";
  constructor(
    private opts: PluginHostOptions,
    private update: (r: LoadAllResult) => void,
  ) {}
  reload(): Promise<void> {
    if (this.stopped) return Promise.resolve();
    this.queued = true;
    this.controller.abort();
    if (!this.running)
      this.running = this.drain().finally(() => {
        this.running = null;
      });
    return this.running;
  }
  private async drain() {
    while (this.queued && !this.stopped) {
      this.queued = false;
      await Promise.all(this.loaded.map((p) => p.dispose()));
      this.loaded = [];
      this.update({ plugins: [], pending: [] });
      this.controller = new AbortController();
      const r = await loadAllPlugins(this.opts, this.controller.signal).catch(
        (e) => {
          console.error(e);
          return { plugins: [], pending: [] };
        },
      );
      if (this.stopped || this.controller.signal.aborted) {
        await Promise.all(r.plugins.map((p) => p.dispose()));
        continue;
      }
      this.loaded = r.plugins;
      this.update(r);
    }
  }
  async poll() {
    if (this.stopped || this.running) return;
    const infos = await api.pluginList();
    const f = JSON.stringify(
      infos.map((p) => [
        p.id,
        p.enabled,
        p.fingerprint,
        p.revision,
        p.error,
        p.pending_approval,
      ]),
    );
    if (f !== this.fingerprint) {
      this.fingerprint = f;
      await this.reload();
    }
  }
  stop() {
    this.stopped = true;
    this.queued = false;
    this.controller.abort();
    return Promise.all(this.loaded.map((p) => p.dispose())).then(() => {});
  }
}
