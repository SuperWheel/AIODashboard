import { useEffect, useState } from "react";
import { usePolling } from "./hooks";
import { api } from "./api";
import type { TodayContext } from "./types";
import Sidebar from "./components/Sidebar";
import TodayView from "./components/TodayView";
import TasksView from "./components/TasksView";
import ProjectsView from "./components/ProjectsView";
import NotesView from "./components/NotesView";
import InboxView from "./components/InboxView";
import SearchPalette from "./components/SearchPalette";

export type ViewName = "today" | "tasks" | "projects" | "notes" | "inbox";

export default function App() {
  const [view, setView] = useState<ViewName>("today");
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [today, setToday] = useState<TodayContext | null>(null);
  // refreshKey 变化时所有视图立即重新拉取（本地变更后同步 UI）
  const [refreshKey, setRefreshKey] = useState(0);
  const bump = () => setRefreshKey((k) => k + 1);

  const loadToday = () => api.getToday().then(setToday).catch(console.error);

  usePolling(loadToday, 4000, [refreshKey]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPaletteOpen((v) => !v);
      }
      if (e.key === "Escape") setPaletteOpen(false);
    };
    const onOpen = () => setPaletteOpen(true);
    window.addEventListener("keydown", onKey);
    window.addEventListener("open-palette", onOpen);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("open-palette", onOpen);
    };
  }, []);

  return (
    <div className="flex h-full w-full overflow-hidden bg-[#0f1115]">
      <Sidebar current={view} onNav={setView} inboxOpen={today?.open_inbox_count ?? 0} />
      <main className="flex-1 overflow-y-auto px-8 py-6">
        <div className="mx-auto max-w-3xl">
          {view === "today" && (
            <TodayView data={today} onChanged={bump} onNav={(v) => setView(v as ViewName)} />
          )}
          {view === "tasks" && <TasksView refreshKey={refreshKey} onChanged={bump} />}
          {view === "projects" && <ProjectsView refreshKey={refreshKey} onChanged={bump} />}
          {view === "notes" && <NotesView refreshKey={refreshKey} onChanged={bump} />}
          {view === "inbox" && <InboxView refreshKey={refreshKey} onChanged={bump} />}
        </div>
      </main>
      <SearchPalette
        open={paletteOpen}
        onClose={() => setPaletteOpen(false)}
        onNavigate={(kind) => {
          const map: Record<string, ViewName> = {
            task: "tasks",
            project: "projects",
            note: "notes",
            inbox: "inbox",
          };
          setView(map[kind] ?? "today");
          setPaletteOpen(false);
        }}
      />
    </div>
  );
}
