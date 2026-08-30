import { useEffect, useState } from "react";
import { api } from "../api";
import type { PluginInfo, PluginManifest } from "../plugins/types";
import { Badge, Button, Card, Empty, PageHeader, SectionTitle } from "./ui";
import { toastError } from "./DialogHost";

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
      // 启停立即生效：通知宿主 dispose 全部插件并按最新启停状态重新加载
      window.dispatchEvent(new CustomEvent("reload-plugins"));
      onChanged();
    } catch (e) {
      toastError(String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div>
      <PageHeader
        title="插件"
        count={plugins?.length}
        desc="插件目录：~/Library/Application Support/AIODashboard/plugins/"
        actions={
          <Button
            variant="ghost"
            title="停用全部插件后重新扫描加载（开发热更新入口）"
            onClick={() => window.dispatchEvent(new CustomEvent("reload-plugins"))}
          >
            重载
          </Button>
        }
      />

      <SectionTitle>已发现插件</SectionTitle>
      {!plugins ? (
        <Empty text="加载中…" glyph="⚙" />
      ) : plugins.length === 0 ? (
        <Empty text="插件目录为空 · 参考 examples/plugins/ 里的 echo 示例" glyph="⚙" />
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
                      <span className="text-sm font-medium text-ink">{m?.name ?? p.name}</span>
                      <span className="text-[10px] text-ink3">v{m?.version ?? p.version}</span>
                      {state === "on" && <Badge tone="green">已启用</Badge>}
                      {state === "off" && <Badge>已停用</Badge>}
                      {state === "new" && <Badge tone="amber">未注册</Badge>}
                      {state === "error" && <Badge tone="red">异常</Badge>}
                    </div>
                    <div className="mt-0.5 truncate text-[11px] text-ink3">{p.id}</div>
                    {m?.description && (
                      <div className="mt-1 text-xs text-ink2">{m.description}</div>
                    )}
                    {p.error && <div className="mt-1 text-xs text-danger">{p.error}</div>}
                  </div>
                  <Button
                    variant={state === "on" ? "ghost" : "primary"}
                    onClick={() => toggle(p)}
                    disabled={busy === p.id || state === "error"}
                    className="shrink-0"
                  >
                    {state === "on" ? "停用" : "启用"}
                  </Button>
                </div>

                {(perms.network?.length ?? 0) + (perms.events?.length ?? 0) + (perms.cron?.length ?? 0) >
                  0 && (
                  <div className="mt-3 flex flex-wrap items-center gap-1.5 border-t border-line/60 pt-3">
                    {PERM_LABELS.map(({ key, label }) => {
                      const items = (perms[key] as string[] | undefined) ?? [];
                      if (items.length === 0) return null;
                      return items.map((v) => (
                        <span
                          key={`${key}-${v}`}
                          className="rounded-md bg-hover px-1.5 py-0.5 text-[10px] text-ink2"
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
