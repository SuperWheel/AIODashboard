// ModuleRegistry：视图 / Today 卡片 / ⌘K 命令的统一注册表。
// 核心五视图与插件贡献点走同一条路径（owner: "core" | pluginId），unregisterOwner 支持干净卸载。

import type { ComponentType } from "react";

import type {
  PluginViewProps,
  PluginCardProps,
  SettingsDefinition,
} from "../../../../packages/plugin-sdk/src/index";
export type {
  PluginViewProps,
  PluginCardProps,
} from "../../../../packages/plugin-sdk/src/index";
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
  /** Bento 网格占位：sm=3 列 / md=4 列 / lg=6 列（Today 页 12 列网格，缺省按 md） */
  size?: CardSize;
}

export type CardSize = "sm" | "md" | "lg";

export interface RegisteredCommand {
  owner: string;
  /** 全局唯一：`${pluginId}.${commandId}` */
  id: string;
  title: string;
  handler: () => void | Promise<void>;
}

export class ModuleRegistry {
  private settingsMap = new Map<
    string,
    { owner: string; definition: SettingsDefinition }
  >();
  private viewMap = new Map<string, RegisteredView>();
  private cardMap = new Map<string, RegisteredCard>();
  private commandMap = new Map<string, RegisteredCommand>();

  registerView(v: RegisteredView): () => void {
    if (this.viewMap.has(v.key)) throw new Error(`视图 key 冲突: ${v.key}`);
    this.viewMap.set(v.key, v);
    return () => {
      if (this.viewMap.get(v.key) === v) this.viewMap.delete(v.key);
    };
  }

  unregisterView(key: string): void {
    this.viewMap.delete(key);
  }

  registerCard(c: RegisteredCard): () => void {
    if (this.cardMap.has(c.id)) throw new Error(`卡片 id 冲突: ${c.id}`);
    this.cardMap.set(c.id, c);
    return () => {
      if (this.cardMap.get(c.id) === c) this.cardMap.delete(c.id);
    };
  }

  unregisterCard(id: string): void {
    this.cardMap.delete(id);
  }

  registerCommand(cmd: RegisteredCommand): () => void {
    if (this.commandMap.has(cmd.id)) throw new Error(`命令 id 冲突: ${cmd.id}`);
    this.commandMap.set(cmd.id, cmd);
    return () => {
      if (this.commandMap.get(cmd.id) === cmd) this.commandMap.delete(cmd.id);
    };
  }

  unregisterCommand(id: string): void {
    this.commandMap.delete(id);
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

  registerSettings(owner: string, definition: SettingsDefinition): () => void {
    const key = `${owner}:${definition.id}`;
    if (this.settingsMap.has(key)) throw new Error("设置冲突");
    const item = { owner, definition };
    this.settingsMap.set(key, item);
    return () => {
      if (this.settingsMap.get(key) === item) this.settingsMap.delete(key);
    };
  }
  get settings() {
    return [...this.settingsMap.values()];
  }
  /** 卸载某个 owner 的全部注册项（插件 unload / 复载时调用）。 */
  unregisterOwner(owner: string): void {
    for (const [k, v] of this.settingsMap)
      if (v.owner === owner) this.settingsMap.delete(k);
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
