# Plans 索引

Plan 级任务（新增 CLI 子命令 / 新增视图 / 单个 command / 跨步小特性）在本目录建档；纯文案/Tailwind 微调与单点 bug 修复属 Vibe，不建档，只记 dev-log。历史 Plan 任务不追溯。

## 规则

1. **命名**：`YYYY-MM-DD-<slug>.md`（日期取提出日）。新建文档从 [_TEMPLATE.md](_TEMPLATE.md) 整段复制。
2. **结构固定**：需求（现状 + 预期复述）→ 分步计划 → 验证 → 结果记录。
3. **状态四态**：`提案 → 已确认·进行中 → 已完成（填完成日期）/ 已废弃`。
4. **流程**：提出 → 复述确认 → 执行（逐项勾选）→ 最小相关门禁通过 → 结果记录 + dev-log。
5. 若实施中发现影响面超出 Plan（新领域模块 / schema / 协议变更），**升级为 Spec**，在 `openspec/changes/` 另立变更包（四件套，见 `openspec/README.md`）并在本文件标注「已升级 → 链接」。

## 索引

| 文档 | 状态 | 提出日期 | 摘要 |
|---|---|---|---|
| [2026-09-17-v0.1-release.md](2026-09-17-v0.1-release.md) | 已完成 | 2026-09-17 | 首次公开发行、macOS 包与 CLI、README 和 GitHub Pages 展示 |
| [2026-09-08-job-tracker-ui-trial.md](2026-09-08-job-tracker-ui-trial.md) | 已完成 | 2026-09-08 | 独立求职台界面试用插件，已导入新版桌面程序；演示数据、真实 View/Today 卡片，不写入真实任务 |
| [2026-09-08-plugin-import-entry.md](2026-09-08-plugin-import-entry.md) | 已完成 | 2026-09-08 | 插件页统一导入 ZIP / 文件夹，预检信息、确认替换、默认停用与审阅启用，复用 Core 安装能力 |
| [2026-09-02-once-task-linger-history.md](2026-09-02-once-task-linger-history.md) | 已完成 | 2026-09-02 | 一次性任务完成当天留痕 + 任务墙历史翻页可见归档任务 + 当天 undo 自动解除归档（修订 004 可见性语义） |
| [2026-08-31-task-wall-modes.md](2026-08-31-task-wall-modes.md) | 已完成 | 2026-08-31 | 任务卡片墙双视图：均衡发牌混排（贪心最矮列）+ 类型分区，localStorage 记忆 |
| [2026-08-31-bugfix-batch.md](2026-08-31-bugfix-batch.md) | 已完成 | 2026-08-31 | 全量 bug 排查批修：账本错账/Tauri 参数契约/插件事件接线/快照缺口 |
| [2026-08-30-statcard-day-delta.md](2026-08-30-statcard-day-delta.md) | 提案 | 2026-08-30 | StatCard 接线「环比昨日」箭头，需 core 补昨日统计 |
| [2026-08-30-project-snapshot-refresh.md](2026-08-30-project-snapshot-refresh.md) | 已完成 | 2026-08-30 | project create/archive/delete 补刷 Widget Snapshot（红线 4 缺口） |
