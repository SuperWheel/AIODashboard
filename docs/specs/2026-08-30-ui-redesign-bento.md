# Spec：UI 重设计 · Bento 总控台（ui-redesign/2026-08）

> 级别：Spec · 状态：**已完成（2026-08-30，切片 1–5 全部落地；遗留项单列）**
> 提出日期：2026-08-30 · 确认日期：2026-08-30 · 完成日期：2026-08-30
> 关联：docs/specs/2026-08-30-plugin-system.md（Today 卡片 size 协议）· dev-log 2026-08-30「UI 重设计」条 · 遗留 plan 见文末

## 决策记录（已拍板）

与用户对齐的四项偏好：
- 风格 C（Bento 总控台）
- 布局：首页网格 + 子页单列
- 主题：跟随系统自动双主题
- 首页重点：今日任务为主 / 插件卡片舞台 / 快速捕捉 / 统计可视化（全选）

## Why

现有五视图为默认样式，信息密度与观感不支撑「All-in-One 总控台」定位；且插件卡片需要与核心模块平起平坐的一等格位。重设计同时解决：双主题基建（插件 UI 天然跟随）、首页信息架构（今日任务为主）、组件库统一（语义 token，杜绝写死色值）。

## 设计概念

**「总控台 + 工作室」两层结构**：

- **首页（Today）= 总控台**：Bento 卡片网格，一屏尽览今日状态；插件卡片与核心模块平起平坐。
- **子页（任务/项目/笔记/收件箱/插件）= 工作室**：保持单列居中阅读流，专注处理单一模块。

气质关键词：克制、秩序、信息密度可控。暗色像深夜工作台，亮色像清晨书房——同一套结构，两种氛围。

## ADDED

### A1 设计 Token（Tailwind 4 CSS-first）

全部颜色走语义 token，组件不再写死 `#0f1115` 这类色值：

| Token | 深色 | 浅色 | 用途 |
|---|---|---|---|
| `--color-bg` | `#0B0D12` | `#F7F6F2`（暖纸白） | 窗口底色 |
| `--color-surface` | `#14171F` | `#FFFFFF` | 卡片 |
| `--color-surface-2` | `#1B1F2A` | `#F0EFEA` | 嵌套/悬浮层 |
| `--color-line` | `white/8%` | `black/8%` | 描边 |
| `--color-ink` | `#E8EAF0` | `#1B1D23` | 主文字 |
| `--color-ink-2` | `#8B93A7` | `#6B7180` | 次要文字 |
| `--color-accent` | emerald-400 | emerald-600 | 主强调（完成、主按钮） |
| `--color-danger/warn/info` | rose/amber/sky 400 | 同族 600 | 状态色 |

- 圆角：卡片 `rounded-2xl`(16px)，控件 `rounded-lg`，徽章 `rounded-full`。
- 数字：统计一律 `tabular-nums`，避免跳动。
- 双主题实现：`@theme inline` 把语义 token 映射到运行时 CSS 变量，双主题 = 变量翻转（浅色 `:root` 默认 / `[data-theme="dark"]` / 系统深色媒体查询三选择器）；组件零 `dark:` 前缀，插件 UI 天然跟随——这是「插件只用语义类名」规范成立的基础。

### A2 主题三态

`data-theme` 属性 + `prefers-color-scheme` 媒体查询兜底；跟随系统/深色/浅色三态存 localStorage，index.html 内联脚本首帧前还原防闪烁；system 态不设属性交给媒体查询，原生控件配色由根 `color-scheme` 驱动。

### A3 新组件

- **ProgressRing**：SVG 圆环显示今日完成率，放问候卡。
- **PageHeader**：子页统一页头（标题 + 数量徽标 + 主操作按钮）。
- **QuickCapture**：独立卡片常驻右上，输入框 + 去向切换（今天到期 / 仅收集到收件箱），Enter 即走；「今天」用本地时区（`hooks.localToday()`，修掉 `toISOString` 跨午夜偏一天）。
- **微动效**：卡片 hover 轻微上浮（translateY(-1px) + 阴影加深）、打勾回弹；一律 ≤200ms，尊重 `prefers-reduced-motion`。
- 可访问性：双主题对比度均 ≥ WCAG AA；所有交互元素有 focus ring。

