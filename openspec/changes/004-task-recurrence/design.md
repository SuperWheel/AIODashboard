# Design：任务循环规则（004）

## 关键决策

1. **规则放在目标区间行，而非 tasks 表**。理由：与 003 的"区间化历史"一致——改规则不回写历史（每周三改成每日后，过去仍按每周三口径统计）；复用 close/insert 区间流程；`target_on(day)` 一次取回 target+recurrence+锚点（区间起始日），逐日计算**零额外查询**。代价：无目标区间覆盖的日子谈不上循环（本就不适用，语义自洽）。
2. **锚点 = 区间起始日**。weekly 空 weekdays 集 → 取锚点星期；monthly/yearly 取锚点日号/月日。用户改规则时新区间锚点=明天，星期类规则因此"从生效日起算"——编辑器在切 weekly 时默认勾选"当天星期几"，所见即所得。
3. **once = daily 适用 + 完成即自动归档**（用户决策的组合推论）。不引入"到期日"字段：顺延语义下到期日每天都在变，无信息量；归档即终态，历史完整保留。
4. **错位 clamp**：`min(锚点日号, 当月天数)`；yearly 再要求月份相等（2/29 平年 → 2/28）。
5. **改规则明天生效**（与改目标一致）：避免当天已打卡数据在新旧规则间口径漂移。编辑器保存后次日完全生效，当天按旧规则。
6. **wall_task_views 与 today_task_views 分离**：任务墙要展示"存在但今天不适用"的卡（否则每周任务周内大部分时间从任务页消失，用户会以为丢了）；Today 严格当日适用。两者共用 build_day_view。
7. **协议 v3 而非可选字段静默追加**：Task JSON 增字段属"CLI 输出字段变更"，按红线 8 升版本并记录。
8. **V4 迁移单事务**：`BEGIN; ALTER TABLE … ADD COLUMN recurrence TEXT; PRAGMA user_version=4; COMMIT;` 一个 batch——堵上 003 遗留的"建表成功但版本号未写、崩溃重跑"窗口（ALTER 对已存在列会报错，重跑安全性由事务保证）。

## 涉及面（六层清单）

| 层 | 改动 |
|---|---|
| domain | `Recurrence` + `matches(anchor, day)` 纯函数 + Default(Daily)；`Task.recurrence`（serde default）；`TaskTargetPeriod.recurrence` |
| storage | SCHEMA_V4；period_repo：TARGET_COLS/map/insert_target 增列（NULL=daily） |
| core | checkin_service（require/build 接入 matches；record/complete 达标且 once → 归档）；task_service（create/update 输入与区间滚动；get/list 回填 Task.recurrence；wall_task_views 入 context_service）；context_service（missed 逐日判定）；overview_service（build_days / library heatmap 逐日判定） |
| protocol | SCHEMA_VERSION=3 + 历史注释 |
| cli | task create/update 增 --recurrence/--weekdays（含 stdin JSON）；解析与校验（weekday 1–7） |
| tauri | create/update params 增 recurrence(kind)+weekdays；新 command `task_wall_views` |
| frontend | types/api；TaskEditor 循环选择器（weekly 显示星期 chips）；TaskCard/CheckinRow not_applicable 态（控件禁用 + "今天不适用"）；TasksView 数据源切 task_wall_views |
| tests | domain matches 单测（周集合/空集回退锚点/月 clamp/年 2-29/脏数据防御）；core：非适用日打卡拒绝+Today 排除+wall 包含、once 达标自动归档、改规则区间滚动；integration：CLI 全链路 + schema_version=3 |

## 边界与已知取舍

- 归档后的一次性任务被 undo：补偿写入原日账本，任务维持归档；恢复后当日计数恢复显示（账本为准）。
- weekly weekdays 允许空集（=锚点星期），编辑器不会产生空集（始终至少勾选一个）。
- `library_tasks` 计数（CLI `library show`）不按当日适用过滤——与现状一致，另行治理（004 不扩scope）。
- 插件 bridge `createTask` 不暴露 recurrence（默认 daily），PLUGIN_API.md 加"后续扩展"注记。
