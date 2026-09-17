import type { PluginManifest } from "../plugins/types";

const GROUPS = [
  ["core", "核心数据"],
  ["ui", "界面扩展"],
  ["network", "网络访问"],
  ["events", "事件订阅"],
  ["cron", "定时任务"],
] as const;

export default function PluginPermissions({
  manifest,
}: {
  manifest?: PluginManifest;
}) {
  return (
    <div className="space-y-1.5 text-xs">
      {GROUPS.map(([key, label]) => {
        const items = manifest?.permissions?.[key] ?? [];
        return (
          <div key={key} className="flex items-start gap-3">
            <span className="w-16 shrink-0 text-ink3">{label}</span>
            <span
              className={`min-w-0 break-words ${items.length ? "text-warn" : "text-ink3"}`}
            >
              {items.join("、") || "无"}
            </span>
          </div>
        );
      })}
      <div className="flex items-start gap-3">
        <span className="w-16 shrink-0 text-ink3">本机数据</span>
        <span className="text-ink2">
          KV 配额：{manifest?.permissions?.storage_quota_bytes ?? 0} 字节
        </span>
      </div>
    </div>
  );
}
