// 事件总线：插件 handler 抛错绝不带崩宿主（T11）；支持按 owner 注销（unload 清理，T8）。

export type EventHandler = (payload: unknown) => void;

interface Subscription {
  owner: string;
  handler: EventHandler;
}

export class EventBus {
  private subs = new Map<string, Set<Subscription>>();

  /** 订阅；返回取消订阅函数。 */
  on(topic: string, owner: string, handler: EventHandler): () => void {
    let set = this.subs.get(topic);
    if (!set) {
      set = new Set();
      this.subs.set(topic, set);
    }
    const sub: Subscription = { owner, handler };
    set.add(sub);
    return () => set.delete(sub);
  }

  /** 注销某个 owner 的全部订阅（插件 unload 时调用）。 */
  offOwner(owner: string): void {
    for (const [topic, set] of this.subs) {
      for (const sub of [...set]) {
        if (sub.owner === owner) set.delete(sub);
      }
      if (set.size === 0) this.subs.delete(topic);
    }
  }

  emit(topic: string, payload: unknown): void {
    for (const sub of [...(this.subs.get(topic) ?? [])]) {
      try {
        void Promise.resolve(sub.handler(payload)).catch((e) =>
          console.error(`[plugin-events] ${topic}`, e),
        );
      } catch (e) {
        console.error(`[plugin-events] '${topic}' 的 handler 抛错:`, e);
      }
    }
  }
}

/** 宿主全局事件总线单例：App 的插件宿主与 api.ts 的领域事件发射必须共用同一实例。 */
export const pluginEvents = new EventBus();
