# Plan：任务页日期翻页（前一天/后一天 + 任意日期跳转）

> 级别：Plan（新增视图交互 + core 新函数）
> 状态：已完成（2026-09-01）
> 提出日期：2026-09-01 · 完成日期：2026-09-01
> 关联：dev-log 2026-09-01 条目

## 需求

任务页支持按日期翻页查看任务墙：前一天/后一天逐日翻页，并可快速跳转任意日期。

## 分步计划

- [x] 1. core：`checkin_service::task_day_view_on(task, day)`（build_day_view 拆出 day/today 双参数）+ `context_service::wall_task_views_on(day)`；`wall_task_views` 改为薄包装
- [x] 2. Tauri：`task_wall_views(day: Option<String>)`；前端 api 接线
- [x] 3. TasksView：`DayNavigator`（◀ ▶ 翻页 + 日期弹层跳任意日 + 非今天显示「回到今天」）
- [x] 4. TaskCard：`anchorDay` 锚定周/月/年周期总览到所选日；`interactive=false` 时打卡控件只读（只能今天打卡）；副标题「今日/当日」措辞与 missed 态
- [x] 5. 测试：`wall_views_on_past_and_future_day`（今天 in_progress / 昨天 not_applicable / 明天 pending / 非法日期拒绝）
- [x] 6. check.sh 全绿

## 边界（设计时已拍板）

- 只有「今天」可打卡；过去/未来日只读（补历史卡 = 写任意 logical_day 的账本，影响 missed 统计与 once 归档语义，留作后续 Spec）。
- 翻页只看当前启用任务；已归档任务不进历史日期。
- CLI 不动，无协议变更。

## 验证

- [x] `bash scripts/check.sh` 全绿
- [ ] 人工验证：翻页后卡片状态/计数正确、周月年卡锚定所选日、非今天打卡控件只读

## 结果记录

- core（checkin_service/context_service 各 +1 函数）、Tauri task_wall_views 加参、api.ts、TasksView（DayNavigator）、TaskCard（anchorDay/interactive/dayWord/missed 文案）、checkin.rs +1 链路测试。
