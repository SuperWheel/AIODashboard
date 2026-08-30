// 番茄钟插件：演示插件系统的完整能力面。
//
// 设计要点：
// - 运行状态 = kv 里的 `ends_at` 时间戳（而非本地内存），重启面板/托盘后台后状态不丢、不走时
// - 倒计时展示用 1s interval 只是"渲染"，正确性完全取决于时间戳比较
// - 完成任务（task.completed）时空闲则自动开始一个番茄钟（manifest 已声明 events 权限）

const WORK_SECONDS = 25 * 60;

export async function onload(api) {
  const h = api.react.createElement;

  const get = async (k, dflt = null) => (await api.storage.kv.get(k)) ?? dflt;
  const todayKey = () => `count:${new Date().toISOString().slice(0, 10)}`;

  async function isRunning() {
    const ends = await api.storage.kv.get("ends_at");
    return ends != null && new Date(ends).getTime() > Date.now();
  }

  async function start() {
    if (await isRunning()) return false;
    await api.storage.kv.set("ends_at", new Date(Date.now() + WORK_SECONDS * 1000).toISOString());
    api.log.info("番茄钟开始");
    return true;
  }

  async function stop() {
    const had = (await api.storage.kv.get("ends_at")) != null;
    await api.storage.kv.delete("ends_at");
    if (had) api.log.info("番茄钟放弃");
    return had;
  }

  // 到点：清状态 + 今日计数 +1
  async function expire() {
    await api.storage.kv.delete("ends_at");
    const n = Number(await get(todayKey(), "0")) + 1;
    await api.storage.kv.set(todayKey(), String(n));
    await api.storage.kv.set("last_completed_at", new Date().toISOString());
    api.log.info(`番茄完成，今日第 ${n} 个`);
  }

  // 事件联动：完成任务 → 空闲时自动开始（休息比硬拼更符合番茄工作法，这里选择直接开始下一个）
  api.events.on("task.completed", async () => {
    if (!(await isRunning())) {
      await start();
    }
  });

  // ⌘K 命令
  api.ui.registerCommand({ id: "start", title: "开始一个番茄钟", handler: start });
  api.ui.registerCommand({ id: "stop", title: "放弃当前番茄钟", handler: stop });

  // 倒计时 Hook：每秒读 kv 里的 ends_at（时间戳权威，interval 只管刷新渲染）
  function usePomodoro() {
    const [st, setSt] = api.react.useState({ running: false, remaining: 0, expired: false });
    api.react.useEffect(() => {
      let alive = true;
      let lastEnds = null;
      const tick = async () => {
        const ends = await api.storage.kv.get("ends_at");
        if (!alive) return;
        if (ends == null) {
          setSt({ running: false, remaining: 0, expired: false });
          return;
        }
        const rem = Math.max(0, Math.floor((new Date(ends).getTime() - Date.now()) / 1000));
        setSt({ running: true, remaining: rem, expired: rem === 0 });
        // ends_at 值变化才触发一次 expire，避免重复计数
        if (rem === 0 && ends !== lastEnds) {
          lastEnds = ends;
          await expire();
        }
      };
      tick();
      const t = setInterval(tick, 1000);
      return () => {
        alive = false;
        clearInterval(t);
      };
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);
    return st;
  }

  const fmt = (s) =>
    `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;

  const Btn = (props) =>
    h(
      "button",
      {
        onClick: () => {
          Promise.resolve(props.onClick()).then(props.after);
        },
        className: `rounded-lg px-3 py-1.5 text-xs font-medium transition-colors ${props.cls ?? "bg-white/5 text-slate-300 hover:bg-white/10"}`,
      },
      props.label,
    );

  const Controls = (props) =>
    h(
      "div",
      { className: "mt-3 flex items-center gap-2" },
      props.running
        ? h(Btn, { key: "stop", label: "放弃", onClick: stop, after: props.refresh })
        : h(Btn, {
            key: "start",
            label: "开始专注",
            onClick: start,
            after: props.refresh,
            cls: "bg-emerald-500/90 text-[#0f1115] hover:bg-emerald-400",
          }),
    );

  // Today 卡片
  api.ui.registerTodayCard({
    id: "card",
    title: "番茄钟",
    component: (props) => {
      const st = usePomodoro();
      const [todayCount, setTodayCount] = api.react.useState("-");
      api.react.useEffect(() => {
        get(todayKey(), "0").then((v) => setTodayCount(v));
      }, [st.expired, props.today?.date]);
      return h(
        "div",
        { className: "rounded-xl border border-white/10 bg-[#161a22] px-4 py-3" },
        h(
          "div",
          { className: "flex items-center justify-between" },
          h("div", { className: "text-xs font-medium text-emerald-300" }, "番茄钟"),
          h("div", { className: "text-[10px] text-slate-500" }, `今日完成 ${todayCount}`),
        ),
        h(
          "div",
          { className: "mt-1 font-mono text-2xl tabular-nums", style: { color: st.running ? "#e2e8f0" : "#475569" } },
          st.running ? (st.expired ? "完成！" : fmt(st.remaining)) : "25:00",
        ),
        h(Controls, { running: st.running, refresh: props.onChanged }),
      );
    },
  });

  // 独立视图
  api.ui.registerView({
    id: "view",
    title: "番茄钟",
    icon: "🍅",
    component: (props) => {
      const st = usePomodoro();
      const [todayCount, setTodayCount] = api.react.useState("0");
      const [total, setTotal] = api.react.useState("0");
      api.react.useEffect(() => {
        get(todayKey(), "0").then((v) => setTodayCount(v));
        // 历史总数 = 所有 count:* 键之和
        api.storage.kv.list("count:").then((entries) => {
          setTotal(String(entries.reduce((acc, e) => acc + Number(e.value || 0), 0)));
        });
      }, [st.expired, props.refreshKey]);
      return h(
        "div",
        null,
        h(
          "div",
          { className: "flex items-baseline justify-between" },
          h("h1", { className: "text-xl font-semibold text-white" }, "番茄钟"),
          h("span", { className: "text-xs text-slate-500" }, "完成一个任务会自动开始专注"),
        ),
        h(
          "div",
          { className: "mt-8 flex flex-col items-center rounded-2xl border border-white/10 bg-[#161a22] py-10" },
          h(
            "div",
            {
              className: "font-mono text-6xl tabular-nums",
              style: { color: st.running ? "#34d399" : "#475569" },
            },
            st.running ? (st.expired ? "完成！" : fmt(st.remaining)) : "25:00",
          ),
          h(
            "div",
            { className: "mt-2 text-xs text-slate-500" },
            st.running ? "专注中…" : "空闲",
          ),
          h(Controls, { running: st.running, refresh: props.onChanged }),
        ),
        h(
          "div",
          { className: "mt-4 grid grid-cols-2 gap-3 text-center" },
          h(
            "div",
            { className: "rounded-xl border border-white/10 bg-[#161a22] px-4 py-3" },
            h("div", { className: "text-2xl font-semibold text-white" }, todayCount),
            h("div", { className: "mt-0.5 text-[11px] text-slate-500" }, "今日完成"),
          ),
          h(
            "div",
            { className: "rounded-xl border border-white/10 bg-[#161a22] px-4 py-3" },
            h("div", { className: "text-2xl font-semibold text-white" }, total),
            h("div", { className: "mt-0.5 text-[11px] text-slate-500" }, "累计完成"),
          ),
        ),
      );
    },
  });

  api.log.info("pomodoro onload 完成");
}

export async function onunload() {
  // 贡献点与订阅由宿主清理
}
