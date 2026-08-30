# Design 003: 打卡式任务 · 关键设计决策

## 决策 1：账本用 append-only + 补偿记录，不用原地改计数

- **选择**：每次操作写一条 `completion_records`（value ±1，kind add/decrement/undo，compensates_operation_id 指向被补偿记录）；当日计数 = Σvalue。
- **理由**：与 PlanningDays 验证过的模型一致——幂等（operation_id 唯一约束）、可审计（撤销留痕）、CLI/GUI/插件多写者安全。
- **后果**：读计数需 SUM 聚合（按 (task_id, logical_day) 建索引，数据量个人级无压力）；不能「编辑历史某条」，只能补偿（与范围声明一致）。

## 决策 2：状态与统计口径照搬 PlanningDays 公式（不重新发明）

- **选择**：五态/六态判定、完整完成率（completeDay/applicableDay）、连续天数（非 complete 即断，不适用日也断）、今日完成率（Σmin(count,target)/Σtarget）、主库聚合（逐日 min(count/target,1) 求均值）全部照搬已验证口径，在 core 实现为纯函数。
- **理由**：PlanningDays 侧这些口径经过 134 项测试验证；重新设计只会引入口径漂移。
- **后果**：不适用日打断连续天数是**有意行为**，需在用户可见文案中表达一致。

## 决策 3：前端色阶用 color-mix + 任务主题色 hex

- **选择**：任务存 `color_hex`（6 预设色：海蓝 #4A90E2 / 青绿 #38A38A / 草绿 #69A84F / 暖橙 #E49345 / 珊瑚 #D96767 / 紫罗兰 #8067C8）；热力格填充 = `color-mix(in srgb, <hex> <intensity>%, transparent)`，intensity 映射照搬（zero 14% / partial 18%+rate×74% / complete 94%）；非颜色通道（虚线/粗框/符号）同时实现。
- **理由**：现有语义 token 体系管「界面」，任务主题色管「数据身份」，二者不冲突；color-mix 双主题自动成立，无需新 token。
- **后果**：6 色在深色主题下不翻转（与 PlanningDays 一致），走查时确认深色下对比度。

## 决策 4：card_style 存库不存前端

- **选择**：`card_style` 作为 Task 字段入库（默认 day）。
- **理由**：CLI 也能读/改；多客户端一致；符合「业务状态在 core」红线。
- **后果**：`task update --card-style` 进 CLI 协议。

## 决策 5：complete/reopen 重映射而非删除

- **选择**：`task complete` = 对今日补满（写 target−count 条或一条等值 add 记录——实施取一条等值记录，kind=add），`task reopen` = 今日清零（一条负补偿）；JSON 输出带 `deprecated: true, replacement: "task checkin"`。
- **理由**：老脚本/插件不直接炸；语义上「完成任务」在打卡体系下最接近的映射就是「今日达标」。
- **后果**：协议 v2 记录该映射；未来大版本可删除。

## 决策 6：「逾期」概念移除，StatCard 改为「已错过」

- **选择**：due_at 删除后「逾期」失去基础；Today 的逾期 StatCard 改为「本周已错过天数」（近 7 日 missed 计数，点击跳任务详情/任务页）；CLI `--today/--overdue` 过滤移除。
- **理由**：打卡体系下「昨天没做」由热力图和 missed 态表达，不需要独立逾期列表。
- **后果**：Today 页 StatCard 四卡变为：今日待办 / 已完成 / 本周错过 / 收件箱。

## 决策 7：+/- 按钮同侧圆角矩形

- **选择**：多目标任务的控件区 = `[进度圆环] [-][+]`，+/- 圆角矩形（rounded-lg）并排于圆环同侧（右侧控件区内圆环居左、按钮组居右，或按钮组整体在圆环旁——实施以视觉走查为准）；单目标任务整个圆环即按钮。
- **理由**：用户明确反馈 +/- 分列圆环两侧「不好看」；圆角矩形为主的设计语言。
- **后果**：与 PlanningDays 的 `[-] [圆环] [+]` 夹心布局不同，属有意偏离。

## 决策 8：逻辑日换算扩展 context_service，不另写

- **选择**：新增 `local_today() -> NaiveDate`（基于现有 `local_today_range` 同一时区逻辑），logical_day 一律存 `YYYY-MM-DD` 文本。
- **理由**：红线 6——时间边界唯一入口。

## 数据模型（SCHEMA_V3）

```text
tasks            重建：status(active|archived), +icon/color_hex/unit/card_style, -due_at/completed_at
completion_records(id PK, task_id FK, operation_id UNIQUE, value INT, kind, compensates_operation_id,
                   logical_day, source, created_at)  INDEX(task_id, logical_day)
task_target_periods(id PK, task_id FK, target INT, start_day, end_day NULL)  不重叠约束在 core 校验
task_activity_periods(id PK, task_id FK, start_day, end_day NULL)
date_libraries(id PK, title, note, icon, color_hex, kind(anniversary|countdown), anchor_day,
               sort_order, status(active|archived), created_at, updated_at)
task_library_membership_periods(id PK, task_id FK, library_id FK NULL(移出=闭区间), start_day, end_day NULL)
```

迁移 V2→V3：todo/doing→active、done→archived；为每个迁移任务补一条开放 activity_period（start=创建日）与一条 target_period（target=1, start=创建日）；due_at/completed_at 数据不回填到打卡账本（避免污染迁移日热力图），随表重建丢弃。

## 风险

- 前端视图量大（卡片×4 + 详情×3 + 主库×3），切片 5/6 可能拆分提交。
- 协议 v2 为有意 breaking change：due 字段消失、complete/reopen 语义变。
- 深色主题下 6 预设色对比度需走查确认。

## 红线核对

| 红线 | 落实 |
|---|---|
| 1 SQL 只在 storage | 5 张新表与全部查询进 dashboard-storage 新 repo 文件 |
| 2 业务只在 core | 账本/状态/统计/归属规则全部在 dashboard-core；CLI/Tauri 只做转换 |
| 3 前端不碰 DB | 前端只 invoke Tauri command |
| 4 变更留痕 | 打卡/目标/归属/主库变更均 log_activity + snapshot::refresh |
| 5 domain 零依赖 | 新实体只用 chrono/serde/uuid，同现有 |
| 6 时间边界唯一入口 | 逻辑日换算扩展 context_service（决策 8） |
| 7 迁移只增不改 | V3 为新分支；V1/V2 分支不动 |
| 8 协议变更升版本 | schema_version 1→2 + dev-log 迁移说明 |
| 9 库代码禁 unwrap | 错误统一 CoreError |
