// cron 注册表：dispatch 按 plugin_id+expr 路由；handler 抛错不炸宿主；offOwner 干净注销。

import { describe, expect, it, vi } from "vitest";
import { CronRegistry } from "../crons";

describe("CronRegistry", () => {
  it("dispatch 只路由到匹配的 plugin_id + expr", () => {
    const crons = new CronRegistry();
    const a = vi.fn();
    const b = vi.fn();
    crons.on("com.a", "*/1 * * * *", a);
    crons.on("com.b", "*/1 * * * *", b);
    crons.on("com.a", "0 9 * * *", b);

    crons.dispatch({ plugin_id: "com.a", expr: "*/1 * * * *" });
    expect(a).toHaveBeenCalledOnce();
    expect(b).not.toHaveBeenCalled();

    crons.dispatch({ plugin_id: "com.a", expr: "0 9 * * *" });
    expect(b).toHaveBeenCalledOnce();
  });

  it("handler 抛错（同步与异步）都不影响宿主与其他 handler", () => {
    const crons = new CronRegistry();
    const bad = vi.fn(() => {
      throw new Error("boom");
    });
    const badAsync = vi.fn(() => Promise.reject(new Error("async boom")));
    const ok = vi.fn();
    crons.on("com.a", "*", bad);
    crons.on("com.a", "*", badAsync);
    crons.on("com.a", "*", ok);

    expect(() => crons.dispatch({ plugin_id: "com.a", expr: "*" })).not.toThrow();
    expect(ok).toHaveBeenCalledOnce();
  });

  it("offOwner 注销该插件全部 cron", () => {
    const crons = new CronRegistry();
    const h = vi.fn();
    crons.on("com.a", "*/1 * * * *", h);
    crons.on("com.a", "0 9 * * *", h);
    crons.on("com.b", "*/1 * * * *", h);

    crons.offOwner("com.a");
    expect(crons.size).toBe(1);

    crons.dispatch({ plugin_id: "com.a", expr: "*/1 * * * *" });
    expect(h).not.toHaveBeenCalled();
  });
});
