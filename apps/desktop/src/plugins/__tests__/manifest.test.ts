// T7：TS 侧 manifest 校验与 Rust 规则对齐（同一批非法样例必须两边都拒绝）。

import { describe, expect, it } from "vitest";
import { extractHost, isValidHost, validateManifest } from "../manifest";
import type { PluginManifest } from "../types";

const valid: PluginManifest = {
  id: "com.test.echo",
  name: "Echo",
  version: "0.1.0",
  entry: "main.js",
};

describe("validateManifest（与 Rust validate 对齐）", () => {
  it("最小合法 manifest 通过，缺省 permissions/contributions 视为空", () => {
    expect(validateManifest(valid)).toEqual([]);
  });

  it("非法 id 拒绝（与 Rust 相同的样例）", () => {
    for (const id of ["no_dot", "-a.b.c", "a..b", "com.X.y", ""]) {
      expect(validateManifest({ ...valid, id }).length).toBeGreaterThan(0);
    }
  });

  it("非法 entry 拒绝", () => {
    for (const entry of ["sub/main.js", "main.txt", "../main.js", ""]) {
      expect(validateManifest({ ...valid, entry }).length).toBeGreaterThan(0);
    }
  });

  it("非法 host / 未知事件 / 错误 cron 拒绝", () => {
    expect(
      validateManifest({ ...valid, permissions: { network: ["https://api.example.com"] } }).length,
    ).toBeGreaterThan(0);
    expect(
      validateManifest({ ...valid, permissions: { network: ["api.example.com:8443"] } }).length,
    ).toBeGreaterThan(0);
    expect(
      validateManifest({ ...valid, permissions: { events: ["task.deleted"] } }).length,
    ).toBeGreaterThan(0);
    expect(
      validateManifest({ ...valid, permissions: { cron: ["* * * *"] } }).length,
    ).toBeGreaterThan(0);
  });

  it("贡献点 id 重复 / 标题缺失拒绝", () => {
    expect(
      validateManifest({
        ...valid,
        contributions: {
          views: [
            { id: "a", title: "A" },
            { id: "a", title: "B" },
          ],
        },
      }).length,
    ).toBeGreaterThan(0);
    expect(
      validateManifest({
        ...valid,
        contributions: { commands: [{ id: "c", title: " " }] },
      }).length,
    ).toBeGreaterThan(0);
  });
});

describe("host 工具（与 Rust extract_host/validate_host 对齐）", () => {
  it("isValidHost", () => {
    expect(isValidHost("api.github.com")).toBe(true);
    expect(isValidHost("https://api.example.com")).toBe(false);
    expect(isValidHost("api.example.com:8443")).toBe(false);
    expect(isValidHost("localhost")).toBe(false);
  });

  it("extractHost", () => {
    expect(extractHost("https://API.GitHub.com/repos/x?q=1")).toBe("api.github.com");
    expect(extractHost("http://evil.com/x")).toBe("evil.com");
    expect(extractHost("ftp://api.github.com")).toBeNull();
    expect(extractHost("not a url")).toBeNull();
  });
});
