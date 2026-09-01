# Design 005: 年热力图滚动窗口

## 窗口定义

`logical_day::rolling_year_range(anchor)`：
- 本周范围复用 `week_range`（周一起），起点 = 本周一 − 364 天（52 周），终点 = 本周日。
- 恒 371 天 = 53 列 × 7 行，起点必为周一 → 前置空格恒 0。
- 右端列恒为本周 → 今天恒在最右列（满足"最右端是今天"的走查要求）。
- 窗口跨年（如 2026-09 的窗口含 2025-09~12 尾部）→ "以前年度"格子由此而来。

## 为什么改核心而不是前端过滤

上一版曾用纯前端过滤未来日实现"右端=今天"（YTD 口径），但无法填满卡片宽度、
也没有上一年尾部格子（用户要求以前年度虚线）。窗口口径必须在 core 统一定义，
否则 CLI 与 GUI 两个客户端会看到不同语义（红线：业务逻辑唯一实现处在 core）。

## week_index 口径

旧：以 1/1 所在周周一为第 0 列（日历年专用）。新：`days_between(window_start, day) / 7`，
对 week/month/year 统一成立（周/月视图不消费该字段，仅年网格按 7 分块）。
`year_week_index` / `year_leading_empty` 随切换删除（无其他消费方）。

## 协议

schema_version "3"→"4"。字段名与信封结构不变；`task overview --period year`、
library/global 热力图的 start_day/end_day/week_index/leading_empty_count 语义变更。
CLI 消费者如需日历年口径，按 start_day/end_day 自行裁剪。

## 前端边框规则

`HeatmapCell` 增 `prevYear` prop；`isPrevYear(day)` 按日历年字符串比较。
dashed = future || prevYear；不适用日本年范围内不再虚线（ faint 填充已足够区分）。
两个网格组件（YearHeatmap / RateHeatmapGrid）渲染完整 53 列并首屏滚动到最右。
