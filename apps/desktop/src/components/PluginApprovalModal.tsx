import type { PluginInfo, PluginManifest } from "../plugins/types";
import PluginPermissions from "./PluginPermissions";

/** 首次发现或导入后的权限审阅；启用绑定本次内容摘要。 */
export default function PluginApprovalModal({
  plugins,
  manifests,
  onApprove,
  onDismiss,
  busy = false,
}: {
  plugins: PluginInfo[];
  manifests: Record<string, PluginManifest>;
  onApprove: (p: PluginInfo) => void;
  onDismiss: (p: PluginInfo) => void;
  busy?: boolean;
}) {
  const p = plugins[0];
  if (!p) return null;
  const m = manifests[p.id];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div role="dialog" aria-modal="true" aria-label="审阅并启用插件" className="max-h-[90vh] w-full max-w-md overflow-y-auto rounded-2xl border border-line bg-surface p-6 shadow-2xl">
        <div className="text-sm font-semibold text-ink">审阅并启用插件</div>
        <div className="mt-3 rounded-xl border border-line bg-surface2 p-4">
          <div className="text-sm font-medium text-ink">
            {m?.name ?? p.name}
          </div>
          <div className="mt-0.5 text-[11px] text-ink3">
            {p.id} · v{m?.version ?? p.version}
          </div>
          {m?.description && (
            <div className="mt-2 text-xs text-ink2">{m.description}</div>
          )}

          <div className="mt-3 text-[11px] font-medium text-ink3">
            请求的权限
          </div>
          <div className="mt-1.5">
            <PluginPermissions manifest={m} />
          </div>
        </div>

        <p className="mt-3 text-[11px] leading-relaxed text-ink3">
          插件与面板共享
          WebView，可以影响整个面板。权限检查不是沙箱；仅启用你信任的代码。来源：
          {p.source}。
        </p>

        <div className="mt-4 flex justify-end gap-2">
          <button
            disabled={busy}
            onClick={() => onDismiss(p)}
            className="rounded-lg bg-hover px-4 py-2 text-xs text-ink2 hover:text-ink"
          >
            暂不启用
          </button>
          <button
            disabled={busy || !m || !!p.error || p.legacy}
            onClick={() => onApprove(p)}
            className="rounded-lg bg-accent px-4 py-2 text-xs font-medium text-onaccent hover:bg-accent/90"
          >
            启用插件
          </button>
        </div>
      </div>
    </div>
  );
}
