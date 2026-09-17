import { describe, expect, it, vi } from "vitest";
import { PluginImportFlow } from "../import";
import type { PluginImportPreview, PluginInstallResult } from "../types";

const preview = (id = "com.test.p"): PluginImportPreview => ({
  manifest: {
    id,
    name: "测试插件",
    version: "1.0.0",
    entry: "main.js",
    api_version: "plugin.protocol/v2",
  },
  source: "zip",
  source_path: "/tmp/plugin.zip",
  replacing: false,
  current_version: null,
  current_enabled: false,
  check: {
    id,
    content_sha256: "hash",
    archive_sha256: "zip-hash",
    target_stamp: "target",
  },
});
const result: PluginInstallResult = {
  id: "com.test.p",
  version: "1.0.0",
  sha256: "zip-hash",
  content_sha256: "hash",
  enabled: false,
};
const deferred = <T>() => {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((a, b) => {
    resolve = a;
    reject = b;
  });
  return { promise, resolve, reject };
};
const fixture = () => {
  const gateway = {
    pluginPickImportSource: vi.fn(
      async (): Promise<string | null> => "/tmp/plugin.zip",
    ),
    pluginPreviewImport: vi.fn(async () => preview()),
    pluginImport: vi.fn(async () => result),
  };
  const changed = vi.fn();
  return { gateway, changed, flow: new PluginImportFlow(gateway, changed) };
};

describe("插件导入交互", () => {
  it("取消系统选择器不预检或安装，关闭弹窗也使迟到结果失效", async () => {
    const { flow, gateway } = fixture();
    gateway.pluginPickImportSource.mockResolvedValueOnce(null);
    await flow.choose("zip");
    expect(flow.getSnapshot().phase).toBe("idle");
    expect(gateway.pluginPreviewImport).not.toHaveBeenCalled();
    const slow = deferred<PluginImportPreview>();
    gateway.pluginPreviewImport.mockReturnValueOnce(slow.promise);
    const choosing = flow.choose("directory");
    await Promise.resolve();
    expect(flow.cancel()).toBe(true);
    slow.resolve(preview());
    await choosing;
    await flow.commit();
    expect(flow.getSnapshot().phase).toBe("idle");
    expect(gateway.pluginImport).not.toHaveBeenCalled();
  });

  it("新选择使旧预检失效，较早的结果不能覆盖新插件", async () => {
    const { flow, gateway } = fixture();
    const slow = deferred<PluginImportPreview>();
    gateway.pluginPreviewImport.mockReturnValueOnce(slow.promise);
    const first = flow.choose("zip");
    await Promise.resolve();
    gateway.pluginPickImportSource.mockResolvedValueOnce("/tmp/new-plugin");
    gateway.pluginPreviewImport.mockResolvedValueOnce(preview("com.test.new"));
    await flow.choose("directory");
    slow.resolve(preview("com.test.old"));
    await first;
    expect(flow.getSnapshot().preview?.manifest.id).toBe("com.test.new");
    await flow.commit();
    expect(gateway.pluginImport).toHaveBeenCalledWith(
      "/tmp/new-plugin",
      preview("com.test.new").check,
    );
  });

  it("重复点击不重复安装，安装中不能取消或切换来源", async () => {
    const { flow, gateway, changed } = fixture();
    await flow.choose("zip");
    const pending = deferred<PluginInstallResult>();
    gateway.pluginImport.mockReturnValueOnce(pending.promise);
    const committing = flow.commit();
    await flow.commit();
    expect(flow.cancel()).toBe(false);
    await flow.choose("directory");
    expect(gateway.pluginPickImportSource).toHaveBeenCalledTimes(1);
    expect(gateway.pluginImport).toHaveBeenCalledTimes(1);
    pending.resolve(result);
    await committing;
    expect(flow.getSnapshot().phase).toBe("success");
    expect(flow.getSnapshot().result?.enabled).toBe(false);
    expect(changed).toHaveBeenCalledExactlyOnceWith(result);
  });

  it("安装失败保留来源和错误，重新检查后才能再次确认", async () => {
    const { flow, gateway, changed } = fixture();
    await flow.choose("zip");
    gateway.pluginImport.mockRejectedValueOnce("来源内容已变化");
    await flow.commit();
    expect(flow.getSnapshot()).toMatchObject({
      phase: "error",
      source: "/tmp/plugin.zip",
      preview: null,
      error: "来源内容已变化",
    });
    await flow.commit();
    expect(gateway.pluginImport).toHaveBeenCalledTimes(1);
    expect(changed).not.toHaveBeenCalled();
    const revised = preview();
    revised.check.content_sha256 = "new-hash";
    gateway.pluginPreviewImport.mockResolvedValueOnce(revised);
    await flow.recheck();
    expect(gateway.pluginImport).toHaveBeenCalledTimes(1);
    await flow.commit();
    expect(gateway.pluginImport).toHaveBeenLastCalledWith(
      "/tmp/plugin.zip",
      revised.check,
    );
    expect(changed).toHaveBeenCalledOnce();
  });

  it("预检错误可重试，重新选择时清除先前可提交的信息", async () => {
    const { flow, gateway } = fixture();
    gateway.pluginPreviewImport.mockRejectedValueOnce("缺少 manifest");
    await flow.choose("directory");
    expect(flow.getSnapshot().error).toBe("缺少 manifest");
    await flow.recheck();
    expect(flow.getSnapshot().phase).toBe("ready");
    const picking = deferred<string | null>();
    gateway.pluginPickImportSource.mockReturnValueOnce(picking.promise);
    const choosing = flow.choose("zip");
    expect(flow.getSnapshot().preview).toBeNull();
    await flow.commit();
    expect(gateway.pluginImport).not.toHaveBeenCalled();
    picking.resolve(null);
    await choosing;
  });
});
