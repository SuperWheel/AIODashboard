// Echo 示例插件：演示 onload / KV 存储 / Today 卡片注册的最小闭环。
//
// 插件入口约定：
// - export async function onload(api) —— 启用时调用一次
// - export async function onunload()  —— 停用时调用（可选）
// - api.react.createElement / api.react.useState 可构建任意 React UI

export async function onload(api) {
  // 1. KV：按插件命名空间隔离，跨重启持久
  const prev = Number((await api.storage.kv.get("loads")) ?? "0");
  const loads = prev + 1;
  await api.storage.kv.set("loads", String(loads));

  // 2. UI：注册 Today 页卡片
  const h = api.react.createElement;
  api.ui.registerTodayCard({
    id: "echo-card",
    title: "Echo",
    size: "sm",
    component: (props) =>
      h(
        "div",
        { className: "rounded-2xl border border-line bg-surface shadow-card h-full px-4 py-3" },
        h("div", { className: "text-xs font-medium text-accent" }, "Echo 插件运行中"),
        h(
          "div",
          { className: "mt-1 text-xs text-ink2" },
          `第 ${loads} 次加载 · 今天是 ${props.today?.date ?? "?"}`,
        ),
      ),
  });

  api.log.info(`onload 完成（第 ${loads} 次）`);
}

export async function onunload() {
  // 注册项与事件订阅由宿主自动清理，无需手动反注册
}
