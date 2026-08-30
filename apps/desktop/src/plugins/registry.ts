// ModuleRegistry：视图 / Today 卡片 / ⌘K 命令的统一注册表。
// 核心五视图与插件贡献点走同一条路径（owner: "core" | pluginId），unregisterOwner 支持干净卸载。

import type { ComponentType, ReactNode } from "react";
import type { TodayContext } from "../types";

export interface PluginViewProps {
  today: TodayContext | null;
  onChanged: () => void;
  onNav: (v: string) => void;
  refreshKey: number;
  /** 仅 Today 视图消费：插件卡片槽位 */
  cards?: ReactNode;
}

export interface PluginCardProps {
  /** 注册该卡片的插件的 PluginApi（loader 保证一一对应） */
  api: unknown;
  onChanged: () => void;
  today: TodayContext | null;
}

export interface RegisteredView {
  owner: string;
  /** 全局唯一视图 key（插件视图为 `${pluginId}.${viewId}`） */
  key: string;
  title: string;
  icon: string;
  component: ComponentType<PluginViewProps>;
}

export interface RegisteredCard {
  owner: string;
  /** 全局唯一：`${pluginId}.${cardId}` */
  id: string;
  title: string;
  component: ComponentType<PluginCardProps>;
}

export interface RegisteredCommand {
  owner: string;
  /** 全局唯一：`${pluginId}.${commandId}` */
  id: string;
  title: string;
  handler: () => void | Promise<void>;
}

export class ModuleRegistry {
  private viewMap = new Map<string, RegisteredView>();
  private cardMap = new Map<string, RegisteredCard>();
  private commandMap = new Map<string, RegisteredCommand>();

  registerView(v: RegisteredView): void {
    if (this.viewMap.has(v.key)) throw new Error(`视图 key 冲突: ${v.key}`);
    this.viewMap.set(v.key, v);
  }

  registerCard(c: RegisteredCard): void {
    if (this.cardMap.has(c.id)) throw new Error(`卡片 id 冲突: ${c.id}`);
    this.cardMap.set(c.id, c);
  }

  registerCommand(cmd: RegisteredCommand): void {
    if (this.commandMap.has(cmd.id)) throw new Error(`命令 id 冲突: ${cmd.id}`);
    this.commandMap.set(cmd.id, cmd);
  }

  get views(): RegisteredView[] {
    return [...this.viewMap.values()];
  }

  get cards(): RegisteredCard[] {
    return [...this.cardMap.values()];
  }

  get commands(): RegisteredCommand[] {
    return [...this.commandMap.values()];
  }

  hasView(key: string): boolean {
    return this.viewMap.has(key);
  }

  /** 卸载某个 owner 的全部注册项（插件 unload / 复载时调用）。 */
  unregisterOwner(owner: string): void {
    for (const [k, v] of [...this.viewMap]) {
      if (v.owner === owner) this.viewMap.delete(k);
    }
    for (const [k, v] of [...this.cardMap]) {
      if (v.owner === owner) this.cardMap.delete(k);
    }
    for (const [k, v] of [...this.commandMap]) {
      if (v.owner === owner) this.commandMap.delete(k);
    }
  }
}
