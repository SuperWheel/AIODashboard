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

- 统一信封：`{ "success": bool, "data": T|null, "error": {code,message}|null, "meta": {"schema_version":"1"} }`
- Exit Code：0/1/2/3/4/5（见 README）
- `--json` 全局开关；`--stdin` 复杂输入；`--dry-run` 危险操作预览
- `DASHBOARD_ACTOR` 环境变量标记来源，写入 `activity_log`

## Context 系统

`dashboard context today --json` 聚合返回：

今日日程（预留）、今日任务、逾期任务、完成统计、活跃项目、Inbox 数量、最近笔记。

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

## 数据库 Schema（V1）

`projects`, `tasks`, `notes`, `inbox_items`, `activity_log`, `widget_snapshots`。
迁移通过 `PRAGMA user_version` 控制。
