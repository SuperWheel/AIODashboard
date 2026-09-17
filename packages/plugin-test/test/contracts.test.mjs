import { test } from "node:test";
import assert from "node:assert/strict";
import { createPluginTestContext } from "../dist/index.js";
const manifest = {
  id: "com.test.a",
  name: "A",
  version: "1.0.0",
  entry: "main.js",
  api_version: "plugin.protocol/v2",
  permissions: { ui: ["command", "settings"], storage_quota_bytes: 100 },
  contributions: {
    commands: [{ id: "c", title: "C" }],
    settings: [{ id: "s", title: "S" }],
  },
};
test("default deny, quota UTF-8, namespaces and disposal", async () => {
  const ctx = createPluginTestContext(manifest);
  await assert.rejects(ctx.api.core.createTask("x"), {
    code: "permission_denied",
  });
  await ctx.api.storage.kv.set("中", "a".repeat(97));
  await assert.rejects(ctx.api.storage.kv.set("x", "x"));
  await ctx.api.storage.kv.set("中", "a");
  let fired = 0;
  const off = ctx.api.events.on("plugin.com.test.a:changed", () => fired++);
  ctx.api.events.emit("plugin.com.test.a:changed", null);
  assert.equal(fired, 1);
  off();
  assert.throws(() =>
    ctx.api.events.on("plugin.com.test.a.child:changed", () => {}),
  );
  assert.throws(() => ctx.api.events.emit("task.created", {}));
  ctx.api.ui.registerCommand({ id: "c", title: "C", handler: () => {} });
  ctx.dispose();
  assert.equal(ctx.commands.size, 0);
  assert.equal(ctx.subscriptions.size, 0);
  await assert.rejects(ctx.api.storage.kv.get("中"));
  const reopened = createPluginTestContext(manifest, { kv: ctx.kv });
  assert.equal(await reopened.api.storage.kv.get("中"), "a");
  reopened.dispose();
});
test("settings persist and validate", async () => {
  const ctx = createPluginTestContext(manifest);
  ctx.api.ui.registerSettings({
    id: "s",
    title: "S",
    fields: [
      {
        key: "count",
        label: "Count",
        type: "number",
        default: 1,
        min: 1,
        max: 4,
      },
    ],
  });
  assert.deepEqual(await ctx.api.settings.get("s"), { count: 1 });
  await ctx.api.settings.set("s", { count: 3 });
  assert.deepEqual(await ctx.api.settings.get("s"), { count: 3 });
  await assert.rejects(ctx.api.settings.set("s", { count: 99 }));
  ctx.dispose();
  assert.equal(ctx.settings.size, 0);
});

test("Rust null quota grants no storage while declared UI remains usable", async () => {
  const ctx = createPluginTestContext({
    ...manifest,
    permissions: { ui: ["command"], storage_quota_bytes: null },
  });
  ctx.api.ui.registerCommand({ id: "c", title: "C", handler: () => {} });
  assert.equal(ctx.commands.size, 1);
  await assert.rejects(ctx.api.storage.kv.get("secret"), {
    code: "permission_denied",
  });
  ctx.dispose();
});
