import { useEffect, useState } from "react";
import { api } from "../api";
import type { PluginInfo, PluginManifest } from "../plugins/types";
import { Badge, Card, Empty, SectionTitle } from "./ui";

const PERM_LABELS: { key: keyof NonNullable<PluginManifest["permissions"]>; label: string }[] = [
  { key: "network", label: "网络" },
  { key: "events", label: "事件" },
  { key: "cron", label: "定时" },
];

export default function PluginsView({
  refreshKey,
  onChanged,
}: {
  refreshKey: number;
  onChanged: () => void;
}) {
  const [plugins, setPlugins] = useState<PluginInfo[] | null>(null);
  const [manifests, setManifests] = useState<Record<string, PluginManifest>>({});
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    api
      .pluginList()
      .then(async (list) => {
        if (!alive) return;
        setPlugins(list);
        const ms: Record<string, PluginManifest> = {};
        for (const p of list) {
          if (p.error) continue;
          try {
            ms[p.id] = await api.pluginReadManifest(p.id);
          } catch {
            // manifest 读不到时仅显示列表信息
          }
        }
        if (alive) setManifests(ms);
      })
      .catch(console.error);
    return () => {
      alive = false;
    };
  }, [refreshKey]);

  const toggle = async (p: PluginInfo) => {
    setBusy(p.id);
    try {
      await api.pluginSetEnabled(p.id, p.enabled !== true);
      onChanged();
    } catch (e) {
      alert(String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div>
      <div className="flex items-baseline justify-between">
        <h1 className="text-xl font-semibold text-white">插件</h1>
        <span className="text-xs text-slate-500">
          插件目录：~/Library/Application Support/AIODashboard/plugins/
        </span>
      </div>

      <SectionTitle>已发现 {plugins?.length ?? "…"} 个</SectionTitle>
      {!plugins ? (
        <Empty text="加载中…" />
      ) : plugins.length === 0 ? (
        <Empty text="插件目录为空 · 参考 examples/plugins/ 里的 echo 示例" />
      ) : (
        <div className="space-y-3">
          {plugins.map((p) => {
            const m = manifests[p.id];
            const perms = m?.permissions ?? {};
            const state =
              p.error != null ? "error" : p.enabled === null ? "new" : p.enabled ? "on" : "off";
            return (
              <Card key={p.id} className="p-4">
                <div className="flex items-center gap-3">
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium text-slate-100">{m?.name ?? p.name}</span>
                      <span className="text-[10px] text-slate-500">v{m?.version ?? p.version}</span>
                      {state === "on" && <Badge tone="green">已启用</Badge>}
                      {state === "off" && <Badge>已停用</Badge>}
                      {state === "new" && <Badge tone="amber">未注册</Badge>}
                      {state === "error" && <Badge tone="red">异常</Badge>}
                    </div>
                    <div className="mt-0.5 truncate text-[11px] text-slate-500">{p.id}</div>
                    {m?.description && (
                      <div className="mt-1 text-xs text-slate-400">{m.description}</div>
                    )}
                    {p.error && <div className="mt-1 text-xs text-rose-300">{p.error}</div>}
                  </div>
                  <button
                    onClick={() => toggle(p)}
                    disabled={busy === p.id || state === "error"}
                    className={`shrink-0 rounded-lg px-3 py-1.5 text-xs font-medium transition-colors disabled:opacity-40 ${
                      state === "on"
                        ? "bg-white/5 text-slate-300 hover:bg-white/10"
                        : "bg-emerald-500/90 text-[#0f1115] hover:bg-emerald-400"
                    }`}
                  >
                    {state === "on" ? "停用" : "启用"}
                  </button>
                </div>

                {(perms.network?.length ?? 0) + (perms.events?.length ?? 0) + (perms.cron?.length ?? 0) >
                  0 && (
                  <div className="mt-3 flex flex-wrap items-center gap-1.5 border-t border-white/5 pt-3">
                    {PERM_LABELS.map(({ key, label }) => {
                      const items = (perms[key] as string[] | undefined) ?? [];
                      if (items.length === 0) return null;
                      return items.map((v) => (
                        <span
                          key={`${key}-${v}`}
                          className="rounded-md bg-white/5 px-1.5 py-0.5 text-[10px] text-slate-400"
                          title={`${label}权限`}
                        >
                          {label} · {v}
                        </span>
                      ));
                    })}
                  </div>
                )}
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}
