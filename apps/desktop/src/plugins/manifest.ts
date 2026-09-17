// manifest 校验的 TS 镜像实现：规则与 Rust plugin_manifest::PluginManifest::validate 对齐，
// 两边由测试（T6/T7）保证一致。返回错误列表，空数组 = 合法。

import type { PluginManifest } from "./types";
import { KNOWN_EVENTS } from "./types";

const SEG_RE = /^[a-z0-9-]+$/;

function isValidPluginId(id: string): boolean {
  if (!id || id.length > 200 || !id.includes(".")) return false;
  return id
    .split(".")
    .every(
      (seg) =>
        seg.length > 0 &&
        seg.length <= 63 &&
        SEG_RE.test(seg) &&
        !seg.startsWith("-") &&
        !seg.endsWith("-"),
    );
}

export function isValidHost(host: string): boolean {
  if (!host || host.length > 253 || !host.includes(".")) return false;
  return host.split(".").every((seg) => seg.length > 0 && SEG_RE.test(seg));
}

/** 从 URL 提取小写 host（仅支持 http/https，端口剥掉）。与 Rust extract_host 对齐。 */
export function extractHost(url: string): string | null {
  try {
    const u = new URL(url);
    return ["http:", "https:"].includes(u.protocol) &&
      !u.username &&
      !u.password
      ? u.hostname
      : null;
  } catch {
    return null;
  }
}

export function validateManifest(m: PluginManifest): string[] {
  const errors: string[] = [];

  if (
    ![undefined, "", "plugin.protocol/v1", "plugin.protocol/v2"].includes(
      m.api_version,
    )
  ) {
    errors.push(
      `插件必须声明受支持的 API 版本，当前为 '${m.api_version ?? ""}'`,
    );
  }
  const stable = /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/;
  if (!stable.test(m.version)) errors.push("version 必须为 x.y.z 稳定版本");
  if (m.min_host_version) {
    if (!stable.test(m.min_host_version))
      errors.push("min_host_version 必须为 x.y.z 稳定版本");
    else {
      const v = m.min_host_version.split(".").map(Number);
      if (v[0] > 0 || v[1] > 1 || (v[1] === 1 && v[2] > 0))
        errors.push("宿主版本低于 min_host_version");
    }
  }
  const quota = m.permissions?.storage_quota_bytes;
  if (
    quota != null &&
    (!Number.isInteger(quota) || quota <= 0 || quota > 10_000_000)
  ) {
    errors.push("storage quota 必须在 1B–10MB 内");
  }
  const knownCore = new Set([
    "context.read",
    "task.read",
    "task.write",
    "note.read",
    "note.write",
    "inbox.read",
    "inbox.write",
    "search.read",
  ]);
  for (const cap of m.permissions?.core ?? []) {
    if (!knownCore.has(cap)) errors.push(`未知 Core capability '${cap}'`);
  }
  const knownUi = new Set(["command", "view", "today_card", "settings"]);
  for (const cap of m.permissions?.ui ?? []) {
    if (!knownUi.has(cap)) errors.push(`未知 UI capability '${cap}'`);
  }

  if (!isValidPluginId(m.id)) {
    errors.push(`无效插件 id '${m.id}'（需为反向域名）`);
  }
  if (!m.name || !m.name.trim() || [...m.name].length > 100) {
    errors.push("插件 name 不能为空且 ≤100 字符");
  }
  if (
    !m.version ||
    !m.version.trim() ||
    new TextEncoder().encode(m.version).length > 32
  ) {
    errors.push("插件 version 不能为空且 ≤32 字符");
  }
  const entry = m.entry ?? "";
  if (
    !entry.endsWith(".js") ||
    entry.includes("/") ||
    entry.includes("\\") ||
    entry.includes("..")
  ) {
    errors.push(`无效 entry '${entry}'（应为插件目录内的单个 .js 文件名）`);
  }

  for (const h of m.permissions?.network ?? []) {
    if (!isValidHost(h)) errors.push(`无效 host '${h}'`);
  }
  for (const ev of m.permissions?.events ?? []) {
    if (!(KNOWN_EVENTS as readonly string[]).includes(ev)) {
      errors.push(`未知事件 '${ev}'（支持 ${KNOWN_EVENTS.join(", ")}）`);
    }
  }
  for (const c of m.permissions?.cron ?? []) {
    if (c.trim().split(/\s+/).length !== 5) {
      errors.push(`无效 cron '${c}'（需 5 段表达式）`);
    }
  }

  const contrib = m.contributions ?? {};
  const groups: [string, { id: string; title?: string }[]][] = [
    ["view", contrib.views ?? []],
    ["today_card", contrib.today_cards ?? []],
    ["command", contrib.commands ?? []],
    ["settings", contrib.settings ?? []],
  ];
  for (const [kind, items] of groups) {
    const seen = new Set<string>();
    for (const item of items) {
      if (!item.id) errors.push(`${kind} 贡献点 id 不能为空`);
      // today_card 无标题字段，其余标题必填
      if (kind !== "today_card" && !(item.title ?? "").trim()) {
        errors.push(`${kind} '${item.id}' 的标题不能为空`);
      }
      if (seen.has(item.id)) errors.push(`${kind} id '${item.id}' 重复`);
      seen.add(item.id);
    }
  }

  return errors;
}