## MODIFIED

### M1 首页 Today → Bento 网格

容器从 `max-w-3xl` 放宽到 `max-w-6xl`，12 列网格，卡片可跨格：

```text
┌────────────────────────── 12 col ──────────────────────────┐
│ [问候 + 今日进度环 · span 5] [快速捕捉 · span 4] [插件槽 1 · span 3] │
│ [今日任务（含逾期折叠区）· span 8] [统计 2×2 迷你卡 · span 4]        │
│ [最近笔记 · span 4] [活跃项目 · span 4] [插件槽 2 · span 4]          │
└────────────────────────────────────────────────────────────┘
```

- **今日任务为主**：最大卡片，逾期任务置顶红边分组，未完成任务排在插件卡之前。
- **插件卡片舞台**：`registry.cards` 是网格中的一等格位；插件协议 `registerTodayCard` 新增可选 `size: "sm" | "md" | "lg"`（sm=3 列 / md=4 列 / lg=6 列，缺省 md）——**协议变更已同步 PLUGIN_API.md + bridge 校验 + 示例插件三方**。
- 窄窗口降级：`<lg` 断点自动堆叠回单列，顺序 = 任务 > 捕捉 > 插件 > 统计 > 其他。

### M2 子页（工作室）

- 保持 `max-w-3xl` 单列居中；统一 PageHeader；列表行 hover 浮现操作，完成态 checkbox 打勾微动效。
- 空态：大号 glyph + 一句引导 + 主按钮。

### M3 Sidebar / StatCard

- **Sidebar**：毛玻璃（`backdrop-blur`），文字图标升级为 lucide-react 线性图标，底部主题切换三态循环。
- **StatCard** 重构为迷你卡：数值 + 环比昨日箭头，2×2 收进侧栏格；卡片可交互（onClick 跳对应视图）。

## REMOVED

- 无。现有功能全部保留，仅视觉与布局重构。

## 红线核对

本 spec 为纯前端/UI 变更，不触碰 storage/core：SQL 红线、业务在 core、前端不碰 DB 均不受影响；唯一协议变更（Today 卡片 `size`）按「文档即协议」纪律同步了 PLUGIN_API.md、bridge/测试、示例插件三方。

## Tasks（切片提交时逐项勾选）✅ 全部完成

- [x] 切片 1 主题基建：styles.css token 化 + `@custom-variant` + 主题切换逻辑；全量替换写死色值（46a5e5d）
- [x] 切片 2 组件库：ui.tsx 重构 + ProgressRing/PageHeader/QuickCapture 新组件（0f619d6）
- [x] 切片 3 Today 页 Bento 化：网格布局 + 逾期折叠 + 插件槽位 size 协议（bdb90f2）
- [x] 切片 4 子页打磨：PageHeader 统一、空态、行交互、Sidebar 图标与毛玻璃（52bd7ed）
- [x] 切片 5 验收：check.sh 全绿 + 双主题真机走查（两轮，修复插件热加载/卡片交互/输入法等）+ dev-log

## 测试与验证清单

- [x] bridge.test.ts 补 Today 卡片 size 透传/非法 size 拒绝用例（vitest 22 通过）
- [x] `bash scripts/check.sh` 六步全绿（fmt/clippy/Rust 测试/构建 + vitest + 前端生产构建）
- [x] 双主题人工走查：浅色/深色下各视图、弹窗、插件卡片观感（两轮真机走查完成）

## 明确不做（本次范围外）

- 三栏式列表/详情布局（任务详情面板化）——后续单独立项。
- 视图自定义排序/拖拽布局——等插件生态起来后再评估。
- 移动端适配——macOS 桌面优先不变。

## 已知遗留（实施后发现，已另立 plan）

- StatCard「环比昨日」箭头未接线——`context today` 无昨日数据，属 core 层新增字段 → `docs/plans/2026-08-30-statcard-day-delta.md`
