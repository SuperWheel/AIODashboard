// cron 注册表：Rust 调度器到点发 `plugin-cron` 事件 → 宿主 dispatch → 插件 handler。
// handler 抛错不炸宿主；按 owner 可整体注销（unload）。

export interface CronFire {
  plugin_id: string;
  expr: string;
}

type CronHandler = () => void | Promise<void>;

export class CronRegistry {
  private handlers = new Map<string, Set<CronHandler>>();

  private key(owner: string, expr: string): string {
    return `${owner}|${expr}`;
  }

  /** 注册（expr 已在桥层做过 manifest 权限校验）。 */
  on(owner: string, expr: string, handler: CronHandler): void {
    const k = this.key(owner, expr);
    let set = this.handlers.get(k);
    if (!set) {
      set = new Set();
      this.handlers.set(k, set);
    }
    set.add(handler);
  }

  offOwner(owner: string): void {
    const prefix = `${owner}|`;
    for (const k of [...this.handlers.keys()]) {
      if (k.startsWith(prefix)) this.handlers.delete(k);
    }
  }

  dispatch(fire: CronFire): void {
    const set = this.handlers.get(this.key(fire.plugin_id, fire.expr));
    if (!set) return;
    for (const h of [...set]) {
      try {
        void Promise.resolve(h()).catch((e) =>
          console.error(`[plugin-cron] ${fire.plugin_id} '${fire.expr}' handler 抛错:`, e),
        );
      } catch (e) {
        console.error(`[plugin-cron] ${fire.plugin_id} '${fire.expr}' handler 同步抛错:`, e);
      }
    }
  }

  get size(): number {
    return this.handlers.size;
  }
}
