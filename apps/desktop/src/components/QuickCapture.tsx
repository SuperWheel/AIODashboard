import { useState } from "react";
import { api } from "../api";
import { toastError } from "./DialogHost";
import { inputCls } from "./ui";

type Target = "today" | "inbox";

const TARGET_META: Record<Target, { label: string; hint: string }> = {
  today: { label: "今天任务", hint: "⏎ 创建为一次性打卡任务（完成即归档）" },
  inbox: { label: "收件箱", hint: "⏎ 收集到收件箱，之后再整理" },
};

/**
 * Today 页 Bento 的快速捕捉卡：一条输入，两个去向（今天到期任务 / 收件箱）。
 * Enter 即提交，不打断当前浏览。
 */
export default function QuickCapture({
  onChanged,
  className = "",
}: {
  onChanged: () => void;
  className?: string;
}) {
  const [text, setText] = useState("");
  const [target, setTarget] = useState<Target>("today");
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    const t = text.trim();
    if (!t || busy) return;
    setBusy(true);
    try {
      if (target === "today") {
        // 快速捕捉的任务默认一次性：完成即归档，不污染长期打卡统计
        await api.createTask({ title: t, recurrence: { kind: "once" } });
      } else {
        await api.addInboxItem(t);
      }
      setText("");
      onChanged();
    } catch (e) {
      toastError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className={`flex flex-col ${className}`}>
      <div className="flex items-center justify-between">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-ink2">快速捕捉</h2>
        <div className="flex rounded-lg bg-hover p-0.5">
          {(Object.keys(TARGET_META) as Target[]).map((k) => (
            <button
              key={k}
              onClick={() => setTarget(k)}
              className={`rounded-md px-2 py-0.5 text-[11px] transition-colors ${
                target === k ? "bg-surface font-medium text-ink shadow-sm" : "text-ink2 hover:text-ink"
              }`}
            >
              {TARGET_META[k].label}
            </button>
          ))}
        </div>
      </div>
      <input
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && submit()}
        placeholder="一个想法、一条待办、一个链接…"
        className={`${inputCls} mt-3 placeholder:text-ink3`}
      />
      <div className="mt-2 text-[11px] text-ink3">{TARGET_META[target].hint}</div>
    </div>
  );
}
