import { useState } from "react";
import { api } from "../api";
import { usePolling } from "../hooks";
import type { ProjectWithStats } from "../types";
import { Badge, Card, Empty, SectionTitle } from "./ui";

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
      <h1 className="text-xl font-semibold text-white">项目</h1>

      <SectionTitle>新建项目</SectionTitle>
      <div className="flex items-center gap-2 rounded-xl border border-white/10 bg-[#161a22] px-3 py-2">
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && add()}
          placeholder="项目名称…"
          className="w-48 bg-transparent text-sm outline-none placeholder:text-slate-600"
        />
        <input
          value={desc}
          onChange={(e) => setDesc(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && add()}
          placeholder="描述（可选）"
          className="flex-1 bg-transparent text-sm outline-none placeholder:text-slate-600"
        />
        <button
          onClick={add}
          disabled={!name.trim() || busy}
          className="rounded-lg bg-emerald-500/90 px-3 py-1 text-xs font-medium text-[#0f1115] hover:bg-emerald-400 disabled:opacity-40"
        >
          创建
        </button>
      </div>

      <SectionTitle>全部项目</SectionTitle>
      {projects.length === 0 ? (
        <Empty text="还没有项目，先创建一个吧" />
      ) : (
        <div className="grid grid-cols-2 gap-3">
          {projects.map((p) => (
            <Card key={p.id} className="p-4">
              <div className="flex items-start justify-between gap-2">
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="truncate font-medium text-white">{p.name}</span>
                    {p.status === "archived" && <Badge tone="slate">已归档</Badge>}
                  </div>
                  {p.description && (
                    <p className="mt-1 line-clamp-2 text-xs text-slate-500">{p.description}</p>
                  )}
                  <div className="mt-2 text-xs text-slate-400">进行中任务 {p.open_tasks}</div>
                </div>
                <div className="flex shrink-0 flex-col gap-1.5">
                  {p.status === "active" && (
                    <button
                      onClick={() => act(() => api.archiveProject(p.id))}
                      className="rounded-md border border-white/10 px-2 py-0.5 text-xs text-slate-400 hover:text-slate-200"
                    >
                      归档
                    </button>
                  )}
                  <button
                    onClick={() => confirm(`删除项目「${p.name}」？其下任务将移出项目。`) && act(() => api.deleteProject(p.id))}
                    className="rounded-md border border-white/10 px-2 py-0.5 text-xs text-slate-400 hover:text-rose-400"
                  >
                    删除
                  </button>
                </div>
              </div>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
