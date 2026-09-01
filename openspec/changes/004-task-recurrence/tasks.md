# Tasks：任务循环规则（004）

- [x] 1. domain：`Recurrence` 枚举（Default=Daily、serde 内部标签）+ `matches(anchor, day)` 纯函数 + `Task.recurrence`/`TaskTargetPeriod.recurrence` 字段；单测：daily/once 恒真、weekly 集合命中与空集回退锚点星期、monthly 31→2 月 clamp 至 28/29、yearly 2/29 平年→2/28、yearly 月份不匹配、脏日期防御 true。
- [x] 2. storage：SCHEMA_V4（`ALTER TABLE task_target_periods ADD COLUMN recurrence TEXT` 与 `user_version=4` 同一事务）；period_repo 读写（NULL↔Daily）；迁移测试：V3 库升级后存量区间 daily、user_version=4。
- [x] 3. core-checkin：`require_applicable_today`/`build_day_view` 接入 `matches`；`record`/`complete_today` 在 once 且当日首次达标时自动归档（activity + snapshot）。
- [x] 4. core-task/context：`CreateTaskInput.recurrence`、`UpdateTaskInput.recurrence`（区间滚动明天生效）；get/list 回填 `Task.recurrence`；`wall_task_views`（全部 active 含 not_applicable）；`missed_days_last_7d` 接入规则。
- [x] 5. core-overview：`build_days` 与主库年度热力图逐日判定接入 `matches`。
- [x] 6. core 测试：非适用日（weekly 非命中）打卡拒绝 + today 排除 + wall 包含（state=not_applicable）；weekly 命中日正常；once 达标自动归档且 Today 消失、历史保留；改 recurrence 后明日起按新规则、历史不变。
- [x] 7. protocol v3 + CLI：`SCHEMA_VERSION="3"`；`task create/update --recurrence/--weekdays`（含 --stdin JSON、weekday 1–7 校验、weekly 空集报错）；`task show` 输出含区间 recurrence。
- [x] 8. Tauri：create/update params 增 `recurrence`+`weekdays`；新 command `task_wall_views`。
- [x] 9. 前端：types/api 对齐；TaskEditor 循环选择器（weekly 星期多选 chips，受 dayLoaded 守卫）；TaskCard 副标题循环标签 + not_applicable 显示"今天不适用"；CheckinControls not_applicable 禁用；TasksView 数据源切 `taskWallViews`。
- [x] 10. integration.rs：`chain_task_recurrence`——CLI 创建 weekly 任务（含今天星期）→ checkin 成功；weekly（不含今天）→ today 列表排除、checkin exit code 4（Validation）；once → checkin 达标后 `task show` 为 archived；`--json` 输出含 recurrence 且 `meta.schema_version=="3"`。
- [x] 11. 门禁与文档：`bash scripts/check.sh` 全绿；dev-log、README 功能清单、docs/architecture.md（V4/协议 v3）、PLUGIN_API.md（createTask 循环参数后续扩展注记）。
