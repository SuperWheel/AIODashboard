import { useEffect, useState, type MutableRefObject } from "react";
import { api } from "../api";
import type { PluginInfo, PluginManifest } from "../plugins/types";
import type { ModuleRegistry } from "../plugins/registry";
import type { LoadedPlugin } from "../plugins/loader";
import type {
  PluginApi,
  SettingsDefinition,
  SettingValue,
} from "../../../../packages/plugin-sdk/src/index";
import { Badge, Button, Card, Empty, PageHeader } from "./ui";
import { toastError } from "./DialogHost";
import PluginApprovalModal from "./PluginApprovalModal";
import PluginImportModal from "./PluginImportModal";
function SettingsForm({
  definition,
  pluginApi,
}: {
  definition: SettingsDefinition;
  pluginApi: PluginApi;
}) {
  const [values, setValues] = useState<Record<string, SettingValue> | null>(
    null,
  );
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let alive = true;
    pluginApi.settings
      .get(definition.id)
      .then((v) => {
        if (alive) setValues(v);
      })
      .catch((e) => toastError(String(e)));
    return () => {
      alive = false;
    };
  }, [pluginApi, definition.id]);
  if (!values) return <p className="text-xs text-ink3">设置加载中…</p>;
  return (
    <form
      className="mt-3 space-y-2 border-t border-line pt-3"
      onSubmit={(e) => {
        e.preventDefault();
        setBusy(true);
        void pluginApi.settings
          .set(definition.id, values)
          .catch((e) => toastError(String(e)))
          .finally(() => setBusy(false));
      }}
    >
      <p className="text-sm text-ink">{definition.title}</p>
      {definition.fields.map((f) => (
        <label
          key={f.key}
          className="flex items-center justify-between gap-3 text-xs text-ink2"
        >
          <span>{f.label}</span>
          {f.type === "boolean" ? (
            <input
              type="checkbox"
              checked={values[f.key] === true}
              onChange={(e) =>
                setValues({ ...values, [f.key]: e.target.checked })
              }
            />
          ) : f.type === "select" ? (
            <select
              className="rounded border border-line bg-surface p-1"
              value={String(values[f.key])}
              onChange={(e) =>
                setValues({ ...values, [f.key]: e.target.value })
              }
            >
              {f.options.map((o) => (
                <option key={o}>{o}</option>
              ))}
            </select>
          ) : (
            <input
              className="rounded border border-line bg-surface p-1"
              type={f.type === "number" ? "number" : "text"}
              value={String(values[f.key])}
              min={f.type === "number" ? f.min : undefined}
              max={f.type === "number" ? f.max : undefined}
              onChange={(e) =>
                setValues({
                  ...values,
                  [f.key]:
                    f.type === "number"
                      ? Number(e.target.value)
                      : e.target.value,
                })
              }
            />
          )}
        </label>
      ))}
      <Button disabled={busy} type="submit">
        保存设置
      </Button>
    </form>
  );
}
export default function PluginsView({
  refreshKey,
  onChanged,
  registry,
  pluginsRef,
}: {
  refreshKey: number;
  onChanged: () => void;
  registry: ModuleRegistry;
  pluginsRef: MutableRefObject<LoadedPlugin[]>;
}) {
  const [plugins, setPlugins] = useState<PluginInfo[] | null>(null);
  const [manifests, setManifests] = useState<Record<string, PluginManifest>>(
    {},
  );
  const [selected, setSelected] = useState<PluginInfo | null>(null);
  const [importOpen, setImportOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let alive = true;
    const fetch = async () => {
      const list = await api.pluginList();
      const ms: Record<string, PluginManifest> = {};
      for (const p of list) {
        if (!p.error)
          try {
            ms[p.id] = await api.pluginReadManifest(p.id);
          } catch {
            /* 列表保留错误状态 */
          }
      }
      if (alive) {
        setPlugins(list);
        setManifests(ms);
      }
    };
    void fetch().catch((e) => toastError(String(e)));
    return () => {
      alive = false;
    };
  }, [refreshKey]);
  const reload = () => {
    window.dispatchEvent(new CustomEvent("reload-plugins"));
    onChanged();
  };
  const reviewImported = async (id: string) => {
    setImportOpen(false);
    try {
      const [list, manifest] = await Promise.all([
        api.pluginList(),
        api.pluginReadManifest(id),
      ]);
      const plugin = list.find((p) => p.id === id);
      if (!plugin) throw new Error("已导入的插件暂不可见，请重载列表后重试");
      setPlugins(list);
      setManifests((ms) => ({ ...ms, [id]: manifest }));
      setSelected(plugin);
    } catch (e) {
      toastError(String(e));
    }
  };
  const toggle = async (p: PluginInfo, enabled: boolean) => {
    setBusy(true);
    try {
      await api.pluginSetEnabled(
        p.id,
        enabled,
        enabled ? p.fingerprint : undefined,
      );
      setSelected(null);
      reload();
    } catch (e) {
      toastError(String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div>
      <PageHeader
        title="插件"
        count={plugins?.length}
        desc="仅运行可信来源的本地插件"
        actions={
          <div className="flex gap-2">
            <Button variant="ghost" onClick={reload}>
              重载
            </Button>
            <Button
              variant="ghost"
              disabled={busy}
              onClick={() => {
                setBusy(true);
                void api
                  .pluginDisableAll()
                  .then(reload)
                  .catch((e) => toastError(String(e)))
                  .finally(() => setBusy(false));
              }}
            >
              停用全部
            </Button>
            <Button disabled={busy} onClick={() => setImportOpen(true)}>
              导入插件
            </Button>
          </div>
        }
      />
      <p className="mb-4 text-xs text-warn">
        插件与面板共享运行环境，可能访问或影响整个面板。权限检查不是沙箱。若界面卡死，可用
        CLI 的 plugin safe-mode 停用后重启。
      </p>
      <p className="mb-4 text-xs text-ink3">
        支持导入 ZIP 或本地插件文件夹。需要恢复上一版时，可用 CLI：dashboard plugin
        rollback 插件ID，回滚后需重新审阅并启用。
      </p>
      {!plugins ? (
        <Empty text="加载中…" />
      ) : plugins.length === 0 ? (
        <Card className="flex flex-col items-center gap-3 py-12">
          <p className="text-sm text-ink2">尚未发现插件</p>
          <p className="text-xs text-ink3">导入本地插件，为面板添加功能。</p>
          <Button onClick={() => setImportOpen(true)}>导入第一个插件</Button>
        </Card>
      ) : (
        <div className="space-y-3">
          {plugins.map((p) => {
            const m = manifests[p.id];
            const loaded = pluginsRef.current.find((v) => v.id === p.id);
            return (
              <Card key={p.id} className="p-4">
                <div className="flex items-center gap-3">
                  <div className="min-w-0 flex-1">
                    <div className="flex gap-2 text-sm text-ink">
                      <span>
                        {p.name} · v{p.version}
                      </span>
                      <Badge>
                        {loaded ? "运行中" : p.enabled ? "等待加载" : "已停用"}
                      </Badge>
                      {p.legacy && <Badge tone="amber">需迁移到 v2</Badge>}
                    </div>
                    <p className="text-xs text-ink3">{p.id}</p>
                  </div>
                  {p.enabled ? (
                    <Button
                      disabled={busy}
                      onClick={() => void toggle(p, false)}
                    >
                      停用
                    </Button>
                  ) : (
                    <Button
                      disabled={busy || !!p.error || p.legacy || !m}
                      onClick={() => setSelected(p)}
                    >
                      审阅并启用
                    </Button>
                  )}
                </div>
                <p className="mt-2 text-xs text-ink2">{m?.description}</p>
                <p className="mt-2 break-all text-xs text-ink3">
                  来源：{p.source} · 协议：{p.api_version} · {p.trust_mode}
                  <br />
                  完整性：{p.integrity} · 安装版本：
                  {p.installed_version ?? "本地开发"} · 上一版：
                  {p.previous_version ?? "无"}
                  <br />
                  目录：{p.dir}
                </p>
                <details className="mt-2 text-xs text-ink3">
                  <summary>内容摘要与权限</summary>
                  <p className="break-all">
                    内容 SHA-256：{p.fingerprint || "不可用"}
                    <br />
                    ZIP SHA-256：{p.sha256 ?? "无（本地目录）"}
                  </p>
                  <p>
                    Core：{m?.permissions?.core?.join("、") || "无"}
                    <br />
                    UI：{m?.permissions?.ui?.join("、") || "无"}
                    <br />
                    网络：{m?.permissions?.network?.join("、") || "无"}
                    <br />
                    事件：{m?.permissions?.events?.join("、") || "无"}
                    <br />
                    Cron：{m?.permissions?.cron?.join("、") || "无"}
                    <br />
                    KV 配额：{m?.permissions?.storage_quota_bytes ?? 0} 字节
                  </p>
                </details>
                {p.error && (
                  <p className="mt-2 text-xs text-danger">{p.error}</p>
                )}
                {loaded &&
                  registry.settings
                    .filter((s) => s.owner === p.id)
                    .map((s) => (
                      <SettingsForm
                        key={`${p.id}:${s.definition.id}`}
                        definition={s.definition}
                        pluginApi={loaded.api}
                      />
                    ))}
              </Card>
            );
          })}
        </div>
      )}
      {selected && (
        <PluginApprovalModal
          busy={busy}
          plugins={[selected]}
          manifests={manifests}
          onApprove={(p) => {
            if (!busy) void toggle(p, true);
          }}
          onDismiss={() => setSelected(null)}
        />
      )}
      {importOpen && (
        <PluginImportModal
          onClose={() => setImportOpen(false)}
          onInstalled={reload}
          onReview={(id) => void reviewImported(id)}
        />
      )}
    </div>
  );
}
