import { useEffect, useState } from "react";
import { api } from "../api";
import { usePolling } from "../hooks";
import type { Note } from "../types";
import { Button, Empty, PageHeader } from "./ui";
import { confirmDialog, toastError } from "./DialogHost";

export default function NotesView({
  refreshKey,
  onChanged,
  navParam,
}: {
  refreshKey: number;
  onChanged: () => void;
  /** 跨视图导航参数：Today 页点击笔记条目时传入笔记 id */
  navParam?: string;
}) {
  const [notes, setNotes] = useState<Note[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [dirty, setDirty] = useState(false);

  const load = async () => {
    const list = await api.listNotes().catch((e) => {
      console.error(e);
      return null;
    });
    if (!list) return;
    setNotes(list);
    setSelectedId((cur) => cur ?? list[0]?.id ?? null);
  };
  usePolling(load, 5000, [refreshKey]);

  const selected = notes.find((n) => n.id === selectedId) ?? null;

  // Today 页「最近笔记」跳转：定位到该条（有未保存修改时不打断用户）
  useEffect(() => {
    if (navParam && !dirty && notes.some((n) => n.id === navParam)) {
      setSelectedId(navParam);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [navParam, notes]);

  useEffect(() => {
    if (selected && !dirty) {
      setTitle(selected.title);
      setBody(selected.body);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedId, refreshKey]);

  const pick = (n: Note) => {
    saveIfDirty();
    setSelectedId(n.id);
    setDirty(false);
  };

  const saveIfDirty = async () => {
    if (!dirty || !selected) return;
    try {
      await api.updateNote(selected.id, title.trim(), body);
      setDirty(false);
      onChanged();
    } catch (e) {
      toastError(String(e));
    }
  };

  const save = async () => {
    if (!selected) return;
    try {
      await api.updateNote(selected.id, title.trim(), body);
      setDirty(false);
      onChanged();
    } catch (e) {
      toastError(String(e));
    }
  };

  const create = async () => {
    try {
      const n = await api.createNote("", "");
      setDirty(false);
      setSelectedId(n.id);
      onChanged();
    } catch (e) {
      toastError(String(e));
    }
  };

  const remove = async () => {
    if (!selected || !(await confirmDialog("删除笔记", `「${selected.title || "(无标题)"}」删除后不可恢复`))) return;
    try {
      await api.deleteNote(selected.id);
      setSelectedId(null);
      setDirty(false);
      onChanged();
    } catch (e) {
      toastError(String(e));
    }
  };

  return (
    <div className="flex h-[calc(100vh-3rem)] flex-col">
      <div className="pb-4">
        <PageHeader
          title="笔记"
          count={notes.length}
          actions={<Button onClick={create}>新建笔记</Button>}
        />
      </div>

      <div className="flex min-h-0 flex-1 gap-4">
        {/* 列表 */}
        <div className="w-64 shrink-0 overflow-y-auto rounded-xl border border-line bg-surface2">
          {notes.length === 0 ? (
            <p className="p-4 text-xs text-ink3">暂无笔记</p>
          ) : (
            notes.map((n) => (
              <button
                key={n.id}
                onClick={() => pick(n)}
                className={`block w-full border-b border-line/60 px-3 py-2.5 text-left transition-colors ${
                  selectedId === n.id ? "bg-accent/10" : "hover:bg-hover"
                }`}
              >
                <div className="truncate text-sm text-ink">
                  {n.title || "(无标题)"}
                </div>
                <div className="mt-0.5 truncate text-xs text-ink3">
                  {n.body.split("\n")[0] || "空笔记"}
                </div>
              </button>
            ))
          )}
        </div>

        {/* 编辑区 */}
        <div className="flex min-w-0 flex-1 flex-col rounded-xl border border-line bg-surface">
          {!selected ? (
            <div className="flex h-full items-center justify-center">
              <Empty text="选择或新建一条笔记" glyph="✎" />
            </div>
          ) : (
            <>
              <input
                value={title}
                onChange={(e) => {
                  setTitle(e.target.value);
                  setDirty(true);
                }}
                placeholder="标题"
                className="border-b border-line bg-transparent px-4 py-3 font-medium outline-none placeholder:text-ink3"
              />
              <textarea
                value={body}
                onChange={(e) => {
                  setBody(e.target.value);
                  setDirty(true);
                }}
                placeholder="正文…（支持多行）"
                className="min-h-0 flex-1 resize-none bg-transparent px-4 py-3 text-sm leading-relaxed outline-none placeholder:text-ink3"
              />
              <div className="flex items-center justify-between border-t border-line px-4 py-2.5">
                <span className="text-xs text-ink3">
                  {dirty ? "有未保存修改" : "已同步"}
                </span>
                <div className="flex gap-2">
                  <Button variant="ghost" onClick={remove} className="hover:text-danger">
                    删除
                  </Button>
                  <Button onClick={save} disabled={!dirty}>
                    保存
                  </Button>
                </div>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
