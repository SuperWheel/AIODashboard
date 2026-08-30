// 插件契约类型：与 Rust 侧 plugin_manifest / plugin_service 对齐（snake_case 字段）。
// 前端 loader 在加载前做镜像校验（规则对齐由 T6/T7 测试保证）。

export const KNOWN_EVENTS = [
  "task.completed",
  "task.created",
  "inbox.added",
  "note.created",
] as const;

export interface PluginPermissions {
  /** 允许访问的 host 白名单（精确域名） */
  network?: string[];
  /** 订阅的领域事件（见 KNOWN_EVENTS）；panel.* 面板事件无需声明 */
  events?: string[];
  /** cron 表达式（5 段），由 Rust 侧调度驱动 */
  cron?: string[];
}

export interface PluginViewContribution {
  id: string;
  title: string;
}

export interface PluginCardContribution {
  id: string;
}

export interface PluginCommandContribution {
  id: string;
  title: string;
}

export interface PluginContributions {
  views?: PluginViewContribution[];
  today_cards?: PluginCardContribution[];
  commands?: PluginCommandContribution[];
}

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  /** 入口 JS 文件名（相对插件目录，不允许路径分隔符） */
  entry: string;
  description?: string;
  permissions?: PluginPermissions;
  contributions?: PluginContributions;
}

export interface PluginInfo {
  id: string;
  name: string;
  version: string;
  /** null = 磁盘上存在但尚未注册 */
  enabled: boolean | null;
  dir: string;
  error?: string;
}
