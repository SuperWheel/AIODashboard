# Design 002: UI 重设计 · 关键设计决策

## 决策 1：语义 token 而非 `dark:` 变体

- **选择**：`@theme inline` 把 `--color-bg/surface/ink/accent…` 映射到运行时 CSS 变量；双主题 = 变量翻转（浅色 `:root` 默认 / `[data-theme="dark"]` / 系统深色媒体查询三选择器）。
- **理由**：组件零 `dark:` 前缀，插件 UI 天然跟随主题——这是「插件只用语义类名」规范成立的基础。
- **后果**：主题切换只改变量，组件无感知；新增视图不需要考虑双主题适配。

## 决策 2：主题三态存 localStorage + 首帧防闪烁

- **选择**：system/light/dark 三态；index.html 内联脚本首帧前还原；system 态不设属性、交给媒体查询；原生控件配色由根 `color-scheme` 统一驱动。
- **理由**：防刷新白闪；`color-scheme` 替掉日期输入框上的 `[color-scheme:dark]` 补丁。
- **后果**：设置暂存 localStorage，后续可落库（留待设置页扩展）。

## 决策 3：插件卡片 size 协议（sm/md/lg）

- **选择**：`registerTodayCard` 新增可选 `size`（sm=3 列 / md=4 列 / lg=6 列，缺省 md），bridge 运行时校验、registry 透传。
- **理由**：插件卡片进 Bento 网格需要一等格位；可选字段向后兼容。
- **后果**：协议变更——按「文档即协议」纪律同步 PLUGIN_API.md + bridge/测试 + 示例插件三方（echo/pomodoro 声明 `sm`）。

## 决策 4：首页信息架构 = 今日任务为主

- **选择**：今日任务最大卡（span 8），逾期任务红色横带置顶分组，未完成任务排在插件卡之前；快速捕捉独立成卡常驻右上。
- **理由**：首页回答「今天做什么」；收集与浏览不打断。

## 决策 5：lucide-react 仅核心视图使用

- **选择**：Sidebar 核心视图图标升级 lucide-react；插件视图仍用 manifest 字符图标。
- **理由**：不扩大插件协议面（图标库不进插件 API）。

## 决策 6：「今天」统一本地时区（顺修）

- **选择**：新增 `hooks.localToday()`，QuickCapture / 番茄钟统一使用。
- **理由**：原 `toISOString().slice(0,10)` 是 UTC 日期，跨午夜偏一天。

## 红线核对

| 红线 | 落实 |
|---|---|
| SQL / core / DB 红线 | 纯前端变更，不触碰 storage/core |
| 变更留 activity log | UI 变更不产生业务写操作，无需新增 |
| 协议变更纪律 | size 字段同步 PLUGIN_API.md + bridge/测试 + 示例三方 |
| 迁移只增不改 | 无 schema 变更 |
| 时间边界唯一入口 | localToday 仅前端展示层格式化，不另立日期边界逻辑 |

## 风险与权衡

- 对比度：双主题均按 WCAG AA 校准（真机走查确认）。
- 性能：微动效 ≤200ms 且尊重 `prefers-reduced-motion`；毛玻璃仅 Sidebar 一处。
