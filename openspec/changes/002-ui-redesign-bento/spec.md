# Spec 002: UI 重设计 · Bento 总控台

> 验收证据见 tasks.md；token 选型与实现方式见 design.md。

## ADDED

### ADDED-1：语义 token 体系

- **要求**：全部颜色走语义 token（`--color-bg / surface / surface-2 / line / ink / ink-2 / accent / danger / warn / info`），组件零写死色值、零 `dark:` 前缀；圆角卡片 `rounded-2xl`、控件 `rounded-lg`、徽章 `rounded-full`；统计数字一律 `tabular-nums`。
- **验收**：全量替换完成后，代码中不存在视图层写死色值。

### ADDED-2：主题三态

- **要求**：跟随系统 / 深色 / 浅色三态切换（Sidebar 底部循环切换），存 localStorage；浅色 `:root` 默认 / `[data-theme="dark"]` / 系统深色媒体查询三选择器实现变量翻转；index.html 内联脚本首帧前还原防闪烁；原生控件配色由根 `color-scheme` 驱动。
- **验收**：三态切换即时生效、刷新不闪错主题；双主题对比度 ≥ WCAG AA。

### ADDED-3：新组件

- **要求**：`ProgressRing`（SVG 圆环今日完成率，放问候卡）；`PageHeader`（子页统一页头：标题 + 数量徽标 + 主操作）；`QuickCapture`（常驻输入卡：今天到期 / 收件箱双去向，Enter 即走，「今天」用本地时区 `hooks.localToday()`）。
- **要求**：微动效——卡片 hover 上浮（translateY(-1px) + 阴影加深）、打勾回弹，一律 ≤200ms 且尊重 `prefers-reduced-motion`；所有交互元素有 focus ring。
- **验收**：真机走查动效与可访问性。

## MODIFIED

- **首页 Today → Bento 网格**：容器 `max-w-3xl` → `max-w-6xl`，12 列网格（问候+进度环 span 5 / 快速捕捉 span 4 / 插件槽 span 3；今日任务 span 8 含逾期红边置顶分组 / 统计 2×2 迷你卡 span 4；最近笔记、活跃项目、插件槽 2 各 span 4）；`<lg` 断点堆叠回单列（顺序：任务 > 捕捉 > 插件 > 统计 > 其他）。
- **插件 Today 卡片协议**：`registerTodayCard` 新增可选 `size: "sm" | "md" | "lg"`（sm=3 列 / md=4 列 / lg=6 列，缺省 md）——bridge 运行时校验 + registry 透传 + PLUGIN_API.md + 示例插件四方同步（向后兼容，可选字段）。
- **子页（工作室）**：保持 `max-w-3xl` 单列；统一 PageHeader；列表行 hover 浮现操作；空态 = 大号 glyph + 引导语 + 主按钮。
- **Sidebar**：毛玻璃 `backdrop-blur`，文字图标升级 lucide-react 线性图标（仅核心视图用，插件视图仍用 manifest 字符图标，协议不变），底部主题切换。
- **StatCard**：重构为迷你卡（数值 + 环比昨日箭头占位），2×2 收进侧栏格，可点击跳对应视图。

## REMOVED

- 无。现有功能全部保留，仅视觉与布局重构。

## 非本变更范围（明确不做）

- 三栏式列表/详情布局（任务详情面板化）——后续单独立项。
- 视图自定义排序/拖拽布局——等插件生态起来后再评估。
- 移动端适配——macOS 桌面优先不变。
