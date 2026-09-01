# Plan：全量 bug 排查批修（账本错账 / Tauri 参数契约 / 插件事件 / 快照缺口）

> 级别：Plan · 状态：已完成（2026-08-31）
> 提出日期：2026-08-31
> 关联：dev-log 2026-08-31；`2026-08-30-project-snapshot-refresh.md`（随本批实施）

## 需求

用户要求排查现有功能 bug。三路并行审查（core / storage / 前端+Tauri）+ CLI 实测，确认 4 严重 + 4 高 + 若干中低。本批修复其中不涉及 CLI JSON 协议与 schema 变更的全部项：

**严重**
1. Tauri 嵌套参数 camelCase 被静默丢弃：GUI 新建任务的卡片样式/项目/主库全部丢失，切换卡片样式、改项目永远不生效（Tauri 只转换命令顶层参数名，嵌套 struct 走纯 serde）。
2. undo 跨日错账：补偿记录固定写 `logical_day=today`，而日聚合按 logical_day 分组 → 撤昨天的卡历史不动、今天账面变负，之后打卡 +1 被 `.max(0)` 钳位吞掉。
3. reopen_today（今日清零）写无补偿指向的批量负记录：正向记录仍算"未补偿"，decrement/undo 继续放行把账面挖负；complete_today 从钳位值算补足量，补不满。
4. TaskEditor 编辑归档任务：target/libraryId 不从 task 初始化且 today_tasks 查不到归档任务 → 查看+保存即静默把目标重置 1 并移出主库。

**高**
5. 插件领域事件（task.completed/created、inbox.added、note.created）声明了却从未发射，pomodoro 示例的核心联动不存在。
6. project create/archive/delete、inbox process_to_note 漏刷 snapshot（后者为新发现）。

**中**
7. missed_days_last_7d 混入归档任务（与 Today 口径矛盾）；open_task_count 残留 `status != 'done'` 判断。
8. Today 页 ⌘Z 劫持输入框文本撤销；详情页撤销按钮无防抖连点连撤；dev StrictMode 双挂载下插件重复注册 → 自动停用写库。

## 分步计划（均已完成）

- [x] 1. core：undo 补偿记入被撤记录所在 logical_day；complete_today 改用未钳位 `day_sum` 算补足量（历史脏数据自愈）；reopen_today 逐条补偿当日未补偿正向记录（新增 repo 查询 `uncompensated_positives_on`）。
- [x] 2. core 测试：新增跨日撤销、reopen 后守卫、complete 真实账面补足三个回归测试（checkin.rs）。
- [x] 3. Tauri：`CreateTaskParams`/`UpdateTaskParams` 加 `#[serde(rename_all = "camelCase")]`；update 增加 `clearProject` 标志表达"移出项目"（JSON null 无法表达 `Some(None)`）。
- [x] 4. 前端 TaskEditor：`dayLoaded` 守卫——今日视图未命中该任务时禁用目标/主库字段并跳过写回。
- [x] 5. 插件事件接线：events.ts 导出模块级总线单例，App 复用；api.ts 在 createTask/taskCheckin(达标时)/addInboxItem/createNote 成功后发射；插件经 bridge 写入同样触发。PLUGIN_API.md 补 payload 文档。
- [x] 6. 插件重载：loadPlugin 在 onload 前清该 owner 的旧注册/订阅（保留跨 owner 冲突抛错语义，registry 测试不变）。
- [x] 7. snapshot：project_service 三用例 + inbox process_to_note 补 `snapshot::refresh`。
- [x] 8. context_service missed 查询加 `status: Active` 过滤；project_repo open_task_count 改 `= 'active'`。
- [x] 9. TodayView ⌘Z 跳过输入框焦点 + 同步清 lastActionTask 防连按；TaskDetailView 撤销按钮 busy 防抖。

## 验证

- [x] `bash scripts/check.sh` 全绿（Rust 全测含 3 新回归、vitest 22、前端 tsc+build）
- [ ] GUI 真机走查：新建任务选卡片样式/主库后确认生效；完成任务确认 pomodoro 自动开始；归档项目后 widget current_focus 更新

## 结果记录

2026-08-31 一次会话完成。**遗留决策**：undo 语义采用「补偿记入被撤记录所在日」＝允许跨日撤销且正确改写历史，未限定只撤今天。**本批未修**（需单独变更）：StatCard 环比昨日（涉及 TodayStats 字段 = CLI JSON 输出，属 Spec 级，见 2026-08-30-statcard-day-delta.md）；create_task/checkin 多表写入事务化；V3 迁移 user_version 与 schema 变更同事务；SQL 越界出现在 core/cli（红线 1）；历史账本中已存在的无指向负记录（complete_today 已可自愈补足，展示层钳位掩盖）。已安装插件目录若含 pomodoro 副本需重新 cp（见 AGENTS 记忆）。
