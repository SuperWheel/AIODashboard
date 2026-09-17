import { afterEach, describe, expect, it, vi } from "vitest";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
import { invoke } from "@tauri-apps/api/core";
import { api } from "../../api";
import { pluginEvents } from "../events";

afterEach(() => vi.clearAllMocks());
describe("插件 IPC 拒绝的统一事件", () => {
  it.each([
    ["open", () => api.pluginOpenContext("com.test.p")],
    ["source", () => api.pluginLoadSource("revoked-token")],
    ["call", () => api.pluginCall("revoked-token", "today")],
  ] as const)("%s 拒绝保留结构化错误并产生事件", async (_, call) => {
    const fault = { code: "permission_denied", message: "上下文已撤销" };
    vi.mocked(invoke).mockRejectedValueOnce(fault);
    const listener = vi.fn();
    const off = pluginEvents.on("plugin.denied", "test", listener);
    try {
      await expect(call()).rejects.toMatchObject(fault);
      expect(listener).toHaveBeenCalledExactlyOnceWith(fault);
    } finally {
      off();
    }
  });
});

describe("插件导入 IPC 契约", () => {
  it("保留嵌套 snake_case 检查凭据，GUI 不传入 actor", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    const check = { id: "com.test.p", content_sha256: "content", archive_sha256: null, target_stamp: "target" };
    await api.pluginPickImportSource("directory");
    expect(invoke).toHaveBeenLastCalledWith("plugin_pick_import_source", { kind: "directory" });
    await api.pluginPreviewImport("/tmp/com.test.p");
    expect(invoke).toHaveBeenLastCalledWith("plugin_preview_import", { source: "/tmp/com.test.p" });
    await api.pluginImport("/tmp/com.test.p", check);
    expect(invoke).toHaveBeenLastCalledWith("plugin_import", { source: "/tmp/com.test.p", check });
  });
});
