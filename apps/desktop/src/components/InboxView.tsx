import { useState } from "react";
import { api } from "../api";
import { usePolling, fmtDateTime } from "../hooks";
import type { InboxItem } from "../types";
import { Badge, Card, Empty, SectionTitle } from "./ui";

export default function InboxView({
  refreshKey,
  onChanged,
}: {
  refreshKey: number;
  onChanged: () => void;
}) {
  const [items, setItems] = useState<InboxItem[]>([]);
  const [content, setContent] = useState("");
  const [showProcessed, setShowProcessed] = useState(false);
  const [busy, setBusy] = useState(false);

  const load = () =>
    api.listInbox(showProcessed).then(setItems).catch(console.error);
  usePolling(load, 4000, [refreshKey, showProcessed]);

  const add = async () => {
    const c = content.trim();
    if (!c || busy) return;
    setBusy(true);
    try {
      await api.addInboxItem(c);
      setContent("");
      onChanged();
    } catch (e) {
      alert(String(e));
    } finally {
      setBusy(false);
    }
  };

  const act = async (fn: () => Promise<unknown>) => {
    try {
      await fn();
      onChanged();
    } catch (e) {
      alert(String(e));
    }
  };

  return (
    <div>
      <h1 className="text-xl font-semibold text-ink">收件箱</h1>
      <p className="mt-1 text-xs text-ink2">
        任何暂时不知道放哪里的想法、待办、网址、文本，先丢进这里，之后再整理成任务 / 笔记。
      </p>

      <SectionTitle>快速收集</SectionTitle>
      <div className="flex items-center gap-2 rounded-xl border border-line bg-surface px-3 py-2">
        <input
          value={content}
          onChange={(e) => setContent(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && add()}
          placeholder="一个想法 / 一条待办 / 一个链接…"
          className="flex-1 bg-transparent text-sm outline-none placeholder:text-ink3"
        />
        <button
          onClick={add}
          disabled={!content.trim() || busy}
          className="rounded-lg bg-violet px-3 py-1 text-xs font-medium text-onaccent hover:bg-violet/90 disabled:opacity-40"
        >
          收集
        </button>
      </div>

      <SectionTitle
        right={
          <label className="flex cursor-pointer items-center gap-1.5 text-xs text-ink3">
            <input
              type="checkbox"
              checked={showProcessed}
              onChange={(e) => setShowProcessed(e.target.checked)}
              className="accent-accent"
            />
            显示已处理
          </label>
        }
      >
        条目（{items.length}）
      </SectionTitle>

      {items.length === 0 ? (
        <Empty text="收件箱是空的" />
      ) : (
        <Card>
          {items.map((i) => (
            <div
              key={i.id}
              className="group flex items-center gap-3 border-b border-line/60 px-3 py-2.5 last:border-b-0 hover:bg-hover"
            >
              <span
                className={`flex-1 truncate text-sm ${i.status === "processed" ? "text-ink3 line-through" : ""}`}
                title={i.content}
              >
                {i.content}
              </span>
              {i.status === "open" ? (
                <>
                  <Badge tone="slate">{fmtDateTime(i.created_at)}</Badge>
                  <button
                    onClick={() => act(() => api.inboxToTask(i.id))}
                    className="invisible rounded-md border border-line px-2 py-0.5 text-xs text-ink2 hover:border-accent/40 hover:text-accent group-hover:visible"
                  >
                    转任务
                  </button>
                  <button
                    onClick={() => act(() => api.inboxToNote(i.id))}
                    className="invisible rounded-md border border-line px-2 py-0.5 text-xs text-ink2 hover:border-info/40 hover:text-info group-hover:visible"
                  >
                    转笔记
                  </button>
                </>
              ) : (
                <Badge tone="green">已处理 · 来自 {i.source}</Badge>
              )}
              <button
                onClick={() => confirm("删除该条目？") && act(() => api.deleteInboxItem(i.id))}
                className="invisible shrink-0 text-ink3 transition-colors hover:text-danger group-hover:visible"
              >
                ×
              </button>
            </div>
          ))}
        </Card>
      )}
    </div>
  );
}
