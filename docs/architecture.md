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

- 统一信封：`{ "success": bool, "data": T|null, "error": {code,message}|null, "meta": {"schema_version":"6"} }`
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

## 数据库 Schema（V8）

`projects`, `tasks`（打卡式：icon/color_hex/unit/card_style）, `notes`, `inbox_items`,
`activity_log`, `widget_snapshots`, `plugin_registry`, `plugin_kv`，以及打卡体系五表：
`completion_records`（append-only 账本，operation_id 唯一约束幂等）、
`task_target_periods`（历史目标区间）、`task_activity_periods`（活动区间）、
`date_libraries`（纪念日/倒计时日重要日）、`task_library_membership_periods`（归属区间）。
迁移通过 `PRAGMA user_version` 控制，只增不改（新变更 = 新 SCHEMA_V{n} 分支）。

## 打卡引擎（task-checkin-cards，变更包 003）

- 逻辑日 = 本地时区 YYYY-MM-DD，唯一换算入口 `context_service::local_today`。
- 状态判定与统计口径为 core 纯函数（`day_state`）：五态（not_applicable/pending/in_progress/completed/missed）+ 热力六态（future/zero/partial_low/partial_high/complete）。
- 统计：完整完成率 = 完整完成天数/适用天数；连续天数遇非 complete 即断（不适用日也断）；
  今日完成率 = Σmin(count,target)/Σtarget；重要日完成率 = 当日有效直属任务 min(count/target,1) 均值。
- 打卡/减少/撤销均写补偿账本；UI 上的 +/- 为圆角矩形同侧并排（设计决策 7）。

## 插件平台（Spec 008）

插件是通过受控 API 调用 Core 的 Trusted WebView 客户端。`plugin_runtime` 在 Core 管理 token、manifest/内容摘要、启用修订和权限；Tauri 只包装命令，不能由插件传入任意 actor/id。GUI 普通 command 拒绝 plugin/cli/ai actor 冒用。这个绑定防止 API 误用，不能隔离共享 WebView 中的恶意 JavaScript。

`plugin_package` 管理普通文件校验、ZIP 限额、跨进程文件锁、安装 journal 与恢复；`plugin_network` 管理 URL/DNS 校验、固定目标地址、GET 和资源限制。V8 在 plugin_registry 增内容摘要、授权摘要、revision 和 install_operation，迁移停用既有插件。安装/升级/回滚后须再次授权。

前端与离线测试桩共用 SDK 权限和 Lifecycle；disposer 清理注册、订阅、cron、SDK 定时器。PluginHost 串行重载、取消失效代；GUI 每 2 秒轮询插件状态指纹，CLI 变更触发自动重载与 cron 重扫。异步导入/onload 有 8 秒限制，同步死循环仍须 CLI safe-mode 后重启。

Core 写入与插件管理操作保留 activity log；网络成功/失败及权限拒绝也审计。高频插件 KV 仅为插件内部状态，不逐键生成 activity/snapshot；领域写入仍走既有 Core 用例的审计与快照。领域事件仅订阅，自定义事件限 `plugin.<id>:<topic>`。声明式设置保存在插件 quota 内的 KV。

桌面插件页通过原生选择器统一导入 ZIP / 目录。`plugin_package::preview_import` 只读生成来源内容摘要与目标状态凭据；`import_checked` 在安装锁内复核后复用安装事务，安装 actor 由宿主固定为 User，CLI 原入口保持 Cli。提交成功后撤销该插件旧上下文、同步 cron 并重载前端贡献，插件默认停用，需重新审阅才能加载。回滚继续通过 CLI。

GUI 同时提供权限确认、完整性/来源状态、启停、重载、设置及停用全部。完整边界与验收见 [PLUGIN_API](PLUGIN_API.md)、[008 tasks](../openspec/changes/008-plugin-security-hardening/tasks.md) 和 [桌面导入 Plan](plans/2026-09-08-plugin-import-entry.md)。
