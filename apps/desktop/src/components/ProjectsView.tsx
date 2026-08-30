import { useState } from "react";
import { api } from "../api";
import { usePolling } from "../hooks";
import type { ProjectWithStats } from "../types";
import { Badge, Button, Card, Empty, PageHeader, SectionTitle } from "./ui";
import { confirmDialog, toastError } from "./DialogHost";

export default function ProjectsView({
  refreshKey,
  onChanged,
}: {
  refreshKey: number;
  onChanged: () => void;
}) {
  const [projects, setProjects] = useState<ProjectWithStats[]>([]);
  const [name, setName] = useState("");
  const [desc, setDesc] = useState("");
  const [busy, setBusy] = useState(false);
  // 内联重命名：editingId 非空时该卡片切换为编辑态
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [editDesc, setEditDesc] = useState("");

  const load = () => api.listProjects().then(setProjects).catch(console.error);
  usePolling(load, 6000, [refreshKey]);

  const add = async () => {
    const n = name.trim();
    if (!n || busy) return;
    setBusy(true);
    try {
      await api.createProject(n, desc.trim());
      setName("");
      setDesc("");
      onChanged();
    } catch (e) {
      toastError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const act = async (fn: () => Promise<unknown>) => {
    try {
      await fn();
      onChanged();
    } catch (e) {
      toastError(String(e));
    }
  };

  const startEdit = (p: ProjectWithStats) => {
    setEditingId(p.id);
    setEditName(p.name);
    setEditDesc(p.description);
  };

  const saveEdit = async () => {
    if (!editingId || !editName.trim()) return;
    const id = editingId;
    setEditingId(null);
    await act(() => api.updateProject(id, editName.trim(), editDesc.trim()));
  };

  return (
    <div>
      <PageHeader title="项目" count={projects.length} desc="相关任务的分组；归档后不再出现在 Today" />

      <SectionTitle>新建项目</SectionTitle>
      <div className="flex items-center gap-2 rounded-xl border border-line bg-surface px-3 py-2">
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && add()}
          placeholder="项目名称…"
          className="w-48 bg-transparent text-sm outline-none placeholder:text-ink3"
        />
        <input
          value={desc}
          onChange={(e) => setDesc(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && add()}
          placeholder="描述（可选）"
          className="flex-1 bg-transparent text-sm outline-none placeholder:text-ink3"
        />
        <Button onClick={add} disabled={!name.trim() || busy}>
          创建
        </Button>
      </div>

      <SectionTitle>全部项目</SectionTitle>
      {projects.length === 0 ? (
        <Empty text="还没有项目，先创建一个吧" glyph="▤" />
      ) : (
        <div className="grid grid-cols-2 gap-3">
          {projects.map((p) => (
            <Card key={p.id} className="p-4">
              {editingId === p.id ? (
                <div className="flex flex-col gap-2">
                  <input
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    onKeyDown={(e) =>
                      e.key === "Enter" && !e.nativeEvent.isComposing && saveEdit()
                    }
                    placeholder="项目名称"
                    autoFocus
                    className="rounded-lg border border-line bg-surface2 px-2.5 py-1.5 text-sm outline-none placeholder:text-ink3 focus:border-accent/50"
                  />
                  <input
                    value={editDesc}
                    onChange={(e) => setEditDesc(e.target.value)}
                    onKeyDown={(e) =>
                      e.key === "Enter" && !e.nativeEvent.isComposing && saveEdit()
                    }
                    placeholder="描述（可选）"
                    className="rounded-lg border border-line bg-surface2 px-2.5 py-1.5 text-xs outline-none placeholder:text-ink3 focus:border-accent/50"
                  />
                  <div className="flex justify-end gap-2">
                    <Button variant="ghost" onClick={() => setEditingId(null)}>
                      取消
                    </Button>
                    <Button onClick={saveEdit} disabled={!editName.trim()}>
                      保存
                    </Button>
                  </div>
                </div>
              ) : (
                <div className="flex items-start justify-between gap-2">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="truncate font-medium text-ink">{p.name}</span>
                      {p.status === "archived" && <Badge tone="slate">已归档</Badge>}
                    </div>
                    {p.description && (
                      <p className="mt-1 line-clamp-2 text-xs text-ink2">{p.description}</p>
                    )}
                    <div className="mt-2 text-xs text-ink2">进行中任务 {p.open_tasks}</div>
                  </div>
                  <div className="flex shrink-0 flex-col gap-1.5">
                    <button
                      onClick={() => startEdit(p)}
                      className="rounded-md border border-line px-2 py-0.5 text-xs text-ink2 hover:text-ink"
                    >
                      编辑
                    </button>
                    {p.status === "active" && (
                      <button
                        onClick={() => act(() => api.archiveProject(p.id))}
                        className="rounded-md border border-line px-2 py-0.5 text-xs text-ink2 hover:text-ink"
                      >
                        归档
                      </button>
                    )}
                    <button
                      onClick={async () =>
                        (await confirmDialog(
                          `删除项目「${p.name}」？`,
                          "其下任务将移出项目（不会被删除）",
                        )) && act(() => api.deleteProject(p.id))
                      }
                      className="rounded-md border border-line px-2 py-0.5 text-xs text-ink2 hover:text-danger"
                    >
                      删除
                    </button>
                  </div>
                </div>
              )}
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
