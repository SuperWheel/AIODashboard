# Plan：一次性任务完成当天留痕 + 历史翻页可见归档任务

> 级别：Plan（修改 004 既有决策的可见性语义，跨 storage/core/前端三层，无协议/schema 变更）
> 状态：已完成（2026-09-02；GUI 真机走查待用户）
> 提出日期：2026-09-02 · 完成日期：2026-09-02
> 关联：openspec/changes/004-task-recurrence（修订其「完成即终态」的可见性部分）· dev-log 2026-09-02 条目

## 需求

现状：一次性（Once）任务打卡达标的瞬间被自动归档，Today 视图与任务墙查询都硬过滤
`status='active'`，卡片当场从主界面消失；日期翻页回看历史时同样只查 active 任务，
已归档的一次性任务在任何历史日都不可见（只剩已归档 Tab/热力图/搜索）。
且 undo 对已归档任务只写补偿不解除归档，等于完成后无法撤销。

预期（用户 2026-09-02 确认，三项一起修）：

1. 一次性任务完成后**今天内仍以完成态保留**在 Today/任务墙（与重复任务完成后的表现一致），
   明天起自然消失——效果即「今天过了再归档」。
2. 任务墙**日期翻页回看**时，活动区间覆盖该日的已归档任务以当日真实状态显示（只读）。
3. 今天内对「今天刚自动归档」的一次性任务撤销完成 → **自动解除归档**，可继续打卡。

模型：保持「完成即归档」不变（归档时刻干净、完成态由账本派生）；改的是可见性规则——
归档只退出「活跃列表」，不退出「当天视图」与「按日重建的历史」。

## 分步计划

- [x] 1. storage：`period_repo` 新增 `task_ids_covering(day)`（活动区间覆盖 day，关闭日含当天）、
  `closed_on(task_id, day)`、`reopen_activity_closed_on(task_id, day)`。
- [x] 2. core：`build_day_view_on` 适用性改为纯区间驱动（去掉 `status==Active` 硬条件）；
  区间不覆盖时补「关闭日分支」：`closed_on(day) && 循环命中 && 当日达标` → 仍按适用（完成态留痕）。
- [x] 3. core：`today_task_views` / `wall_task_views_on` 候选集改为「区间覆盖当日」的任务全集
  （含已归档）；归档任务 state=not_applicable 一律不展示（手动归档今天未完成 → 当天也不留 NA 卡）。
- [x] 4. core：`task_service::unarchive_if_archived_today`（重开今天关闭的区间 + 恢复 Active）；
  `undo()` 对「Once + 今天归档 + 撤前今日 completed + 撤后不再达标」自动调用。
- [x] 5. 前端：留痕归档卡（`task.status==="archived"`）的交互——Today 卡主体点击不再触发打卡报错、
  隐藏 − 按钮、圆圈撤销可用；任务墙 CheckinControls 对归档卡只显示完成环（可点=撤销），隐藏 +/-。
- [x] 6. 测试：改 `once_task_auto_archives_on_completion`（Today 保留完成态）；新增
  undo 当天解除归档并可重新打卡、多目标撤销回 in_progress、历史翻页可见归档任务（backdate 活动区间）、
  手动归档当天退出 Today/墙；CLI integration 既有断言不受影响（已核对）。
- [x] 7. 文档：004 spec/design 相关行加修订注记；dev-log；本文件结果记录。

## 验证

- [x] `bash scripts/check.sh full` 全绿（rust fmt/clippy/test/build + CLI 链路 + 前端 vitest 32 + build）
- [ ] 人工验证核心路径（待用户真机走查）：完成一次性任务 → 完成卡留首页/任务墙；
  圆圈/⌘Z 撤销 → 回待打卡；翻回完成日 → 完成卡可见；翻回更早日期 → 当日在册任务可见。

## 结果记录（完成后填写）

- 实际改动：
  - storage：`crates/dashboard-storage/src/period_repo.rs`（+3 函数，SQL 仍只在 storage）
  - core：`checkin_service.rs`（build_day_view_on 区间驱动 + 关闭日分支；undo 自动解除当天归档）、
    `context_service.rs`（两个视图候选集改 task_ids_covering + 归档 NA 过滤）、
    `task_service.rs`（unarchive_if_archived_today）
  - 前端：`TodayTaskCard.tsx` / `TaskCard.tsx`（留痕归档卡交互）
  - 测试：`crates/dashboard-core/tests/checkin.rs`（改写 1 + 新增 3）
  - 文档：004 spec/design 修订注记、dev-log 2026-09-02 条目
- 与计划无偏离。未提交说明：两个前端卡片文件与 006 拖拽走查 WIP 同文件纠缠，随该批入库。
- dev-log：docs/dev-log.md「2026-09-02 一次性任务完成当天留痕 + 历史翻页可见归档任务」
