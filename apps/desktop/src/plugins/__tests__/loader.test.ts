import { describe, it, expect, vi, afterEach } from "vitest";
import { activateModule, PluginHost } from "../loader";
import { ModuleRegistry } from "../registry";
import { EventBus } from "../events";
import { CronRegistry } from "../crons";
import type { PluginApi } from "../bridge";
vi.mock("../../api", () => ({
  api: {
    pluginCall: vi.fn(async () => null),
    pluginCloseContext: vi.fn(async () => {}),
    pluginList: vi.fn(async () => []),
  },
}));
import { api } from "../../api";
const manifest = {
  id: "com.test.p",
  name: "P",
  version: "1.0.0",
  entry: "main.js",
  api_version: "plugin.protocol/v2",
  permissions: {
    ui: ["command"],
    events: ["task.created"],
    cron: ["* * * * *"],
  },
  contributions: { commands: [{ id: "c", title: "C" }] },
};
const opts = () => ({
  registry: new ModuleRegistry(),
  events: new EventBus(),
  crons: new CronRegistry(),
  onChanged: () => {},
});
afterEach(() => vi.useRealTimers());
describe("加载事务与取消", () => {
  it("onload 抛错清理已注册项目", async () => {
    const o = opts();
    await expect(
      activateModule(
        manifest.id,
        manifest,
        "token",
        {
          onload(a) {
            a.ui.registerCommand({ id: "c", title: "C", handler: () => {} });
            a.events.on("task.created", () => {});
            a.registerCron("* * * * *", () => {});
            throw new Error("boom");
          },
        },
        o,
        new AbortController().signal,
      ),
    ).rejects.toThrow("boom");
    expect(o.registry.commands).toHaveLength(0);
    expect(o.crons.size).toBe(0);
  });
  it("超时后继续的异步 onload 无法注册", async () => {
    vi.useFakeTimers();
    const o = opts();
    let release!: () => void;
    let captured!: PluginApi;
    const work = activateModule(
      manifest.id,
      manifest,
      "token",
      {
        async onload(a) {
          captured = a;
          await new Promise<void>((r) => {
            release = r;
          });
        },
      },
      o,
      new AbortController().signal,
    );
    const rejected = expect(work).rejects.toThrow("8 秒");
    await vi.advanceTimersByTimeAsync(8001);
    await rejected;
    release();
    await Promise.resolve();
    expect(() =>
      captured.ui.registerCommand({ id: "c", title: "C", handler: () => {} }),
    ).toThrow("关闭");
    expect(o.registry.commands).toHaveLength(0);
  });
  it("abort 后清理，重复 dispose 幂等", async () => {
    const o = opts();
    const abort = new AbortController();
    const unload = vi.fn();
    const p = await activateModule(
      manifest.id,
      manifest,
      "token",
      {
        onload(a) {
          a.ui.registerCommand({ id: "c", title: "C", handler: () => {} });
        },
        onunload: unload,
      },
      o,
      abort.signal,
    );
    abort.abort();
    expect(o.registry.commands).toHaveLength(0);
    await p.dispose();
    await p.dispose();
    expect(unload).toHaveBeenCalledOnce();
  });
  it("连点 reload 只排队执行，停止后不会恢复", async () => {
    vi.clearAllMocks();
    const h = new PluginHost(opts(), () => {});
    await Promise.all([h.reload(), h.reload(), h.reload()]);
    await h.stop();
    const count = vi.mocked(api.pluginList).mock.calls.length;
    await h.reload();
    expect(vi.mocked(api.pluginList).mock.calls.length).toBe(count);
    expect(count).toBeLessThanOrEqual(2);
  });
});
