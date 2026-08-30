import { useEffect, useRef, useState } from "react";
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
import { EventBus } from "./plugins/events";
import { ModuleRegistry, type PluginCardProps } from "./plugins/registry";
import { loadAllPlugins, type LoadedPlugin } from "./plugins/loader";

export default function App() {
  const [view, setView] = useState<string>("today");
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [today, setToday] = useState<TodayContext | null>(null);
  // refreshKey 变化时所有视图立即重新拉取（本地变更后同步 UI）
  const [refreshKey, setRefreshKey] = useState(0);
  // 插件注册表变化时强制重渲染（注册/卸载贡献点）
  const [pluginsVersion, setPluginsVersion] = useState(0);

  const registryRef = useRef<ModuleRegistry | null>(null);
  const eventsRef = useRef<EventBus | null>(null);
  const pluginsRef = useRef<LoadedPlugin[]>([]);

  if (!registryRef.current) {
    const registry = new ModuleRegistry();
    // 核心五视图与插件走同一注册路径（dogfooding）
    registry.registerView({
      owner: "core",
      key: "today",
      title: "今天",
      icon: "◎",
      component: (p) => <TodayView data={p.today} onChanged={p.onChanged} onNav={p.onNav} extraCards={p.cards} />,
    });
    registry.registerView({
      owner: "core",
      key: "tasks",
      title: "任务",
      icon: "☑",
      component: (p) => <TasksView refreshKey={p.refreshKey} onChanged={p.onChanged} />,
    });
    registry.registerView({
      owner: "core",
      key: "projects",
      title: "项目",
      icon: "▤",
      component: (p) => <ProjectsView refreshKey={p.refreshKey} onChanged={p.onChanged} />,
    });
    registry.registerView({
      owner: "core",
      key: "notes",
      title: "笔记",
      icon: "✎",
      component: (p) => <NotesView refreshKey={p.refreshKey} onChanged={p.onChanged} />,
    });
    registry.registerView({
      owner: "core",
      key: "inbox",
      title: "收件箱",
      icon: "⬇",
      component: (p) => <InboxView refreshKey={p.refreshKey} onChanged={p.onChanged} />,
    });
    registryRef.current = registry;
  }
  if (!eventsRef.current) {
    eventsRef.current = new EventBus();
  }
  const registry = registryRef.current;

  // bump = 本地变更信号：刷新数据 + 通知插件（panel.refresh）
  const bump = () => {
    eventsRef.current?.emit("panel.refresh", null);
    setRefreshKey((k) => k + 1);
  };

  const loadToday = () => api.getToday().then(setToday).catch(console.error);

  usePolling(loadToday, 4000, [refreshKey]);

  // 面板可见性事件：插件可跟随面板状态（panel.show / panel.hide）
  useEffect(() => {
    const onVisibility = () => {
      if (document.visibilityState === "visible") {
        eventsRef.current?.emit("panel.show", null);
      } else {
        eventsRef.current?.emit("panel.hide", null);
      }
    };
    document.addEventListener("visibilitychange", onVisibility);
    return () => document.removeEventListener("visibilitychange", onVisibility);
  }, []);

  useEffect(() => {
    let cancelled = false;
    loadAllPlugins({
      registry,
      events: eventsRef.current!,
      onChanged: bump,
    })
      .then((loaded) => {
        if (!cancelled) {
          pluginsRef.current = loaded;
          setPluginsVersion((v) => v + 1);
        }
      })
      .catch(console.error);
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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

  // 插件卡片槽位（Today 视图消费）
  void pluginsVersion;
  const apiById = new Map(pluginsRef.current.map((p) => [p.id, p.api]));
  const cardsNode = registry.cards.length > 0 && (
    <>
      {registry.cards.map((c) => {
        const props: PluginCardProps = {
          api: apiById.get(c.owner) ?? null,
          onChanged: bump,
          today,
        };
        return <c.component key={c.id} {...props} />;
      })}
    </>
  );

  const activeView =
    registry.views.find((v) => v.key === view) ?? registry.views[0];

  return (
    <div className="flex h-full w-full overflow-hidden bg-[#0f1115]">
      <Sidebar
        items={registry.views.map((v) => ({ key: v.key, label: v.title, icon: v.icon }))}
        current={activeView.key}
        onNav={setView}
        inboxOpen={today?.open_inbox_count ?? 0}
      />
      <main className="flex-1 overflow-y-auto px-8 py-6">
        <div className="mx-auto max-w-3xl">
          <activeView.component
            today={today}
            onChanged={bump}
            onNav={setView}
            refreshKey={refreshKey}
            cards={cardsNode ?? undefined}
          />
        </div>
      </main>
      <SearchPalette
        open={paletteOpen}
        onClose={() => setPaletteOpen(false)}
        onNavigate={(kind) => {
          const map: Record<string, string> = {
            task: "tasks",
            project: "projects",
            note: "notes",
            inbox: "inbox",
          };
          setView(map[kind] ?? "today");
          setPaletteOpen(false);
        }}
        commands={registry.commands}
        onRunCommand={(cmd) => {
          void Promise.resolve(cmd.handler())
            .catch((e) => alert(`命令执行失败: ${String(e)}`))
            .finally(() => {
              bump();
              setPaletteOpen(false);
            });
        }}
      />
    </div>
  );
}
