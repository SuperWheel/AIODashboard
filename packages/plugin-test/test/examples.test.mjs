import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createPluginTestContext } from "../dist/index.js";
for (const id of ["com.leeyl.echo", "com.leeyl.pomodoro", "com.leeyl.network", "com.leeyl.job-tracker-trial"])
  test(`${id}: load/dispose`, async () => {
    const root = new URL(`../../../examples/plugins/${id}/`, import.meta.url);
    const manifest = JSON.parse(
      await readFile(new URL("manifest.json", root), "utf8"),
    );
    const mod = await import(new URL("main.js", root));
    const ctx = createPluginTestContext(manifest, {
      responses: {
        "https://api.github.com/": { status: 200, text: "{}", json: {} },
      },
    });
    await mod.onload(ctx.api);
    assert.ok(ctx.cards.size + ctx.commands.size + ctx.views.size > 0);
    if (id === "com.leeyl.network") {
      await ctx.commands.get("fetch")();
      assert.deepEqual(await ctx.api.settings.get("preferences"), {
        show_status: true,
      });
    }
    ctx.dispose();
    await mod.onunload?.();
    assert.equal(
      ctx.cards.size + ctx.commands.size + ctx.views.size + ctx.settings.size,
      0,
    );
  });
