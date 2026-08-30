// T8/T9：ModuleRegistry 注册冲突 / unregisterOwner 干净卸载；EventBus handler 抛错不炸宿主。

import { describe, expect, it, vi } from "vitest";
import { ModuleRegistry, type RegisteredCard } from "../registry";
import { EventBus } from "../events";

const card = (owner: string, id: string): RegisteredCard => ({
  owner,
  id,
  title: id,
  component: () => null,
});

describe("ModuleRegistry", () => {
  it("T9: 注册 → 查询；key 冲突拒绝", () => {
    const r = new ModuleRegistry();
    r.registerCard(card("com.a", "com.a.card"));
    expect(r.cards).toHaveLength(1);

    r.registerView({ owner: "core", key: "today", title: "今天", icon: "◎", component: () => null });
    expect(r.hasView("today")).toBe(true);
    expect(() =>
      r.registerView({ owner: "core", key: "today", title: "重复", icon: "◎", component: () => null }),
    ).toThrow(/冲突/);

    r.registerCommand({ owner: "com.a", id: "com.a.cmd", title: "命令", handler: () => {} });
    expect(() =>
      r.registerCommand({ owner: "com.b", id: "com.a.cmd", title: "冲突", handler: () => {} }),
    ).toThrow(/冲突/);
  });

  it("T8: unregisterOwner 移除该 owner 的全部注册项，不影响他人", () => {
    const r = new ModuleRegistry();
    r.registerCard(card("com.a", "com.a.card1"));
    r.registerCard(card("com.a", "com.a.card2"));
    r.registerCard(card("com.b", "com.b.card"));
    r.registerView({ owner: "com.a", key: "com.a.view", title: "V", icon: "▣", component: () => null });
    r.registerCommand({ owner: "com.a", id: "com.a.cmd", title: "C", handler: () => {} });

    r.unregisterOwner("com.a");

    expect(r.cards.map((c) => c.id)).toEqual(["com.b.card"]);
    expect(r.views).toHaveLength(0);
    expect(r.commands).toHaveLength(0);
  });
});

describe("EventBus", () => {
  it("T11: emit → handler 收到；handler 抛错不影响其他 handler 与宿主", () => {
    const bus = new EventBus();
    const ok = vi.fn();
    const bad = vi.fn(() => {
      throw new Error("boom");
    });

    bus.on("task.completed", "com.a", bad);
    bus.on("task.completed", "com.b", ok);

    expect(() => bus.emit("task.completed", { id: "tsk_1" })).not.toThrow();
    expect(bad).toHaveBeenCalledOnce();
    expect(ok).toHaveBeenCalledOnce();
    expect(ok).toHaveBeenCalledWith({ id: "tsk_1" });
  });

  it("T8: offOwner 移除该 owner 的全部订阅", () => {
    const bus = new EventBus();
    const h = vi.fn();
    bus.on("task.completed", "com.a", h);
    bus.on("panel.refresh", "com.a", h);

    bus.offOwner("com.a");
    bus.emit("task.completed", null);
    bus.emit("panel.refresh", null);

    expect(h).not.toHaveBeenCalled();
  });

  it("on 返回的取消订阅函数生效", () => {
    const bus = new EventBus();
    const h = vi.fn();
    const off = bus.on("topic", "com.a", h);
    off();
    bus.emit("topic", null);
    expect(h).not.toHaveBeenCalled();
  });
});
