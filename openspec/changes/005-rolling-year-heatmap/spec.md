# Spec 005: 年热力图滚动窗口（rolling-year-heatmap）

> 协议影响：`meta.schema_version` "3" → "4"。窗口口径变更属语义变更，字段名不变。

### MODIFIED-1：年总览窗口口径（task overview --period year）

- **原口径**：日历年 [1/1, 12/31]，leading_empty_count = 1/1 的周几索引，week_index 以 1/1 所在周周一为第 0 列。
- **新口径**：滚动 53 周 [本周一 − 52 周, 本周日]，恒 371 天；起点必为周一 → leading_empty_count 恒 0；week_index 以窗口起点为第 0 列。
- **验收**：`year_overview_rolling_53_weeks` 链路测试（371 天、首日为周一、今天在 week_index=52 列、窗口跨年）。

### MODIFIED-2：聚合热力图窗口口径（library / global year heatmap）

- **要求**：与 MODIFIED-1 同一窗口函数 `logical_day::rolling_year_range`；聚合规则不变
  （单任务 min(actual/target,1)，日均 = 有效任务贡献均值；未来日 display_state=future）。
- **验收**：现有 `library_heatmap_aggregate` / `global_heatmap_*` 测试在滚动窗口下通过。

### ADDED-1：热力格边框规则（前端）

- **要求**：未来日、上一年度格子 = 虚线边框；本年已过 = 实线；今天 = 主题色粗框；
  格内纯色填充无符号标记（沿用 2026-09-01 走查决策）。

### ADDED-2：快速捕捉与今日任务入口

- **要求**：快速捕捉建任务默认 `recurrence=once`（一次性，完成即归档）；
  今日任务卡标题点击进任务页；头部提供新建按钮（复用 TaskEditor）。
