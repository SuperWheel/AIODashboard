// 番茄钟 0.2.0：完整周期（专注/短休/长休）+ 暂停继续 + 设置 + 圆环 UI。
//
// 状态权威仍是 KV 时间戳，不是组件内存：
// - 运行中：session.ends_at
// - 暂停中：session.paused + remaining_s
// - 宿主 registerInterval 负责到点结算；UI 的 1s interval 只刷新渲染

export async function onload(api) {
  const h = api.react.createElement;
  const { useState, useEffect } = api.react;

  const MODE = {
    focus: {
      label: "专注",
      color: "var(--accent)",
      soft: "color-mix(in srgb, var(--accent) 14%, transparent)",
      chip: "color-mix(in srgb, var(--accent) 18%, transparent)",
    },
    short_break: {
      label: "短休",
      color: "var(--info)",
      soft: "color-mix(in srgb, var(--info) 14%, transparent)",
      chip: "color-mix(in srgb, var(--info) 18%, transparent)",
    },
    long_break: {
      label: "长休",
      color: "var(--violet)",
      soft: "color-mix(in srgb, var(--violet) 16%, transparent)",
      chip: "color-mix(in srgb, var(--violet) 20%, transparent)",
    },
  };

  const DEFAULTS = {
    focus_minutes: 25,
    short_break_minutes: 5,
    long_break_minutes: 15,
    long_break_every: 4,
  };

  // ---------- 存储 ----------

  const kvGet = async (k, dflt = null) => {
    try {
      return (await api.storage.kv.get(k)) ?? dflt;
    } catch {
      return dflt;
    }
  };
  const kvSet = (k, v) => api.storage.kv.set(k, v);
  const kvDel = (k) => api.storage.kv.delete(k);

  const pad = (n) => String(n).padStart(2, "0");
  const todayKey = () => {
    const d = new Date();
    return `count:${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  };
  const todayMinKey = () => `${todayKey()}:min`;

  async function readPrefs() {
    let raw = {};
    try {
      raw = (await api.settings.get("preferences")) || {};
    } catch {
      raw = {};
    }
    const num = (k, min, max, dflt) => {
      const v = Number(raw[k]);
      if (!Number.isFinite(v) || v < min || v > max) return dflt;
      return Math.round(v);
    };
    return {
      focus_minutes: num("focus_minutes", 15, 60, DEFAULTS.focus_minutes),
      short_break_minutes: num("short_break_minutes", 3, 15, DEFAULTS.short_break_minutes),
      long_break_minutes: num("long_break_minutes", 10, 30, DEFAULTS.long_break_minutes),
      long_break_every: num("long_break_every", 2, 8, DEFAULTS.long_break_every),
    };
  }

  function durationOf(mode, prefs) {
    const m =
      mode === "focus"
        ? prefs.focus_minutes
        : mode === "short_break"
          ? prefs.short_break_minutes
          : prefs.long_break_minutes;
    return m * 60;
  }

  async function readSession() {
    const raw = await kvGet("session");
    if (!raw) return null;
    try {
      const s = typeof raw === "string" ? JSON.parse(raw) : raw;
      if (!s || !MODE[s.mode]) return null;
      return s;
    } catch {
      return null;
    }
  }

  async function writeSession(s) {
    await kvSet("session", JSON.stringify(s));
  }

  async function readCycle() {
    const n = Number(await kvGet("cycle_position", "0"));
    return Number.isFinite(n) && n >= 0 ? n : 0;
  }

  async function writeCycle(n) {
    await kvSet("cycle_position", String(n));
  }

  async function readTodayCount() {
    return Number((await kvGet(todayKey(), "0")) || 0);
  }

  async function readTodayMinutes() {
    return Number((await kvGet(todayMinKey(), "0")) || 0);
  }

  async function readTotal() {
    const entries = await api.storage.kv.list("count:");
    return entries.reduce((acc, e) => {
      if (e.key.endsWith(":min")) return acc;
      return acc + (Number(e.value) || 0);
    }, 0);
  }

  /** 旧版 0.1.x 只有 ends_at，首次加载时迁入 session 结构。 */
  async function migrate() {
    const oldEnds = await kvGet("ends_at");
    if (!oldEnds) return;
    const session = await readSession();
    await kvDel("ends_at");
    if (session) return;
    const rem = Math.max(0, Math.floor((new Date(oldEnds).getTime() - Date.now()) / 1000));
    if (rem > 0) {
      await writeSession({
        mode: "focus",
        ends_at: oldEnds,
        duration_s: rem,
        remaining_s: rem,
        paused: false,
      });
      api.log.info("已从旧版 ends_at 迁移会话");
    }
  }

  // ---------- 命令逻辑 ----------

  /** cycle = 当前块已完成的专注数；达到 long_break_every 后建议长休。 */
  function suggestedMode(cycle, longEvery) {
    return cycle >= longEvery ? "long_break" : "focus";
  }

  async function startMode(mode, { silent } = {}) {
    const prefs = await readPrefs();
    const duration = durationOf(mode, prefs);
    const ends = new Date(Date.now() + duration * 1000).toISOString();
    await writeSession({
      mode,
      ends_at: ends,
      duration_s: duration,
      remaining_s: duration,
      paused: false,
    });
    if (!silent) api.log.info(`番茄开始 · ${MODE[mode].label} ${duration / 60} 分钟`);
    notify();
    return true;
  }

  async function pause() {
    const s = await readSession();
    if (!s || s.paused) return false;
    const rem = Math.max(0, Math.floor((new Date(s.ends_at).getTime() - Date.now()) / 1000));
    await writeSession({ ...s, paused: true, remaining_s: rem, ends_at: null });
    api.log.info("番茄暂停");
    notify();
    return true;
  }

  async function resume() {
    const s = await readSession();
    if (!s || !s.paused) return false;
    const rem = Math.max(1, s.remaining_s || 0);
    const ends = new Date(Date.now() + rem * 1000).toISOString();
    await writeSession({ ...s, paused: false, ends_at: ends, remaining_s: rem });
    api.log.info("番茄继续");
    notify();
    return true;
  }

  async function togglePause() {
    const s = await readSession();
    if (!s) {
      const prefs = await readPrefs();
      return startMode(suggestedMode(await readCycle(), prefs.long_break_every));
    }
    return s.paused ? resume() : pause();
  }

  async function reset() {
    const had = (await readSession()) != null;
    await kvDel("session");
    if (had) api.log.info("番茄已重置");
    notify();
    return had;
  }

  /** 跳过：不计入完成数、不推进 cycle。 */
  async function skip() {
    const had = (await readSession()) != null;
    await kvDel("session");
    if (had) api.log.info("番茄已跳过");
    notify();
    return had;
  }

  async function completeSession(s) {
    if (s.mode === "focus") {
      const n = (await readTodayCount()) + 1;
      const mins = Math.max(0, Math.round((s.duration_s || 0) / 60));
      const todayMin = (await readTodayMinutes()) + mins;
      await kvSet(todayKey(), String(n));
      await kvSet(todayMinKey(), String(todayMin));
      await kvSet("last_completed_at", new Date().toISOString());
      const cycle = (await readCycle()) + 1;
      await writeCycle(cycle);
      const prefs = await readPrefs();
      const next = suggestedMode(cycle, prefs.long_break_every);
      api.log.info(`专注完成，今日第 ${n} 个 · 建议${MODE[next].label}`);
    } else if (s.mode === "long_break") {
      await writeCycle(0);
      api.log.info("长休结束，cycle 已归零");
    } else {
      api.log.info("短休结束");
    }
    await kvDel("session");
    notify();
  }

  async function maybeComplete() {
    const s = await readSession();
    if (!s || s.paused || !s.ends_at) return false;
    const rem = Math.floor((new Date(s.ends_at).getTime() - Date.now()) / 1000);
    if (rem > 0) return false;
    // session 仍在且已到点 → 本调用负责结算；completeSession 删除 session，天然幂等
    await completeSession(s);
    return true;
  }

  // ---------- 共享快照（UI + 宿主 interval 共用） ----------

  const listeners = new Set();
  const notify = () => listeners.forEach((fn) => fn());

  let snap = {
    status: "idle",
    mode: "focus",
    remaining: DEFAULTS.focus_minutes * 60,
    duration: DEFAULTS.focus_minutes * 60,
    cycle: 0,
    prefs: { ...DEFAULTS },
    todayCount: 0,
    todayMinutes: 0,
    total: 0,
  };

  async function refreshSnapshot() {
    const prefs = await readPrefs();
    const cycle = await readCycle();
    const s = await readSession();
    const extra = {
      prefs,
      cycle,
      todayCount: await readTodayCount(),
      todayMinutes: await readTodayMinutes(),
      total: await readTotal(),
    };
    if (!s) {
      const mode = suggestedMode(cycle, prefs.long_break_every);
      snap = {
        ...extra,
        status: "idle",
        mode,
        remaining: durationOf(mode, prefs),
        duration: durationOf(mode, prefs),
      };
      return;
    }
    if (s.paused) {
      snap = {
        ...extra,
        status: "paused",
        mode: s.mode,
        remaining: Math.max(0, s.remaining_s || 0),
        duration: s.duration_s || durationOf(s.mode, prefs),
      };
      return;
    }
    const rem = Math.max(0, Math.floor((new Date(s.ends_at).getTime() - Date.now()) / 1000));
    snap = {
      ...extra,
      status: "running",
      mode: s.mode,
      remaining: rem,
      duration: s.duration_s || durationOf(s.mode, prefs),
    };
  }

  async function tick() {
    await maybeComplete();
    await refreshSnapshot();
    notify();
  }

  // ---------- 设置 ----------

  api.ui.registerSettings({
    id: "preferences",
    title: "番茄钟设置",
    fields: [
      { key: "focus_minutes", label: "专注时长（分钟）", type: "number", min: 15, max: 60, default: DEFAULTS.focus_minutes },
      { key: "short_break_minutes", label: "短休时长（分钟）", type: "number", min: 3, max: 15, default: DEFAULTS.short_break_minutes },
      { key: "long_break_minutes", label: "长休时长（分钟）", type: "number", min: 10, max: 30, default: DEFAULTS.long_break_minutes },
      { key: "long_break_every", label: "长休间隔（专注次数）", type: "number", min: 2, max: 8, default: DEFAULTS.long_break_every },
    ],
  });

  // ---------- ⌘K 命令 ----------

  api.ui.registerCommand({ id: "start_focus", title: "开始专注", handler: () => startMode("focus") });
  api.ui.registerCommand({ id: "start_short", title: "开始短休", handler: () => startMode("short_break") });
  api.ui.registerCommand({ id: "start_long", title: "开始长休", handler: () => startMode("long_break") });
  api.ui.registerCommand({ id: "toggle_pause", title: "暂停 / 继续番茄钟", handler: togglePause });
  api.ui.registerCommand({ id: "skip", title: "跳过当前番茄", handler: skip });
  api.ui.registerCommand({ id: "reset", title: "重置番茄钟", handler: reset });

  // ---------- UI 部件 ----------

  const fmt = (s) => `${pad(Math.floor(s / 60))}:${pad(s % 60)}`;

  function ProgressRing({
    value,
    size = 120,
    stroke = 8,
    color = "var(--accent)",
    soft = "var(--surface-2)",
  }) {
    const r = (size - stroke) / 2;
    const c = 2 * Math.PI * r;
    const clamped = Math.min(1, Math.max(0, value));
    return h(
      "svg",
      {
        width: size,
        height: size,
        viewBox: `0 0 ${size} ${size}`,
        className: "shrink-0",
        role: "img",
        "aria-label": `进度 ${Math.round(clamped * 100)}%`,
      },
      h("circle", {
        cx: size / 2,
        cy: size / 2,
        r,
        fill: "none",
        stroke: soft,
        strokeWidth: stroke,
      }),
      h("circle", {
        cx: size / 2,
        cy: size / 2,
        r,
        fill: "none",
        stroke: color,
        strokeWidth: stroke,
        strokeLinecap: "round",
        strokeDasharray: c,
        strokeDashoffset: c * (1 - clamped),
        transform: `rotate(-90 ${size / 2} ${size / 2})`,
        style: { transition: "stroke-dashoffset 0.4s linear, stroke 0.2s ease" },
      }),
    );
  }

  function Btn({ label, onClick, variant = "ghost", className = "" }) {
    const base =
      "inline-flex items-center justify-center rounded-xl px-3.5 py-2 text-xs font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-40";
    const styles =
      variant === "primary"
        ? "bg-accent text-onaccent hover:bg-accent/90"
        : variant === "soft"
          ? "bg-hover text-ink hover:bg-surface2"
          : "border border-line text-ink2 hover:bg-hover hover:text-ink";
    return h(
      "button",
      {
        type: "button",
        className: `${base} ${styles} ${className}`,
        onClick: () => {
          Promise.resolve(onClick()).catch((e) => api.log.error(String(e?.message || e)));
        },
      },
      label,
    );
  }

  function ModeChip({ mode, active, onClick }) {
    const m = MODE[mode];
    return h(
      "button",
      {
        type: "button",
        onClick,
        className: `rounded-full px-3 py-1.5 text-xs font-medium transition-colors ${
          active ? "" : "text-ink3 hover:text-ink2"
        }`,
        style: {
          background: active ? m.chip : "transparent",
          boxShadow: active
            ? `inset 0 0 0 1px color-mix(in srgb, ${m.color} 35%, transparent)`
            : "none",
          color: active ? m.color : undefined,
        },
      },
      m.label,
    );
  }

  function usePomodoro() {
    const [, force] = useState(0);
    useEffect(() => {
      const listener = () => force((x) => x + 1);
      listeners.add(listener);
      tick().catch(() => {});
      const t = setInterval(() => {
        // 轻量本地刷新；到点结算由宿主 interval / tick 负责
        refreshSnapshot()
          .then(() => listener())
          .catch(() => {});
      }, 1000);
      return () => {
        listeners.delete(listener);
        clearInterval(t);
      };
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);
    return snap;
  }

  function progressOf(s) {
    if (!s.duration) return 0;
    return Math.min(1, Math.max(0, 1 - s.remaining / s.duration));
  }

  function statusLabel(s) {
    if (s.status === "running") return s.mode === "focus" ? "专注中…" : "休息中…";
    if (s.status === "paused") return "已暂停";
    return "准备开始";
  }

  function startSuggested() {
    return startMode(snap.mode);
  }

  function SuggestNote({ s }) {
    if (s.status !== "idle") return null;
    if (s.cycle >= s.prefs.long_break_every) {
      return h(
        "p",
        { className: "mt-1 text-[11px] text-ink3" },
        `已完成 ${s.cycle} 个专注 · 建议进入长休`,
      );
    }
    if (s.cycle > 0) {
      return h(
        "p",
        { className: "mt-1 text-[11px] text-ink3" },
        `本块已 ${s.cycle}/${s.prefs.long_break_every} 个专注 · 下一段仍建议短休休息后继续`,
      );
    }
    if (s.todayCount > 0) {
      return h(
        "p",
        { className: "mt-1 text-[11px] text-ink3" },
        `今日已完成 ${s.todayCount} 个 · 每 ${s.prefs.focus_minutes} 分钟一段`,
      );
    }
    return h(
      "p",
      { className: "mt-1 text-[11px] text-ink3" },
      `今日目标：每 ${s.prefs.focus_minutes} 分钟一次专注 · ${s.prefs.long_break_every} 个后长休`,
    );
  }

  function StatTile({ label, value, hint }) {
    return h(
      "div",
      { className: "rounded-2xl border border-line bg-surface shadow-card px-4 py-3" },
      h("div", { className: "text-[11px] text-ink3" }, label),
      h(
        "div",
        { className: "mt-0.5 font-mono text-2xl font-medium tabular-nums text-ink" },
        value,
      ),
      h("div", { className: "mt-0.5 text-[10px] text-ink3" }, hint),
    );
  }

  // ---------- Today 卡片 ----------

  api.ui.registerTodayCard({
    id: "card",
    title: "番茄钟",
    size: "md",
    component: () => {
      const s = usePomodoro();
      const theme = MODE[s.mode];
      const progress = progressOf(s);
      const dots = Array.from({ length: s.prefs.long_break_every }, (_, i) => i);
      return h(
        "div",
        {
          className:
            "flex h-full flex-col rounded-2xl border border-line bg-surface shadow-card px-4 py-3.5",
        },
        h(
          "div",
          { className: "flex items-start justify-between gap-2" },
          h(
            "div",
            null,
            h(
              "div",
              { className: "flex items-center gap-1.5" },
              h("span", { className: "text-sm", "aria-hidden": "true" }, "🍅"),
              h(
                "span",
                {
                  className: "rounded-full px-2 py-0.5 text-[11px] font-medium",
                  style: { background: theme.soft, color: theme.color },
                },
                theme.label,
              ),
              h("span", { className: "text-[11px] text-ink3" }, statusLabel(s)),
            ),
            h(
              "div",
              {
                className: "mt-1.5 font-mono text-3xl tabular-nums tracking-tight text-ink",
              },
              fmt(s.remaining),
            ),
          ),
          h(
            "div",
            { className: "-mt-0.5 -mr-1" },
            h(ProgressRing, {
              value: s.status === "idle" ? 0 : progress,
              size: 52,
              stroke: 5,
              color: theme.color,
              soft: "var(--line)",
            }),
          ),
        ),
        h(
          "div",
          { className: "mt-2 flex items-center gap-1" },
          dots.map((i) =>
            h("span", {
              key: i,
              className: "h-1.5 w-1.5 rounded-full",
              style: {
                background: i < s.cycle ? "var(--accent)" : "var(--line)",
              },
            }),
          ),
          h(
            "span",
            { className: "ml-1.5 text-[10px] text-ink3" },
            `${s.cycle}/${s.prefs.long_break_every} · 今日 ${s.todayCount}`,
          ),
        ),
        h(
          "div",
          { className: "mt-auto flex items-center gap-2 pt-3" },
          s.status === "running"
            ? h(Btn, { label: "暂停", onClick: pause, variant: "soft" })
            : s.status === "paused"
              ? h(Btn, { label: "继续", onClick: resume, variant: "primary" })
              : h(Btn, {
                  label: MODE[s.mode].label === "长休" ? "开始长休" : "开始专注",
                  onClick: startSuggested,
                  variant: "primary",
                }),
          s.status !== "idle" && h(Btn, { label: "结束", onClick: reset }),
        ),
      );
    },
  });

  // ---------- 独立视图 ----------

  api.ui.registerView({
    id: "view",
    title: "番茄钟",
    icon: "🍅",
    component: () => {
      const s = usePomodoro();
      const theme = MODE[s.mode];
      const progress = progressOf(s);
      const ringSize = 220;
      const dots = Array.from({ length: s.prefs.long_break_every }, (_, i) => i);

      const switchMode = (mode) => async () => {
        if (s.status !== "idle") await kvDel("session");
        await startMode(mode);
      };

      return h(
        "div",
        { className: "pb-8" },
        h(
          "div",
          { className: "flex flex-wrap items-end justify-between gap-3" },
          h(
            "div",
            null,
            h(
              "h1",
              { className: "text-xl font-semibold tracking-tight text-ink" },
              "番茄钟",
            ),
            h(
              "p",
              { className: "mt-0.5 text-xs text-ink3" },
              "以时间戳权威运行，关窗进托盘不走时",
            ),
          ),
          h(
            "div",
            {
              className: "flex items-center gap-1 rounded-full border border-line bg-surface p-1",
            },
            h(ModeChip, {
              mode: "focus",
              active: s.mode === "focus" && s.status !== "idle",
              onClick: switchMode("focus"),
            }),
            h(ModeChip, {
              mode: "short_break",
              active: s.mode === "short_break" && s.status !== "idle",
              onClick: switchMode("short_break"),
            }),
            h(ModeChip, {
              mode: "long_break",
              active: s.mode === "long_break" && s.status !== "idle",
              onClick: switchMode("long_break"),
            }),
          ),
        ),

        h(
          "div",
          {
            className:
              "mt-6 flex flex-col items-center rounded-3xl border border-line bg-surface px-6 py-10 shadow-card",
            style: {
              background: `linear-gradient(180deg, ${theme.soft} 0%, var(--surface) 48%)`,
            },
          },
          h(
            "div",
            { className: "relative flex items-center justify-center" },
            h(ProgressRing, {
              value: s.status === "idle" ? 0 : progress,
              size: ringSize,
              stroke: 10,
              color: theme.color,
              soft: "color-mix(in srgb, var(--ink) 8%, transparent)",
            }),
            h(
              "div",
              { className: "absolute inset-0 flex flex-col items-center justify-center" },
              h(
                "div",
                {
                  className:
                    "font-mono text-5xl font-medium tabular-nums tracking-tight",
                  style: {
                    color: s.status === "idle" ? "var(--ink-3)" : theme.color,
                  },
                },
                fmt(s.remaining),
              ),
              h(
                "div",
                { className: "mt-1.5 text-xs font-medium", style: { color: theme.color } },
                theme.label,
              ),
              h("div", { className: "text-[11px] text-ink3" }, statusLabel(s)),
            ),
          ),

          h(
            "div",
            { className: "mt-6 flex items-center gap-2" },
            dots.map((i) =>
              h(
                "span",
                {
                  key: i,
                  className:
                    "flex h-7 w-7 items-center justify-center rounded-full text-[11px] font-medium transition-colors",
                  style: {
                    background:
                      i < s.cycle
                        ? "color-mix(in srgb, var(--accent) 20%, transparent)"
                        : "var(--surface-2)",
                    color: i < s.cycle ? "var(--accent)" : "var(--ink-3)",
                    boxShadow:
                      i < s.cycle
                        ? "inset 0 0 0 1px color-mix(in srgb, var(--accent) 35%, transparent)"
                        : "none",
                  },
                },
                String(i + 1),
              ),
            ),
            h(
              "span",
              { className: "ml-2 text-[11px] text-ink3" },
              `每 ${s.prefs.long_break_every} 个专注进长休`,
            ),
          ),

          h(SuggestNote, { s }),

          h(
            "div",
            { className: "mt-6 flex flex-wrap items-center justify-center gap-2" },
            s.status === "running"
              ? h(Btn, {
                  label: "暂停",
                  onClick: pause,
                  variant: "soft",
                  className: "px-5 py-2.5 text-sm",
                })
              : s.status === "paused"
                ? h(Btn, {
                    label: "继续",
                    onClick: resume,
                    variant: "primary",
                    className: "px-5 py-2.5 text-sm",
                  })
                : h(Btn, {
                    label: MODE[s.mode].label === "长休" ? "开始长休" : "开始专注",
                    onClick: startSuggested,
                    variant: "primary",
                    className: "px-5 py-2.5 text-sm",
                  }),
            s.status !== "idle" &&
              h(Btn, {
                label: "跳过",
                onClick: skip,
                className: "px-4 py-2.5 text-sm",
              }),
            s.status !== "idle" &&
              h(Btn, {
                label: "重置",
                onClick: reset,
                className: "px-4 py-2.5 text-sm",
              }),
          ),
        ),

        h(
          "div",
          { className: "mt-4 grid grid-cols-2 gap-3 sm:grid-cols-4" },
          h(StatTile, {
            label: "今日完成",
            value: String(s.todayCount),
            hint: "专注番茄",
          }),
          h(StatTile, {
            label: "今日专注",
            value: String(s.todayMinutes),
            hint: "分钟",
          }),
          h(StatTile, {
            label: "累计完成",
            value: String(s.total),
            hint: "历史累计",
          }),
          h(StatTile, {
            label: "专注时长",
            value: `${s.prefs.focus_minutes}`,
            hint: "分钟 / 次",
          }),
        ),
      );
    },
  });

  // ---------- 启动 ----------

  await migrate();
  await tick();
  // 宿主托管定时器：停用自动清理；负责到点结算，不依赖 UI 是否打开
  api.registerInterval(tick, 1000);
  api.log.info("pomodoro 0.2.0 onload 完成");
}

export async function onunload() {
  // 贡献点、订阅与 interval 由宿主清理
}
