// T10：API 桥的权限执行——network 白名单、events 声明、KV 命名空间锁定、actor 归因。

import { describe, expect, it, vi } from "vitest";
import { createPluginApi, type PluginDeps } from "../bridge";
import { CronRegistry } from "../crons";
import { EventBus } from "../events";
import { ModuleRegistry } from "../registry";
import type { PluginManifest } from "../types";

// 桩掉后端 api：只记录调用，断言 actor 与命名空间参数
vi.mock("../../api", () => ({
  api: {
    getToday: vi.fn(async () => ({ date: "2026-08-30" })),
    listTasks: vi.fn(async () => []),
    createTaskAs: vi.fn(async (_actor: string, title: string) => ({ title })),
    checkinAs: vi.fn(async () => ({})),
    archiveTask: vi.fn(async () => ({})),
    deleteTaskAs: vi.fn(async () => undefined),
    searchAll: vi.fn(async () => ({ total: 0, hits: [] })),
    addInboxItemAs: vi.fn(async () => ({})),
    createNoteAs: vi.fn(async () => ({})),
    pluginKvGet: vi.fn(async (pluginId: string, key: string) => ({ pluginId, key })),
    pluginKvSet: vi.fn(async () => undefined),
    pluginKvDelete: vi.fn(async () => true),
    pluginKvList: vi.fn(async (pluginId: string) => [{ key: `${pluginId}/k`, value: "v" }]),
    pluginHttpFetch: vi.fn(async () => ({ status: 200, text: "{}", json: {} })),
  },
}));
import { api as coreApi } from "../../api";

const manifest = (perms: PluginManifest["permissions"]): PluginManifest => ({
  id: "com.test.echo",
  name: "Echo",
  version: "0.1.0",
  entry: "main.js",
  permissions: perms,
});

function makeDeps(): PluginDeps & { registry: ModuleRegistry; events: EventBus; crons: CronRegistry } {
  return {
    registry: new ModuleRegistry(),
    events: new EventBus(),
    crons: new CronRegistry(),
    onChanged: () => {},
  };
}

describe("API 桥权限执行", () => {
  it("T10: fetch 未声明 host 拒绝（本地判定，不发起请求）", async () => {
    const deps = makeDeps();
    const apiObj = createPluginApi("com.test.echo", manifest({ network: ["api.github.com"] }), deps);

    await expect(apiObj.fetch("https://evil.com/x")).rejects.toThrow(/network 权限/);
    expect(coreApi.pluginHttpFetch).not.toHaveBeenCalled();

    await expect(apiObj.fetch("https://api.github.com/x")).resolves.toMatchObject({ status: 200 });
  });

  it("T10: events 订阅需 manifest 声明；panel.* 无需声明", () => {
    const deps = makeDeps();
    const apiObj = createPluginApi("com.test.echo", manifest({ events: ["task.completed"] }), deps);

    expect(() => apiObj.events.on("task.completed", () => {})).not.toThrow();
    expect(() => apiObj.events.on("task.created", () => {})).toThrow(/events 权限/);
    expect(() => apiObj.events.on("panel.refresh", () => {})).not.toThrow();
  });

  it("T10: KV 一律锁定本插件命名空间", async () => {
    const deps = makeDeps();
    const apiObj = createPluginApi("com.test.echo", manifest({}), deps);

    await apiObj.storage.kv.set("counter", "1");
    expect(coreApi.pluginKvSet).toHaveBeenCalledWith("com.test.echo", "counter", "1");

    await apiObj.storage.kv.get("counter");
    expect(coreApi.pluginKvGet).toHaveBeenCalledWith("com.test.echo", "counter");

    // 公开协议 = storage.kv.*（对齐 PLUGIN_API.md）；桥不暴露任何能改 pluginId 的口子
    expect(Object.keys(apiObj.storage)).toEqual(["kv"]);
    expect(Object.keys(apiObj.storage.kv).sort()).toEqual(["delete", "get", "list", "set"]);
  });

  it("领域写入带 actor=plugin:<id>（审计归因）", async () => {
    const deps = makeDeps();
    const apiObj = createPluginApi("com.test.echo", manifest({}), deps);

    await apiObj.core.createTask("插件创建的任务");
    expect(coreApi.createTaskAs).toHaveBeenCalledWith(
      "plugin:com.test.echo",
      "插件创建的任务",
      undefined,
    );
  });

  it("贡献点注册进 registry 且带插件前缀（冲突会抛错）", () => {
    const deps = makeDeps();
    const apiObj = createPluginApi("com.test.echo", manifest({}), deps);

    const FakeComponent = () => null;
    apiObj.ui.registerTodayCard({ id: "card", title: "Echo", component: FakeComponent });
    expect(deps.registry.cards.map((c) => c.id)).toEqual(["com.test.echo.card"]);

    expect(() =>
      apiObj.ui.registerTodayCard({ id: "card", title: "Echo", component: FakeComponent }),
    ).toThrow(/冲突/);
  });

  it("Today 卡片 size 透传进 registry；非法 size 拒绝注册", () => {
    const deps = makeDeps();
    const apiObj = createPluginApi("com.test.echo", manifest({}), deps);

    const FakeComponent = () => null;
    apiObj.ui.registerTodayCard({ id: "a", title: "A", size: "sm", component: FakeComponent });
    apiObj.ui.registerTodayCard({ id: "b", title: "B", component: FakeComponent });
    expect(deps.registry.cards.map((c) => c.size)).toEqual(["sm", undefined]);

    expect(() =>
      apiObj.ui.registerTodayCard({
        id: "c",
        title: "C",
        // @ts-expect-error 运行时校验：绕过类型声明的非法值
        size: "xl",
        component: FakeComponent,
      }),
    ).toThrow(/size 非法/);
    expect(deps.registry.cards.map((c) => c.id)).not.toContain("com.test.echo.c");
  });

  it("registerCron 需要 manifest 声明该表达式；声明后进入 cron 注册表", () => {
    const deps = makeDeps();
    const apiObj = createPluginApi(
      "com.test.echo",
      manifest({ cron: ["*/1 * * * *"] }),
      deps,
    );

    expect(() => apiObj.registerCron("0 9 * * *", () => {})).toThrow(/cron 权限/);
    expect(deps.crons.size).toBe(0);

    apiObj.registerCron("*/1 * * * *", () => {});
    expect(deps.crons.size).toBe(1);
  });
});
