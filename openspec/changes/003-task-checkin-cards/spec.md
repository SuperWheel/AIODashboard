# Spec 003: 打卡式任务 + 日期主库 + 四种任务卡片

> 验收证据见 tasks.md；关键决策与红线核对见 design.md。

## ADDED

### ADDED-1：打卡账本（completion_records）

- **要求**：每次打卡/减少/撤销落一条 append-only 记录：`id(crc_ 前缀) / task_id / operation_id(全局唯一) / value(+1 或负补偿) / kind(add|decrement|undo) / compensates_operation_id(可空) / logical_day(本地 YYYY-MM-DD) / source(user|cli|ai|plugin:*) / created_at(UTC)`。当日计数 = Σvalue，下限 0。
- **要求**：幂等——同 operation_id 重放直接返回原结果，不重复计数；一条正向记录最多被补偿一次；减少不能使当日计数 < 0；任务当日不适用（归档中/无有效目标）时拒绝打卡并报错。
- **验收**：core 单测覆盖幂等重放、重复补偿拒绝、减到 0 以下拒绝、不适用拒绝；integration.rs 有 CLI 打卡链路。

### ADDED-2：历史目标区间（task_target_periods）

- **要求**：每日目标保存为有效区间 `[start_day, end_day)`；修改目标从当前逻辑日起生效，旧日期继续读当时目标；目标取值 1–999；重叠区间或非法目标报一致性错误，不静默选取。
- **验收**：单测覆盖改目标后历史日期仍按旧目标判定、重叠区间拒绝。

### ADDED-3：任务活动区间（task_activity_periods）

- **要求**：新建任务建立开放区间；归档关闭区间（归档日及此前历史保留）；恢复建立新区间；区间外日期状态为「不适用」，不进入统计分母。
- **验收**：单测覆盖归档-恢复间隔日显示不适用、不进入完成率分母。

### ADDED-4：当日五态与热力六态判定（纯函数）

- **要求**：五态 `not_applicable / pending / in_progress / completed / missed`——不适用优先；actual ≥ target 即 completed（超额算完成）；过去未达标 = missed；有记录未达标 = in_progress；其余 pending。六态 `not_applicable / future / zero / partial_low / partial_high / complete`——未来日一律 future；0 次 = zero；完成率 <50% partial_low、≥50% partial_high、100% complete。
- **验收**：纯函数单测逐态覆盖（含超额、未来、无目标边界）。

### ADDED-5：周期总览（周/月/年）

- **要求**：后端按 `period(week|month|year) + anchor_day` 输出：周期起止、每日（logical_day、六态、五态、actual、target、is_today、week_index、month）、summary（actual_count、applicable_day_count、complete_day_count、complete_day_rate、current_streak、longest_streak）、buckets（周=每日、月=按周、年=按月）。口径：完整完成率 = completeDay/applicableDay；连续天数遇非 complete 即断（不适用日也断）；超额封顶 100% 但保留真实次数。
- **验收**：单测覆盖各口径公式与 PlanningDays 报告一致；CLI `task overview --period` 可输出。

### ADDED-6：日期主库（date_libraries）

- **要求**：两种类型——纪念日（锚点 ≤ 今天，显示「第 N 天」）与倒计时日（锚点 ≥ 今天，显示「还剩 N 天 / 就是今天 / 已逾期 N 天」）；日期差按 Calendar 自然日，不用秒除 86400；过期倒计时不自动转纪念日、不自动归档。字段：`id(dlb_ 前缀) / title / note / icon / color_hex / kind / anchor_day / sort_order / status(active|archived)`。
- **验收**：单测覆盖天数计算（含当天、逾期）；CLI `library` 命令组可用。

### ADDED-7：任务归属区间（task_library_membership_periods）

- **要求**：一个任务同一逻辑日最多属于一个主库，也可独立（无归属）；移库/移出从当前逻辑日起生效，过去日期保留原归属；主库归档时三选一：保留归属 / 转独立 / 移到另一主库，一个事务完成，失败整体回滚。
- **验收**：单测覆盖移库后历史仍归原库、同日唯一归属约束、归档三选一事务回滚。

### ADDED-8：主库综合热力图

- **要求**：按年逐日聚合：单任务当日贡献 = min(max（净次数，0) ÷ 当日历史目标， 1)；主库完成率 = 有效直属任务贡献之和 ÷ 有效任务数；无有效任务 = 不适用；归属按当日真实生效区间读取（不用当前归属回算历史）。
- **验收**：单测覆盖聚合公式（100%+50% 两任务 = 75%）、移库前后各归各库。

### ADDED-9：任务编辑器与卡片样式偏好

