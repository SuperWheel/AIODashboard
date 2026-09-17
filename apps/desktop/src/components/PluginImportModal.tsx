import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import {
  CheckCircle2,
  FileArchive,
  FolderOpen,
  LoaderCircle,
  Upload,
  X,
} from "lucide-react";
import { api } from "../api";
import { PluginImportFlow } from "../plugins/import";
import { Badge, Button } from "./ui";
import PluginPermissions from "./PluginPermissions";

export default function PluginImportModal({
  onClose,
  onInstalled,
  onReview,
}: {
  onClose: () => void;
  onInstalled: () => void;
  onReview: (id: string) => void;
}) {
  const [flow] = useState(() => new PluginImportFlow(api, onInstalled));
  const state = useSyncExternalStore(flow.subscribe, flow.getSnapshot);
  const dialog = useRef<HTMLDialogElement>(null);
  const { phase, source, preview, result, error } = state;
  const installing = phase === "installing";
  const selecting = phase === "picking";
  const success = phase === "success";

  useEffect(() => {
    const element = dialog.current;
    element?.showModal();
    return () => {
      element?.close();
      flow.cancel();
    };
  }, [flow]);

  const close = () => {
    if (flow.cancel()) onClose();
  };

  return (
    <dialog
      ref={dialog}
      aria-labelledby="plugin-import-title"
      onCancel={(e) => {
        e.preventDefault();
        if (!selecting) close();
      }}
      className="m-auto w-[calc(100%-2rem)] max-w-xl overflow-hidden rounded-2xl border border-line bg-surface p-0 text-ink shadow-2xl backdrop:bg-black/60"
    >
      <div className="flex items-center gap-3 border-b border-line px-6 py-4">
        <div className="rounded-xl bg-accent/10 p-2.5 text-accent">
          <Upload size={20} />
        </div>
        <div className="flex-1">
          <h2 id="plugin-import-title" className="text-base font-semibold">
            导入插件
          </h2>
          <p className="mt-0.5 text-xs text-ink3">从本机添加你信任的插件</p>
        </div>
        <button
          type="button"
          aria-label="关闭导入"
          disabled={installing || selecting}
          onClick={close}
          className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg border border-line text-ink2 hover:bg-hover disabled:opacity-40"
        >
          <X size={16} />
        </button>
      </div>

      <div className="max-h-[65vh] space-y-4 overflow-y-auto px-6 py-5">
        {success && result ? (
          <div className="rounded-xl border border-accent/25 bg-accent/5 p-5">
            <CheckCircle2 className="mb-3 text-accent" size={28} />
            <p className="font-medium">已导入，尚未启用</p>
            <p className="mt-2 text-sm text-ink2">
              {preview?.manifest.name ?? result.id} · v{result.version}
            </p>
            <p className="mt-1 break-all text-xs text-ink3">{result.id}</p>
            <p className="mt-4 text-xs leading-relaxed text-ink2">
              审阅权限后即可启用，也可以稍后在插件列表中启用。
            </p>
          </div>
        ) : (
          <>
            <div className="rounded-xl border border-dashed border-line bg-surface2 p-4">
              <div className="flex gap-2">
                <Button
                  variant="ghost"
                  disabled={selecting || installing}
                  onClick={() => void flow.choose("zip")}
                >
                  <FileArchive size={14} className="mr-1.5" />
                  选择 ZIP
                </Button>
                <Button
                  variant="ghost"
                  disabled={selecting || installing}
                  onClick={() => void flow.choose("directory")}
                >
                  <FolderOpen size={14} className="mr-1.5" />
                  选择文件夹
                </Button>
              </div>
              <p className="mt-3 text-xs leading-relaxed text-ink3">
                ZIP 应包含一个插件根目录；文件夹应包含 manifest.json
                和入口文件，文件夹名需与插件 ID 一致。
              </p>
            </div>
            {source && (
              <div className="text-xs text-ink3">
                <p className="mb-1">已选择</p>
                <p className="break-all text-ink2">{source}</p>
              </div>
            )}
            {(phase === "picking" || phase === "checking" || installing) && (
              <p
                role="status"
                className="flex items-center gap-2 text-sm text-ink2"
              >
                <LoaderCircle size={16} className="animate-spin" />
                {selecting
                  ? "请在系统窗口中选择…"
                  : installing
                    ? "正在导入，请稍候…"
                    : "正在检查插件…"}
              </p>
            )}
            {error && (
              <p
                role="alert"
                className="rounded-xl border border-danger/25 bg-danger/5 p-3 text-sm text-danger"
              >
                {error}
              </p>
            )}
            {preview && (
              <div className="rounded-xl border border-line bg-surface2 p-4">
                <div className="flex items-center gap-2">
                  <p className="min-w-0 break-words text-sm font-semibold">
                    {preview.manifest.name}
                  </p>
                  <Badge tone={preview.replacing ? "amber" : "green"}>
                    {preview.replacing ? "替换已有插件" : "新插件"}
                  </Badge>
                </div>
                <p className="mt-1 break-all text-xs text-ink3">
                  {preview.manifest.id}
                </p>
                <p className="mt-3 text-sm text-ink">
                  {preview.replacing
                    ? `v${preview.current_version ?? "未知"} → v${preview.manifest.version}`
                    : `v${preview.manifest.version}`}
                </p>
                {preview.manifest.description && (
                  <p className="mt-2 text-xs leading-relaxed text-ink2">
                    {preview.manifest.description}
                  </p>
                )}
                {preview.replacing && (
                  <p className="mt-3 text-xs leading-relaxed text-warn">
                    将替换此 ID 的现有插件，并保留上一版备份。
                    {preview.current_enabled ? "正在运行的版本会停用，" : ""}
                    导入后需重新审阅并启用。
                  </p>
                )}
                <div className="mt-4 border-t border-line pt-3">
                  <p className="mb-2 text-xs font-medium text-ink2">
                    请求的权限
                  </p>
                  <PluginPermissions manifest={preview.manifest} />
                </div>
              </div>
            )}
            <p className="text-xs leading-relaxed text-ink3">
              插件与面板共享运行环境，权限检查不是沙箱。请仅导入可信来源；导入后不会自动运行。
            </p>
          </>
        )}
      </div>

      <div className="flex justify-end gap-2 border-t border-line px-6 py-4">
        <Button
          variant="ghost"
          disabled={installing || selecting}
          onClick={close}
        >
          {success ? "完成" : "取消"}
        </Button>
        {success && result ? (
          <Button onClick={() => onReview(result.id)}>审阅并启用</Button>
        ) : phase === "error" && source ? (
          <Button onClick={() => void flow.recheck()}>重新检查</Button>
        ) : (
          <Button
            disabled={phase !== "ready"}
            onClick={() => void flow.commit()}
          >
            {installing
              ? "导入中…"
              : preview?.replacing
                ? "确认替换"
                : "确认导入"}
          </Button>
        )}
      </div>
    </dialog>
  );
}
