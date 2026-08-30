# 架构说明

对应总体设计文档 V0.1 的 MVP 实现（Phase 0 + Phase 1）。

## 六层结构落地

```text
Presentation   React GUI (apps/desktop/src)
               CLI      (apps/cli → `dashboard`)
                    │
Interface      Tauri Commands (apps/desktop/src-tauri/src/lib.rs)
               clap Commands  (apps/cli/src/main.rs)
                    │
Application    dashboard-core: task/project/note/inbox/search/context/snapshot services
                    │
Domain         dashboard-domain: 实体与枚举，零外部依赖
                    │
Infrastructure dashboard-storage: rusqlite，唯一写 SQL 的 crate
                    │
Platform       macOS（SQLite WAL 支持多进程并发）
```

关键约束：

- Domain / Application 不依赖 GUI、CLI、Tauri。
- 业务规则只存在一份：`create_task()` 只在 core 实现一次，GUI 与 CLI 都调用它。
- 前端不直接触碰 SQLite。

## AI CLI 协议

- 统一信封：`{ "success": bool, "data": T|null, "error": {code,message}|null, "meta": {"schema_version":"2"} }`
- Exit Code：0/1/2/3/4/5（见 README）
- `--json` 全局开关；`--stdin` 复杂输入；`--dry-run` 危险操作预览
- `DASHBOARD_ACTOR` 环境变量标记来源，写入 `activity_log`

## Context 系统

`dashboard context today --json` 聚合返回：

今日日程（预留）、今日任务（含打卡状态）、完成率/近7天错过统计、活跃项目、Inbox 数量、最近笔记。

AI 无需连续执行十几个命令即可了解用户当前状态。

## Widget Snapshot

Rust Core 在每次业务变更后：

1. 计算简单快照（`widget.snapshot/v1`）；
2. UPSERT 到 `widget_snapshots` 表；
3. 原子写入快照文件（App Group 目录优先，兜底 DB 同目录）。

未来 SwiftUI Widget 只需读取该 JSON 文件并展示，不接触数据库 Schema —— 降低平台耦合。

## 同步机制（MVP）

```text
CLI/AI 写库 ──► SQLite WAL
GUI 轮询(4s) + focus 刷新 ──► 读同一数据库 ──► UI 更新
```

按设计文档 §26，暂不实现 Daemon；后续如需要可演进为 `dashboardd`。

## 测试策略

- 单元测试：core（时间区间、枚举解析）
- 集成测试：`apps/cli/tests/integration.rs`
  - CLI 创建 → Core 读取（链路一）
  - Core 创建 → CLI `--json` 读取 + 信封稳定性（链路二）
  - CLI 完成 → Core 状态同步（状态流转）
  - not found → exit code 3 + 错误信封（AI 错误协议）
  - context today → 快照文件生成（链路四的前半段）
  - dry-run → 不产生副作用

## 数据库 Schema（V3）

`projects`, `tasks`（打卡式：icon/color_hex/unit/card_style）, `notes`, `inbox_items`,
`activity_log`, `widget_snapshots`, `plugin_registry`, `plugin_kv`，以及打卡体系五表：
`completion_records`（append-only 账本，operation_id 唯一约束幂等）、
`task_target_periods`（历史目标区间）、`task_activity_periods`（活动区间）、
`date_libraries`（纪念日/倒计时日主库）、`task_library_membership_periods`（归属区间）。
迁移通过 `PRAGMA user_version` 控制，只增不改（新变更 = 新 SCHEMA_V{n} 分支）。

## 打卡引擎（task-checkin-cards，变更包 003）

- 逻辑日 = 本地时区 YYYY-MM-DD，唯一换算入口 `context_service::local_today`。
- 状态判定与统计口径为 core 纯函数（`day_state`）：五态（not_applicable/pending/in_progress/completed/missed）+ 热力六态（future/zero/partial_low/partial_high/complete）。
- 统计：完整完成率 = 完整完成天数/适用天数；连续天数遇非 complete 即断（不适用日也断）；
  今日完成率 = Σmin(count,target)/Σtarget；主库完成率 = 当日有效直属任务 min(count/target,1) 均值。
- 打卡/减少/撤销均写补偿账本；UI 上的 +/- 为圆角矩形同侧并排（设计决策 7）。
