# Tasks 005: 年热力图滚动窗口

## core

- [x] `logical_day::rolling_year_range` / `rolling_week_index` + 单测（53 周、起点周一、跨年）
- [x] 三处接入：`task_period_overview` / `library_year_heatmap` / `global_year_heatmap`
- [x] 删除 `year_week_index` / `year_leading_empty`（无消费方）
- [x] 协议 `SCHEMA_VERSION` "3"→"4" + 历史注释
- [x] 测试：`year_overview_rolling_53_weeks` 重写；integration.rs schema_version 断言升 4

## 前端

- [x] `HeatmapCell` prevYear 虚线规则；`isPrevYear` 工具；TaskCard 周/月格、TaskDetailView 月格接入
- [x] `YearHeatmap` / `RateHeatmapGrid` 渲染完整 53 列（未来日虚线）+ 首屏滚到最右
- [x] 标题文案改「近一年热力」/「综合热力图 · 近一年」（窗口跨年不显示单一年份）
- [x] 快速捕捉默认 once；今日任务标题跳任务页 + 头部新建按钮（TaskEditor 弹层）

## 收尾

- [x] dev-log 记录（协议 v4 迁移说明）
- [x] check.sh 全绿
- [ ] 用户 GUI 走查：53 列填满、虚线/实线规则、快速捕捉一次性任务完成即归档
