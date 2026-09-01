# Change Proposal 005: 年热力图改滚动 53 周窗口（rolling-year-heatmap）

> 状态：已完成（2026-09-01） · 提出日期：2026-09-01
> 关联：003（打卡式任务）、004（循环规则）、Plan `docs/plans/2026-09-01-home-heatmap-today-cards.md`

## Why

首页/任务年卡/重要日详情的年热力图原为日历年窗口（1/1–12/31）：今天埋在中间、右端是遥远的未来日，
卡片宽度利用差。用户走查要求：填满卡片、最右端列恒为本周、未来日与上一年度用虚线、本年已过用实线。

## What

1. **年窗口语义变更（协议 v4）**：`task overview --period year`、`library_year_heatmap`、
   `global_year_heatmap` 的年窗口从日历年改为**滚动 53 周**——`[本周一 − 52 周, 本周日]`，
   恒 371 天，起点必为周一（`leading_empty_count` 恒 0），`week_index` 以窗口起点为第 0 列。
   `meta.schema_version` 升 "3" → "4"。
2. **前端边框规则**：虚线 = 未来日 / 上一年度；实线 = 本年已过；粗框 = 今天。格内仍纯色填充无符号。
3. **配套 UI**：快速捕捉建任务默认一次性（once，完成即归档）；今日任务标题可点击进任务页、
   头部加新建按钮（复用 TaskEditor 弹层）。

## Unknown / 风险

- CLI 消费者若按日历年聚合 year 总览需改用 start_day/end_day 字段自定窗口（迁移说明见 dev-log）。
- 窗口跨年，前端一律不再显示单一年份标签（改「近一年」）。
