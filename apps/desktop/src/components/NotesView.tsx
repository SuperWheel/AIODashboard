import { useEffect, useState } from "react";
import { api } from "../api";
import { usePolling } from "../hooks";
import type { Note } from "../types";
import { Empty } from "./ui";

export default function NotesView({
  refreshKey,
  onChanged,
}: {
  refreshKey: number;
  onChanged: () => void;
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
      alert(String(e));
    }
  };

  const save = async () => {
    if (!selected) return;
    try {
      await api.updateNote(selected.id, title.trim(), body);
      setDirty(false);
      onChanged();
    } catch (e) {
      alert(String(e));
    }
  };

  const create = async () => {
    try {
      const n = await api.createNote("", "");
      setDirty(false);
      setSelectedId(n.id);
      onChanged();
    } catch (e) {
      alert(String(e));
    }
  };

  const remove = async () => {
    if (!selected || !confirm(`删除笔记「${selected.title || "(无标题)"}」？`)) return;
    try {
      await api.deleteNote(selected.id);
      setSelectedId(null);
      setDirty(false);
      onChanged();
    } catch (e) {
      alert(String(e));
    }
  };

  return (
    <div className="flex h-[calc(100vh-3rem)] flex-col">
      <div className="flex items-center justify-between pb-4">
        <h1 className="text-xl font-semibold text-white">笔记</h1>
        <button
          onClick={create}
          className="rounded-lg bg-emerald-500/90 px-3 py-1.5 text-xs font-medium text-[#0f1115] hover:bg-emerald-400"
        >
          新建笔记
        </button>
      </div>

      <div className="flex min-h-0 flex-1 gap-4">
        {/* 列表 */}
        <div className="w-64 shrink-0 overflow-y-auto rounded-xl border border-white/10 bg-[#12151c]">
          {notes.length === 0 ? (
            <p className="p-4 text-xs text-slate-600">暂无笔记</p>
          ) : (
            notes.map((n) => (
              <button
                key={n.id}
                onClick={() => pick(n)}
                className={`block w-full border-b border-white/5 px-3 py-2.5 text-left transition-colors ${
                  selectedId === n.id ? "bg-emerald-500/10" : "hover:bg-white/[0.03]"
                }`}
              >
                <div className="truncate text-sm text-slate-200">
                  {n.title || "(无标题)"}
                </div>
                <div className="mt-0.5 truncate text-xs text-slate-600">
                  {n.body.split("\n")[0] || "空笔记"}
                </div>
              </button>
            ))
          )}
        </div>

        {/* 编辑区 */}
        <div className="flex min-w-0 flex-1 flex-col rounded-xl border border-white/10 bg-[#161a22]">
          {!selected ? (
            <div className="flex h-full items-center justify-center">
              <Empty text="选择或新建一条笔记" />
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
                className="border-b border-white/10 bg-transparent px-4 py-3 font-medium outline-none placeholder:text-slate-600"
              />
              <textarea
                value={body}
                onChange={(e) => {
                  setBody(e.target.value);
                  setDirty(true);
                }}
                placeholder="正文…（支持多行）"
                className="min-h-0 flex-1 resize-none bg-transparent px-4 py-3 text-sm leading-relaxed outline-none placeholder:text-slate-600"
              />
              <div className="flex items-center justify-between border-t border-white/10 px-4 py-2.5">
                <span className="text-xs text-slate-600">
                  {dirty ? "有未保存修改" : "已同步"}
                </span>
                <div className="flex gap-2">
                  <button
                    onClick={remove}
                    className="rounded-md border border-white/10 px-2.5 py-1 text-xs text-slate-400 hover:text-rose-400"
                  >
                    删除
                  </button>
                  <button
                    onClick={save}
                    disabled={!dirty}
                    className="rounded-md bg-emerald-500/90 px-3 py-1 text-xs font-medium text-[#0f1115] hover:bg-emerald-400 disabled:opacity-40"
                  >
                    保存
                  </button>
                </div>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
