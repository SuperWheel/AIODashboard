import { useEffect, useState } from "react";

/**
 * 应用内对话框/提示总线：Tauri（WKWebView）不支持 window.confirm / window.alert，
 * 原生调用会静默失败——确认操作一律走 confirmDialog，错误提示一律走 toastError。
 * 宿主 <DialogHost /> 挂在 App 根部一次。
 */

export function confirmDialog(title: string, desc?: string): Promise<boolean> {
  return new Promise((resolve) => {
    window.dispatchEvent(
      new CustomEvent("app-confirm", { detail: { title, desc, resolve } }),
    );
  });
}

export function toastError(msg: string) {
  window.dispatchEvent(new CustomEvent("app-error", { detail: msg }));
}

interface ConfirmState {
  title: string;
  desc?: string;
  resolve: (v: boolean) => void;
}

interface Toast {
  id: number;
  msg: string;
}

export default function DialogHost() {
  const [confirmState, setConfirmState] = useState<ConfirmState | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);

  useEffect(() => {
    const onConfirm = (e: Event) => {
      setConfirmState((e as CustomEvent<ConfirmState>).detail);
    };
    const onError = (e: Event) => {
      const id = Date.now() + Math.random();
      const msg = String((e as CustomEvent<unknown>).detail);
      setToasts((list) => [...list.slice(-2), { id, msg }]);
      window.setTimeout(() => {
        setToasts((list) => list.filter((t) => t.id !== id));
      }, 5000);
    };
    window.addEventListener("app-confirm", onConfirm);
    window.addEventListener("app-error", onError);
    return () => {
      window.removeEventListener("app-confirm", onConfirm);
      window.removeEventListener("app-error", onError);
    };
  }, []);

  const settle = (v: boolean) => {
    confirmState?.resolve(v);
    setConfirmState(null);
  };

  return (
    <>
      {confirmState && (
        <div
          className="fixed inset-0 z-[60] flex items-center justify-center bg-black/60"
          onClick={() => settle(false)}
        >
          <div
            className="w-full max-w-sm rounded-2xl border border-line bg-surface p-5 shadow-2xl"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="text-sm font-semibold text-ink">{confirmState.title}</div>
            {confirmState.desc && (
              <div className="mt-1.5 text-xs leading-relaxed text-ink2">
                {confirmState.desc}
              </div>
            )}
            <div className="mt-4 flex justify-end gap-2">
              <button
                onClick={() => settle(false)}
                className="rounded-lg bg-hover px-4 py-2 text-xs text-ink2 hover:text-ink"
              >
                取消
              </button>
              <button
                onClick={() => settle(true)}
                className="rounded-lg bg-danger px-4 py-2 text-xs font-medium text-onaccent hover:bg-danger/90"
              >
                确认
              </button>
            </div>
          </div>
        </div>
      )}

      {toasts.length > 0 && (
        <div className="pointer-events-none fixed bottom-5 right-5 z-[70] flex w-80 flex-col gap-2">
          {toasts.map((t) => (
            <div
              key={t.id}
              className="pointer-events-auto rounded-xl border border-danger/30 bg-surface px-4 py-3 text-xs leading-relaxed text-danger shadow-lg"
            >
              {t.msg}
            </div>
          ))}
        </div>
      )}
    </>
  );
}
