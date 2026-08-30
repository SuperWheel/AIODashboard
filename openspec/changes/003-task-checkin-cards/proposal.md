# Change Proposal 003: 打卡式任务 + 日期主库 + 四种任务卡片（task-checkin-cards）

> 状态：**进行中** · 模式：Spec · 关联：移植自 PlanningDays（/Users/leeyl/Project/ToDoList）卡片体系；触碰 AI 协议（schema_version 1→2）

## Why（为什么做）

现有任务模块是简单 todo（title/status/due_at/project_id），不支持「长期习惯/每日多次打卡」场景，也没有统计与可视化。PlanningDays 项目已验证一套完整设计：日/周/月/年四种任务卡片 + 打卡账本 + 日期主库，本次将其移植进 AIODashboard 并替换现有任务模块，同时贴合现有双主题 Bento 风格（圆角矩形主导的设计语言）。

## What I Want（要什么）

已与用户对齐的决策（2026-08-30 计划评审确认）：

1. **数据引擎完整移植 + 日期主库**：每日目标次数、append-only 幂等打卡账本（+1/-1/撤销）、历史目标区间、活动区间（归档/恢复）；纪念日/倒计时日主库、任务归属按天生效并保留历史、主库综合热力图、归档三选一流程。
2. **旧字段**：废弃 `due_at`（与每日打卡语义冲突），保留 `project_id`；`status` 三态改 `active/archived`；旧数据迁移 todo/doing→active、done→archived。
3. **卡片位置**：Tasks 页改卡片墙（每任务可选日/周/月/年样式并持久化）；Today Bento 的今日任务卡换成打卡式列表。
4. **设计语言**：圆角矩形为主——+/- 按钮为圆角矩形且**并排位于同一侧**（不与进度圆环分列两侧）；进度显示保留圆环。
5. **协议变更**：CLI `complete/reopen` 语义重映射，`meta.schema_version` 1→2，dev-log 记迁移说明。

## What I Know（已知）

- PlanningDays 侧的五态/六态判定、色阶公式、连续天数与完整完成率口径、热力图交互规格已提取为设计报告（见 design.md 决策 2/3）。
- AIODashboard 侧：Task 实体在 `dashboard-domain`，SCHEMA 迁移用「只增不改 + 表重建」模式（V2 已有重建 activity_log 先例）；前端语义 token 双主题体系可承载任务主题色（`color-mix` 色阶）。
- 前端当前**没有任务编辑入口**（update_task 用例未接线），本次随任务编辑器一并补上。
- 「今天」边界唯一入口 `context_service::local_today_range`，逻辑日换算在此基础上扩展，不另写。

## What I Don't Know（提案时待定 → 决策见 design.md）

- 旧 `complete/reopen` 命令的精确重映射语义与 deprecated 标注形态 → design.md 决策 5。
- `card_style`（卡片样式偏好）存库还是存前端 → design.md 决策 4（结论：存库，CLI/GUI 同步）。
- 年卡在 Bento 卡片宽度下的格子尺寸策略 → 实施时按 PlanningDays 动态公式适配。
