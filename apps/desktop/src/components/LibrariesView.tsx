import { useState } from "react";
import { api } from "../api";
import { localToday, usePolling } from "../hooks";
import type { LibraryListItem } from "../types";
import LibraryDetailView from "./LibraryDetailView";
import { Button, Card, Empty, PageHeader } from "./ui";
import { confirmDialog, toastError } from "./DialogHost";
import { taskColor, TASK_COLORS } from "../taskVisual";

function dayBadge(it: LibraryListItem): { text: string; cls: string } {
  switch (it.day_info.display_kind) {
    case "day_n":
      return { text: `第 ${it.day_info.day_count} 天`, cls: "text-accent" };
    case "remaining":
      return { text: `还剩 ${it.day_info.day_count} 天`, cls: "text-info" };
    case "today":
      return { text: "就是今天", cls: "text-accent" };
    default:
      return { text: `已逾期 ${it.day_info.day_count} 天`, cls: "text-danger" };
  }
}

/** 重要日编辑器（新建/编辑）。 */
function LibraryEditor({
  library,
  onClose,
  onSaved,
}: {
  library: LibraryListItem | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [title, setTitle] = useState(library?.title ?? "");
  const [kind, setKind] = useState<"anniversary" | "countdown">(library?.kind ?? "countdown");
  const [anchor, setAnchor] = useState(library?.anchor_day ?? localToday());
  const [icon, setIcon] = useState(library?.icon ?? "");
  const [color, setColor] = useState(library?.color_hex ?? TASK_COLORS[1].hex);
  const [note, setNote] = useState(library?.note ?? "");
  const [busy, setBusy] = useState(false);

  const save = async () => {
    const t = title.trim();
    if (!t || busy) return;
    setBusy(true);
    try {
      if (library) {
        await api.updateLibrary(library.id, { title: t, note, icon, color, anchorDay: anchor });
      } else {
        await api.createLibrary({ title: t, kind, anchorDay: anchor, note, icon, color });
      }
      onSaved();
      onClose();
    } catch (e) {
      toastError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const fieldCls =
    "w-full rounded-lg border border-line bg-surface2 px-2.5 py-1.5 text-sm outline-none focus:border-accent/50";
  const labelCls = "mb-1 block text-xs text-ink2";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30" onClick={onClose}>
      <div
        className="w-[420px] max-w-[92vw] rounded-2xl border border-line bg-surface p-5 shadow-lg"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 className="text-sm font-semibold text-ink">{library ? "编辑重要日" : "新建重要日"}</h2>
        <div className="mt-4 grid grid-cols-[72px_1fr] gap-3">
          <div>
            <label className={labelCls}>图标</label>
            <input value={icon} onChange={(e) => setIcon(e.target.value)} maxLength={4} placeholder="📅" className={`${fieldCls} text-center text-lg`} />
          </div>
          <div>
            <label className={labelCls}>标题</label>
            <input value={title} onChange={(e) => setTitle(e.target.value)} autoFocus placeholder="例如：考研倒计时、恋爱纪念日…" className={fieldCls} />
          </div>
        </div>
        <div className="mt-3 grid grid-cols-2 gap-3">
          <div>
            <label className={labelCls}>类型</label>
            <select value={kind} onChange={(e) => setKind(e.target.value as typeof kind)} className={fieldCls} disabled={!!library}>
              <option value="countdown">倒计时日</option>
              <option value="anniversary">纪念日</option>
            </select>
          </div>
          <div>
            <label className={labelCls}>锚点日期</label>
            <input type="date" value={anchor} onChange={(e) => setAnchor(e.target.value)} className={fieldCls} />
          </div>
        </div>
        <div className="mt-3">
          <label className={labelCls}>主题色</label>
          <div className="flex gap-2">
            {TASK_COLORS.map((c) => (
              <button
                key={c.hex}
                title={c.name}
                onClick={() => setColor(c.hex)}
                className="h-7 w-7 rounded-lg transition-transform hover:scale-110"
                style={{
                  background: c.hex,
                  boxShadow: color === c.hex ? `0 0 0 2px var(--surface), 0 0 0 4px ${c.hex}` : undefined,
                }}
              />
            ))}
          </div>
        </div>
        <div className="mt-3">
          <label className={labelCls}>备注</label>
          <input value={note} onChange={(e) => setNote(e.target.value)} className={fieldCls} />
        </div>
        <div className="mt-5 flex justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>取消</Button>
          <Button onClick={save} disabled={!title.trim() || busy}>{library ? "保存" : "创建"}</Button>
        </div>
      </div>
    </div>
  );
}

/** 重要日总览：卡片网格 + 已归档。navParam 以 dlb_ 开头 → 详情。 */
export default function LibrariesView({
  refreshKey,
  onChanged,
  navParam,
  onNav,
}: {
  refreshKey: number;
  onChanged: () => void;
  navParam?: string;
  onNav: (view: string, param?: string) => void;
}) {
  const [items, setItems] = useState<LibraryListItem[]>([]);
  const [showArchived, setShowArchived] = useState(false);
  const [editorOpen, setEditorOpen] = useState(false);
  const [editing, setEditing] = useState<LibraryListItem | null>(null);

  const detailId = navParam?.startsWith("dlb_") ? navParam : null;

  const load = () => api.listLibraries(true).then(setItems).catch(console.error);
  usePolling(load, 4000, [refreshKey]);

  if (detailId) {
    return (
      <LibraryDetailView
        libraryId={detailId}
        refreshKey={refreshKey}
        onChanged={onChanged}
        onBack={() => onNav("libraries")}
        onNav={onNav}
      />
    );
  }

  const active = items.filter((i) => i.status === "active");
  const archived = items.filter((i) => i.status === "archived");
  const shown = showArchived ? archived : active;

  const archive = async (it: LibraryListItem) => {
    if (it.task_count > 0) {
      toastError(`「${it.title}」还有 ${it.task_count} 个直属任务，请在详情页归档（可选择任务去向）`);
      onNav("libraries", it.id);
      return;
    }
    if (!(await confirmDialog("归档重要日", `「${it.title}」将进入已归档列表`))) return;
    try {
      await api.archiveLibrary(it.id, "keep");
      onChanged();
    } catch (e) {
      toastError(String(e));
    }
  };

  return (
    <div>
      <PageHeader
        title="重要日"
        count={active.length}
        desc="用纪念日或倒计时日组织长期任务"
        actions={<Button onClick={() => { setEditing(null); setEditorOpen(true); }}>＋ 新建重要日</Button>}
      />

      <div className="mt-4 flex gap-1">
        <button
          onClick={() => setShowArchived(false)}
          className={`rounded-lg px-2.5 py-1 text-xs transition-colors ${!showArchived ? "bg-accent/10 font-medium text-accent" : "text-ink3 hover:text-ink"}`}
        >
          进行中（{active.length}）
        </button>
        <button
          onClick={() => setShowArchived(true)}
          className={`rounded-lg px-2.5 py-1 text-xs transition-colors ${showArchived ? "bg-accent/10 font-medium text-accent" : "text-ink3 hover:text-ink"}`}
        >
          已归档（{archived.length}）
        </button>
      </div>

      {shown.length === 0 ? (
        <div className="mt-4">
          <Empty
            text={showArchived ? "没有已归档的重要日" : "还没有重要日，建一个倒计时试试看"}
            glyph="📅"
            action={!showArchived ? <Button onClick={() => setEditorOpen(true)}>新建重要日</Button> : undefined}
          />
        </div>
      ) : (
        <div className="mt-4 grid grid-cols-1 gap-4 xl:grid-cols-2">
          {shown.map((it) => {
            const accent = taskColor(it.color_hex);
            const badge = dayBadge(it);
            return (
              <Card
                key={it.id}
                className="relative p-4"
                hoverable
                onClick={() => onNav("libraries", it.id)}
              >
                <div className="flex items-center gap-3">
                  <div
                    className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl text-lg"
                    style={{ background: `color-mix(in srgb, ${accent} 16%, transparent)` }}
                  >
                    {it.icon || "📅"}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm font-semibold text-ink">{it.title}</div>
                    <div className="mt-0.5 text-xs text-ink3">
                      {it.kind === "anniversary" ? "纪念日" : "倒计时日"} · 锚点 {it.anchor_day}
                    </div>
                  </div>
                  <div className="shrink-0 text-right">
                    <div className={`text-lg font-semibold tabular-nums ${badge.cls}`}>{badge.text}</div>
                    <div className="mt-0.5 text-[11px] text-ink3">{it.task_count} 个任务</div>
                  </div>
                </div>
                <div className="absolute right-3 top-3 flex gap-1 opacity-0 transition-opacity [div:hover>&]:opacity-100">
                  {!showArchived ? (
                    <>
                      <button
                        className="rounded-md px-1.5 py-0.5 text-xs text-ink3 hover:bg-hover hover:text-ink"
                        onClick={(e) => { e.stopPropagation(); setEditing(it); setEditorOpen(true); }}
                        title="编辑"
                      >
                        ✎
                      </button>
                      <button
                        className="rounded-md px-1.5 py-0.5 text-xs text-ink3 hover:bg-hover hover:text-danger"
                        onClick={(e) => { e.stopPropagation(); void archive(it); }}
                        title="归档"
                      >
                        🗃
                      </button>
                    </>
                  ) : (
                    <button
                      className="rounded-md px-1.5 py-0.5 text-xs text-ink3 hover:bg-hover hover:text-ink"
                      onClick={(e) => {
                        e.stopPropagation();
                        api.restoreLibrary(it.id).then(onChanged).catch((err) => toastError(String(err)));
                      }}
                      title="恢复"
                    >
                      ↩
                    </button>
                  )}
                </div>
              </Card>
            );
          })}
        </div>
      )}

      {editorOpen && (
        <LibraryEditor library={editing} onClose={() => setEditorOpen(false)} onSaved={onChanged} />
      )}
    </div>
  );
}
