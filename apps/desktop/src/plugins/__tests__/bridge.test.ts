import { describe, it, expect, vi } from "vitest";
import { createPluginApi } from "../bridge";
import { ModuleRegistry } from "../registry";
import { EventBus } from "../events";
import { CronRegistry } from "../crons";
import type { PluginManifest } from "../types";
import { Lifecycle } from "../../../../../packages/plugin-sdk/src/index";
vi.mock("../../api", () => ({ api: { pluginCall: vi.fn(async () => null) } }));
import { api } from "../../api";
function setup(perms: PluginManifest["permissions"] = {}) {
  const registry = new ModuleRegistry(),
    events = new EventBus(),
    crons = new CronRegistry(),
    life = new Lifecycle();
  const manifest: PluginManifest = {
    id: "com.test.p",
    name: "P",
    version: "1.0.0",
    entry: "main.js",
    api_version: "plugin.protocol/v2",
    permissions: perms,
    contributions: {
      commands: [{ id: "c", title: "C" }],
      today_cards: [{ id: "card" }],
      views: [{ id: "v", title: "V" }],
    },
  };
  return {
    registry,
    events,
    crons,
    life,
    api: createPluginApi(
      manifest.id,
      "test-token",
      manifest,
      { registry, events, crons, onChanged: () => {} },
      life,
    ),
  };
}
describe("真实宿主桥与共享 SDK 权限", () => {
  it("空权限拒绝写入、today、KV 和 UI", async () => {
    const s = setup();
    await expect(s.api.core.createTask("x")).rejects.toMatchObject({
      code: "permission_denied",
    });
    await expect(s.api.core.today()).rejects.toThrow();
    await expect(s.api.storage.kv.get("x")).rejects.toThrow();
    expect(() =>
      s.api.ui.registerCommand({ id: "c", title: "C", handler: () => {} }),
    ).toThrow();
  });
  it("传递 token，不能传 actor 或他人 plugin_id", async () => {
    vi.clearAllMocks();
    const s = setup({ core: ["task.write"], storage_quota_bytes: 100 });
    await s.api.core.createTask("x");
    expect(api.pluginCall).toHaveBeenCalledWith("test-token", "create_task", {
      title: "x",
      target: undefined,
    });
    await s.api.storage.kv.set("k", "v");
    expect(api.pluginCall).toHaveBeenCalledWith("test-token", "kv_set", {
      key: "k",
      value: "v",
    });
  });
  it("UI 权限与贡献声明均检查，disposer 幂等", () => {
    const s = setup({ ui: ["command", "today_card", "view"] });
    const off = s.api.ui.registerCommand({
      id: "c",
      title: "C",
      handler: () => {},
    });
    expect(s.registry.commands).toHaveLength(1);
    expect(() =>
      s.api.ui.registerCommand({ id: "other", title: "X", handler: () => {} }),
    ).toThrow();
    off();
    off();
    expect(s.registry.commands).toHaveLength(0);
    s.api.ui.registerTodayCard({
      id: "card",
      title: "C",
      size: "sm",
      component: () => null,
    });
    expect(s.registry.cards[0].size).toBe("sm");
    s.life.dispose();
    expect(s.registry.cards).toHaveLength(0);
  });
  it("事件不能伪造领域事实或跨命名空间监听", () => {
    const s = setup({ events: ["task.created"] });
    const f = vi.fn();
    s.api.events.on("task.created", f);
    s.events.emit("task.created", 1);
    expect(f).toHaveBeenCalledWith(1);
    expect(() => s.api.events.emit("task.created", {})).toThrow();
    expect(() =>
      s.api.events.on("plugin.com.test.p.child:secret", f),
    ).toThrow();
    const off = s.api.events.on("plugin.com.test.p:local", f);
    s.api.events.emit("plugin.com.test.p:local", 2);
    expect(f).toHaveBeenCalledWith(2);
    off();
    s.life.dispose();
    expect(() => s.api.events.on("panel.refresh", f)).toThrow();
  });
  it("停用撤销旧命令和 cron 回调", () => {
    const s = setup({ ui: ["command"], cron: ["* * * * *"] });
    const f = vi.fn();
    s.api.ui.registerCommand({ id: "c", title: "C", handler: f });
    const command = s.registry.commands[0];
    s.api.registerCron("* * * * *", f);
    s.life.dispose();
    void command.handler();
    s.crons.dispatch({ plugin_id: "com.test.p", expr: "* * * * *" });
    expect(f).not.toHaveBeenCalled();
    expect(s.crons.size).toBe(0);
  });
});