- **要求**：前端新增任务编辑表单（标题/图标/主题色 6 预设/每日目标/单位/所属项目/所属主库），新建与编辑共用；`card_style(day|week|month|year)` 每任务持久化，右键菜单切换。
- **验收**：人工走查编辑保存生效；切样式后刷新仍保持。

### ADDED-10：四种任务卡片与任务详情

- **要求**：TaskCard 四模式共用 header（44px 圆角矩形图标块 + 标题 + 状态副标题）；日卡=状态行，周卡=7 格横排，月卡=当月紧凑色块阵列，年卡=7 行横滚热力图（首屏定位今天所在周）。控件：目标=1 时圆环完成钮；目标>1 时圆角矩形 `[-][+]` **并排同侧** + 进度圆环。任务详情页周/月/年三档总览 + 统计（实际次数/适用天数/完整完成天数/完成率/连续天数），快捷键 `⌘1/2/3`、`⌥←/→`。
- **要求**：热力格六态渲染——色阶用 `color-mix(in srgb, 任务色 X%, transparent)`；非颜色通道：不适用=虚线框、今天=加粗边框、未来=小圆点、0 次=横杠、部分完成=单/双点、完成=✓；带图例与日期详情（hover/选中）。
- **验收**：双主题真机走查；色阶、虚线、符号在浅色/深色下均可区分。

### ADDED-11：Today 打卡式任务列表

- **要求**：Today 页今日任务卡改为打卡式：按 未完成/进行中/已完成 分组，行内进度圆环 + 圆角矩形 +/-（同侧）；键盘 `Space` 主操作、`Enter` 详情、`⌘Z` 撤销本次运行内最近一次打卡；今日完成率口径 = Σmin（次数，目标） ÷ Σ目标。
- **验收**：真机走查打卡/减少/撤销与分组流转；Widget Snapshot 今日统计同步为新口径。

### ADDED-12：日期主库前端视图

- **要求**：侧边栏新增「全部主库 / 各活动主库 / 已归档」入口；主库卡片网格（三段式：图标标题 / 大号天数 / 直属任务数+锚点日）；主库详情四区块（日期概览、综合热力图、直属任务、设置），直属任务复用任务卡片与打卡；归档弹窗三选一；已归档页可恢复。
- **验收**：真机走查创建/移库/归档三选一/恢复全链路。

### ADDED-13：CLI 打卡与主库命令

- **要求**：`task create/update` 支持 `--target/--unit/--icon/--color/--card-style`；新增 `task checkin / decrement / undo / overview --period`；新增 `library create/list/show/archive/restore` 与 `task move --library`；全部支持 `--json`，错误映射 exit code。
- **验收**：integration.rs 链路测试：CLI 打卡→Core 读一致；主库创建/移库/归档链路；协议信封 `meta.schema_version="2"`。

## MODIFIED

- **Task 实体**：`status` 由 `todo/doing/done` 改 `active/archived`；删除 `due_at / completed_at`；新增 `icon / color_hex / unit / card_style`；保留 `project_id`。
- **AI 协议**：`meta.schema_version` 1→2；`task list/show/create/update` 输出字段随实体变更；`task complete/reopen` 重映射为「补满今日目标 / 今日清零」并在输出中带 `deprecated: true` 提示；迁移说明记 dev-log。
- **Widget Snapshot**：`today` 统计改打卡口径（今日完成率 = Σmin（次数，目标）/Σ目标；任务数为当日有效任务数）。
- **Tasks 页**：行列表改为卡片墙网格；导航参数（navParam tab）保留兼容（today/overdue tab 语义映射：overdue 页在打卡体系下显示「已错过」历史提示或移除——实施时以 spec 决策为准：移除 overdue tab，Today 的逾期 StatCard 改指「已错过」含义或移除该卡，见 design.md 决策 6）。
- **插件桥**：task 相关桥接 API 字段随实体变更同步（PLUGIN_API.md + bridge + 示例插件三方同步）。

## REMOVED

- `due_at` 字段及 CLI `--due`、`--today/--overdue` 过滤、前端日期输入与逾期分组。
- `TaskStatus::Doing`（三态简化为主启用/归档；doing 语义由每日进行中状态替代）。
- 前端 `TaskRow` 的复选框 toggle 交互（由打卡控件替代）。

## 非本变更范围（明确不做）

- 阶段与里程碑（PlanningDays ProgressNode）、子任务、依赖关系。
- 历史补打卡 UI、单条打卡记录编辑。
- 主库背景图、周年提醒、通知/提醒系统。
- Widget 本体（SwiftUI 小组件）开发；多设备同步/CloudKit。
- 已归档任务管理页（Core 保留恢复能力，UI 后续单独立项）。
