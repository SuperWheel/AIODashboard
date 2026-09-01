# Plan：首页面板改造——年度热力图 + 今日任务双列填充卡 + 主库改名重要日

> 级别：Plan（新增视图组件 / 新增 Tauri command / UI 重构）
> 状态：已确认·进行中
> 提出日期：2026-09-01 · 完成日期：
> 关联：openspec/changes/003-task-checkin-cards（主库综合热力图口径来源）· dev-log 2026-09-01 条目

## 需求

首页面板三项改造（2026-09-01 对话确认）：

1. **新增任务热力图**：GitHub 式年度热力图，聚合所有任务（不限重要日归属），口径同主库综合热力图。
2. **今日任务太长 → 双列渐进填充卡片**：圆角矩形卡片，主题色从左侧按 count/target 比例填充；
   单次任务完成即满填充；计数显示 `1/4`；末尾圆形勾选框，全部完成后打钩；保留 3s 位置冻结。
3. **主库上首页 + 改名「重要日」**：右栏新增重要日卡；全仓人类可见文案 主库→重要日
   （协议字段 `library_id` 等不动；openspec 历史变更包与 dev-log 历史条目不回改）。

主库与项目**保留两个实体**（已确认）：项目=主题分类（可挂笔记），重要日=时间战役（日期锚点+归属历史+综合热力图）。

## 分步计划

- [x] 1. core：`overview_service::global_year_heatmap`（抽出 TaskDayFacts/task_day_contrib/aggregate_days 与主库热力图共用）+ 链路测试 ×2
- [x] 2. Tauri：`global_year_heatmap` command（CLI 不动，避免协议变更）
- [x] 3. 前端：`Heatmap.tsx` 抽出共享 `RateHeatmapGrid`（重要日详情改用）；TodayView 新增整行年度热力卡（accent 色、图例、点击跳任务页）
- [x] 4. 前端：右栏「重要日」卡（图标+标题+第 N 天/剩 N 天，点击进详情）；全仓改名主库→重要日（UI/CLI help/README/architecture）
- [x] 5. 前端：新组件 `TodayTaskCard`（双列网格 + 渐进填充 + 圆形勾选框）；TodayView 去掉三组分组，改位置快照（完成沉底）
- [x] 6. `bash scripts/check.sh` 全绿；人工验证首页三视图（待用户走查）
- [x] 7. dev-log 更新；切片提交——工作区混有 8/31 bugfix 批+004 实施（待验收）+ 本批，提交策略待用户定

## 验证

- [ ] `bash scripts/check.sh` 全绿
- [ ] 人工验证核心路径：首页热力图渲染与 hover 提示；打卡后今日卡片填充比例/勾选态变化且位置不跳；重要日卡点击进详情；双主题下填充色可读

## 结果记录（完成后填写）

- 实际改动文件、与计划的偏离、dev-log 链接。
