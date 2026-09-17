import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { usePolling } from "./hooks";
import { api } from "./api";
import type { TodayContext } from "./types";
import Sidebar from "./components/Sidebar";
import TodayView from "./components/TodayView";
import TasksView from "./components/TasksView";
import LibrariesView from "./components/LibrariesView";
import ProjectsView from "./components/ProjectsView";
import NotesView from "./components/NotesView";
import InboxView from "./components/InboxView";
import SearchPalette from "./components/SearchPalette";
import PluginsView from "./components/PluginsView";
import PluginApprovalModal from "./components/PluginApprovalModal";
import PluginErrorBoundary from "./components/PluginErrorBoundary";
import DialogHost, { toastError } from "./components/DialogHost";
import { EventBus, pluginEvents } from "./plugins/events";
import { CronRegistry } from "./plugins/crons";
import { ModuleRegistry, type PluginCardProps } from "./plugins/registry";
import {
  PluginHost,
  type LoadedPlugin,
  type PluginHostOptions,
} from "./plugins/loader";
import type { PluginInfo } from "./plugins/types";

export default function App() {
  const [view, setView] = useState<string>("today");
  // 跨视图导航参数（如 tasks 的 tab、notes 的笔记 id）
  const [navParam, setNavParam] = useState<string | undefined>(undefined);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [today, setToday] = useState<TodayContext | null>(null);
  // refreshKey 变化时所有视图立即重新拉取（本地变更后同步 UI）
  const [refreshKey, setRefreshKey] = useState(0);
  // 插件注册表变化时强制重渲染（注册/卸载贡献点）
  const [pluginsVersion, setPluginsVersion] = useState(0);
  // 等待权限确认的新发现插件
  const [pendingPlugins, setPendingPlugins] = useState<PluginInfo[]>([]);

  const registryRef = useRef<ModuleRegistry | null>(null);
  const eventsRef = useRef<EventBus | null>(null);
  const cronsRef = useRef<CronRegistry | null>(null);
  const pluginsRef = useRef<LoadedPlugin[]>([]);

  if (!registryRef.current) {
    const registry = new ModuleRegistry();
    // 核心五视图与插件走同一注册路径（dogfooding）
    registry.registerView({
      owner: "core",
      key: "today",
      title: "今天",
      icon: "◎",
      component: (p) => (
        <TodayView
          data={p.today}
          onChanged={p.onChanged}
          onNav={p.onNav}
          extraCards={p.cards}
        />
      ),
    });
    registry.registerView({
      owner: "core",
      key: "tasks",
      title: "任务",
      icon: "☑",
      component: (p) => (
        <TasksView
          refreshKey={p.refreshKey}
          onChanged={p.onChanged}
          navParam={p.navParam}
          onNav={p.onNav}
        />
      ),
    });
    registry.registerView({
      owner: "core",
      key: "libraries",
      title: "重要日",
      icon: "📅",
      component: (p) => (
        <LibrariesView
          refreshKey={p.refreshKey}
          onChanged={p.onChanged}
          navParam={p.navParam}
          onNav={p.onNav}
        />
      ),
    });
    registry.registerView({
      owner: "core",
      key: "projects",
      title: "项目",
      icon: "▤",
      component: (p) => (
        <ProjectsView refreshKey={p.refreshKey} onChanged={p.onChanged} />
      ),
    });
    registry.registerView({
      owner: "core",
      key: "notes",
      title: "笔记",
      icon: "✎",
      component: (p) => (
        <NotesView
          refreshKey={p.refreshKey}
          onChanged={p.onChanged}
          navParam={p.navParam}
        />
      ),
    });
    registry.registerView({
      owner: "core",
      key: "inbox",
      title: "收件箱",
      icon: "⬇",
      component: (p) => (
        <InboxView refreshKey={p.refreshKey} onChanged={p.onChanged} />
      ),
    });
    registry.registerView({
      owner: "core",
      key: "plugins",
      title: "插件",
      icon: "⚙",
      component: (p) => (
        <PluginsView
          refreshKey={p.refreshKey}
          onChanged={p.onChanged}
          registry={registry}
          pluginsRef={pluginsRef}
        />
      ),
    });
    registryRef.current = registry;
  }
  if (!eventsRef.current) {
    // 用模块单例：api.ts 发射领域事件（task.completed 等）与插件宿主必须是同一条总线
    eventsRef.current = pluginEvents;
  }
  if (!cronsRef.current) {
    cronsRef.current = new CronRegistry();
  }
  const registry = registryRef.current;

  const hostOpts = (): PluginHostOptions => ({
    registry,
    events: eventsRef.current!,
    crons: cronsRef.current!,
    onChanged: bump,
    onLoadError: (id, error) =>
      toastError(`插件 ${id} 加载失败，已停用：${String(error)}`),
  });

  // bump = 本地变更信号：刷新数据 + 通知插件（panel.refresh）
  const bump = () => {
    eventsRef.current?.emit("panel.refresh", null);
    setRefreshKey((k) => k + 1);
  };

  const nav = (v: string, param?: string) => {
    setNavParam(param);
    setView(v);
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
    const host = new PluginHost(hostOpts(), ({ plugins, pending }) => {
      pluginsRef.current = plugins;
      setPendingPlugins(pending);
      setPluginsVersion((v) => v + 1);
      bump();
    });
    const poll = () => void host.poll().catch(console.error);
    const reload = () => void host.reload();
    poll();
    const timer = setInterval(poll, 2000);
    window.addEventListener("reload-plugins", reload);
    return () => {
      clearInterval(timer);
      window.removeEventListener("reload-plugins", reload);
      void host.stop();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Rust cron 调度 → 插件 handler
  useEffect(() => {
    const unlisten = listen<{ plugin_id: string; expr: string }>(
      "plugin-cron",
      (e) => {
        cronsRef.current?.dispatch(e.payload);
      },
    );
    return () => {
      void unlisten.then((f) => f());
    };
  }, []);

  // 新插件权限确认
  const approvePlugin = (info: PluginInfo) => {
    setPendingPlugins((list) => list.filter((p) => p.id !== info.id));
    void (async () => {
      try {
        await api.pluginSetEnabled(info.id, true, info.fingerprint);
        window.dispatchEvent(new CustomEvent("reload-plugins"));
      } catch (e) {
        console.error(`[plugins] 启用 ${info.id} 失败:`, e);
      }
    })();
  };
  const dismissPlugin = (info: PluginInfo) => {
    setPendingPlugins((list) => list.filter((p) => p.id !== info.id));
    // 登记为停用：不再重复询问，可在插件页随时启用
    void api.pluginSetEnabled(info.id, false).catch(console.error);
  };

  // 待确认插件的 manifest（弹窗展示权限用）
  const [pendingManifests, setPendingManifests] = useState<
    Record<string, import("./plugins/types").PluginManifest>
  >({});
  useEffect(() => {
    let alive = true;
    void (async () => {
      const ms: Record<string, import("./plugins/types").PluginManifest> = {};
      for (const p of pendingPlugins) {
        try {
          ms[p.id] = await api.pluginReadManifest(p.id);
        } catch {
          // 读不到时弹窗降级为列表信息
        }
      }
      if (alive) setPendingManifests(ms);
    })();
    return () => {
      alive = false;
    };
  }, [pendingPlugins]);

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

  // 插件卡片槽位（Today 视图消费）：按 size 包一层 Bento 网格占位（12 列）
  void pluginsVersion;
  const apiById = new Map(pluginsRef.current.map((p) => [p.id, p.api]));
  const CARD_SPAN: Record<string, string> = {
    sm: "lg:col-span-3",
    md: "lg:col-span-4",
    lg: "lg:col-span-6",
  };
  const cardsNode = registry.cards.length > 0 && (
    <>
      {registry.cards.map((c) => {
        const props: PluginCardProps = {
          api: apiById.get(c.owner) ?? null,
          onChanged: bump,
          today: pluginsRef.current
            .find((p) => p.id === c.owner)
            ?.manifest.permissions?.core?.includes("context.read")
            ? today
            : null,
        };
        return (
          <div key={c.id} className={CARD_SPAN[c.size ?? "md"]}>
            <PluginErrorBoundary name={c.id}>
              <c.component {...props} />
            </PluginErrorBoundary>
          </div>
        );
      })}
    </>
  );

  const activeView =
    registry.views.find((v) => v.key === view) ?? registry.views[0];
  // Today（Bento 总控台）与插件视图用宽容器；列表类子页保持单列工作室
  const wide = activeView.key === "today" || activeView.owner !== "core";

  return (
    <div className="flex h-full w-full overflow-hidden bg-bg">
      <Sidebar
        items={registry.views.map((v) => ({
          key: v.key,
          label: v.title,
          icon: v.icon,
        }))}
        current={activeView.key}
        onNav={nav}
        inboxOpen={today?.open_inbox_count ?? 0}
      />
      <main className="flex-1 overflow-y-auto px-8 py-6">
        <div className={`mx-auto ${wide ? "max-w-6xl" : "max-w-3xl"}`}>
          {activeView.owner === "core" ? (
            <activeView.component
              today={today}
              onChanged={bump}
              onNav={nav}
              navParam={navParam}
              refreshKey={refreshKey}
              cards={cardsNode ?? undefined}
            />
          ) : (
            /* 插件视图包错误边界：插件抛错只降级这一块，不白屏整个面板 */
            <PluginErrorBoundary name={activeView.key} key={activeView.key}>
              <activeView.component
                api={apiById.get(activeView.owner)}
                today={
                  pluginsRef.current
                    .find((p) => p.id === activeView.owner)
                    ?.manifest.permissions?.core?.includes("context.read")
                    ? today
                    : null
                }
                onChanged={bump}
                onNav={nav}
                navParam={navParam}
                refreshKey={refreshKey}
              />
            </PluginErrorBoundary>
          )}
        </div>
      </main>
      {/* 新插件权限确认（安装时刻） */}
      {pendingPlugins.length > 0 && (
        <PluginApprovalModal
          plugins={pendingPlugins}
          manifests={pendingManifests}
          onApprove={approvePlugin}
          onDismiss={dismissPlugin}
        />
      )}
      <DialogHost />
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
          nav(map[kind] ?? "today");
          setPaletteOpen(false);
        }}
        commands={registry.commands}
        onRunCommand={(cmd) => {
          void Promise.resolve()
            .then(() => cmd.handler())
            .catch((e) => toastError(`命令执行失败: ${String(e)}`))
            .finally(() => {
              bump();
              setPaletteOpen(false);
            });
        }}
      />
    </div>
  );
}
