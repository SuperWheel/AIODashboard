import type { PluginInfo, PluginManifest } from "../plugins/types";

const PERM_GROUPS: { key: keyof NonNullable<PluginManifest["permissions"]>; label: string }[] = [
  { key: "network", label: "网络访问" },
  { key: "events", label: "事件订阅" },
  { key: "cron", label: "定时任务" },
];

/** 首次发现插件时的权限确认弹窗（「安装」时刻）。 */
export default function PluginApprovalModal({
  plugins,
  manifests,
  onApprove,
  onDismiss,
}: {
  plugins: PluginInfo[];
  manifests: Record<string, PluginManifest>;
  onApprove: (p: PluginInfo) => void;
  onDismiss: (p: PluginInfo) => void;
}) {
  const p = plugins[0];
  if (!p) return null;
  const m = manifests[p.id];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="w-full max-w-md rounded-2xl border border-white/10 bg-[#161a22] p-6 shadow-2xl">
        <div className="text-sm font-semibold text-white">发现新插件</div>
        <div className="mt-3 rounded-xl border border-white/10 bg-[#12151c] p-4">
          <div className="text-sm font-medium text-slate-100">{m?.name ?? p.name}</div>
          <div className="mt-0.5 text-[11px] text-slate-500">
            {p.id} · v{m?.version ?? p.version}
          </div>
          {m?.description && <div className="mt-2 text-xs text-slate-400">{m.description}</div>}

          <div className="mt-3 text-[11px] font-medium text-slate-500">请求的权限</div>
          <div className="mt-1.5 space-y-1">
            {PERM_GROUPS.map(({ key, label }) => {
              const items = (m?.permissions?.[key] as string[] | undefined) ?? [];
              return (
                <div key={key} className="flex items-start gap-2 text-xs">
                  <span className="w-16 shrink-0 text-slate-500">{label}</span>
                  {items.length === 0 ? (
                    <span className="text-slate-600">无</span>
                  ) : (
                    <span className="text-amber-300">{items.join("、")}</span>
                  )}
                </div>
              );
            })}
            <div className="flex items-start gap-2 text-xs">
              <span className="w-16 shrink-0 text-slate-500">本机数据</span>
              <span className="text-slate-400">读写任务/笔记等核心数据（全部记入审计日志）</span>
            </div>
          </div>
        </div>

        <p className="mt-3 text-[11px] leading-relaxed text-slate-500">
          插件在本面板内运行并拥有其声明的权限。只启用你信任来源的插件；之后可在「插件」页随时停用。
        </p>

        <div className="mt-4 flex justify-end gap-2">
          <button
            onClick={() => onDismiss(p)}
            className="rounded-lg bg-white/5 px-4 py-2 text-xs text-slate-300 hover:bg-white/10"
          >
            暂不启用
          </button>
          <button
            onClick={() => onApprove(p)}
            className="rounded-lg bg-emerald-500/90 px-4 py-2 text-xs font-medium text-[#0f1115] hover:bg-emerald-400"
          >
            启用插件
          </button>
        </div>
      </div>
    </div>
  );
}
