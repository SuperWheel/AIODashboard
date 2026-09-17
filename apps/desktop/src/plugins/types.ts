// 插件契约类型：与 Rust 侧 plugin_manifest / plugin_service 对齐（snake_case 字段）。
// 前端 loader 在加载前做镜像校验（规则对齐由 T6/T7 测试保证）。

export const KNOWN_EVENTS = [
  "task.completed",
  "task.created",
  "inbox.added",
  "note.created",
] as const;

export type { PluginManifest } from "../../../../packages/plugin-sdk/src/index";
import type { PluginManifest } from "../../../../packages/plugin-sdk/src/index";

export type PluginImportSourceKind = "zip" | "directory";

export interface PluginImportCheck {
  id: string;
  content_sha256: string;
  archive_sha256: string | null;
  target_stamp: string;
}

export interface PluginImportPreview {
  manifest: PluginManifest;
  source: PluginImportSourceKind;
  source_path: string;
  replacing: boolean;
  current_version: string | null;
  current_enabled: boolean;
  check: PluginImportCheck;
}

export interface PluginInstallResult {
  id: string;
  version: string;
  sha256: string | null;
  content_sha256: string;
  enabled: boolean;
}

export interface PluginInfo {
  id: string;
  name: string;
  version: string;
  /** null = 磁盘上存在但尚未注册 */
  enabled: boolean | null;
  dir: string;
  source: string;
  trust_mode: string;
  api_version: string;
  sha256?: string | null;
  installed_version?: string | null;
  previous_version?: string | null;
  fingerprint: string;
  integrity: string;
  legacy: boolean;
  revision: number;
  pending_approval: boolean;
  error?: string;
}
