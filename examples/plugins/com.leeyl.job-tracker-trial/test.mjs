import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createPluginTestContext } from "../../../packages/plugin-test/dist/index.js";
import * as plugin from "./main.js";

const manifest = JSON.parse(await readFile(new URL("manifest.json", import.meta.url), "utf8"));

test("trial registers both native entry points without reading or writing host data", async () => {
  const ctx = createPluginTestContext(manifest);
  assert.deepEqual(manifest.permissions, { ui: ["view", "today_card"] });
  // SDK denied calls would reject onload or appear in denied; no DOM is needed yet.
  await plugin.onload(ctx.api);
  assert.deepEqual([...ctx.views], ["view"]);
  assert.deepEqual([...ctx.cards], ["card"]);
  assert.equal(ctx.kv.size, 0);
  assert.equal(ctx.emitted.length, 0);
  assert.equal(ctx.denied.length, 0);
  ctx.dispose();
  await plugin.onunload();
  await plugin.onunload();
  assert.equal(ctx.views.size + ctx.cards.size, 0);
});

test("registration failure cleans the already registered view", async () => {
  let views = 0;
  const ctx = createPluginTestContext(manifest);
  await assert.rejects(plugin.onload({
    ...ctx.api,
    ui: {
      ...ctx.api.ui,
      registerView: () => { views++; return () => { views--; }; },
      registerTodayCard: () => { throw new Error("模拟注册冲突"); },
    },
  }), /模拟注册冲突/);
  assert.equal(views, 0);
  await plugin.onunload();
  ctx.dispose();
});
