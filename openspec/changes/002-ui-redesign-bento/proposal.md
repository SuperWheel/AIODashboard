# Change Proposal 002: UI 重设计 · Bento 总控台（ui-redesign/2026-08）

> 状态：**已完成（2026-08-30，切片 1–5 全部落地；遗留项已立 plan）** · 模式：Spec · 关联：openspec/changes/001-plugin-system（Today 卡片 size 协议）· docs/plans/2026-08-30-statcard-day-delta.md（遗留）

## Why（为什么做）

默认样式不支撑「All-in-One 总控台」定位；插件卡片需要与核心模块平起平坐的一等格位。重设计同时解决：双主题基建（插件 UI 天然跟随）、首页信息架构（今日任务为主）、组件库统一（语义 token，杜绝写死色值）。

## What I Want（要什么）

与用户对齐的四项偏好（确认记录）：
1. 风格 C——**Bento 总控台**：首页 = 卡片网格总控台，子页 = 单列工作室。
2. 布局：首页 12 列网格可跨格，子页保持单列居中阅读流。
3. 主题：**跟随系统自动双主题**（system/light/dark 三态）。
4. 首页重点（全选）：今日任务为主 / 插件卡片舞台 / 快速捕捉 / 统计可视化。

配套：语义 token 体系、新组件（ProgressRing / PageHeader / QuickCapture）、微动效、可访问性（WCAG AA + focus ring + reduced-motion）。

## What I Know（已知）

- Tailwind 4 CSS-first（`@theme` / `@custom-variant`）可用变量翻转实现双主题，无需 `dark:` 前缀散落组件。
- 插件 Today 卡片是既有协议贡献点，尺寸声明属协议变更，须同步 PLUGIN_API.md + bridge + 示例三方。
- `toISOString().slice(0,10)` 存在跨午夜偏一天的隐患，「今天」须用本地时区。

## What I Don't Know（提案时待定 → 后续决策见 design.md）

- 双主题在各视图/弹窗/插件卡片的实际观感 → 依赖真机走查（实施后完成两轮）。
- StatCard「环比昨日」的数据来源 → 实施时确认 `context today` 无昨日数据，属 core 层新增字段，留作独立 plan（docs/plans/2026-08-30-statcard-day-delta.md）。
