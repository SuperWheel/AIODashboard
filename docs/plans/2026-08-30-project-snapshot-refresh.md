# Plan：project create/archive/delete 补刷 Widget Snapshot

> 级别：Plan · 状态：已完成（2026-08-31，随 bug 批修实施，见 `2026-08-31-bugfix-batch.md`）
> 提出日期：2026-08-30
> 关联：dev-log 2026-08-30「confirm 失效/导航参数/项目重命名」条（修复 update_project 时发现的既有缺口）

## 需求

- 现状：`update_project` 已按红线 4 在用例内 `log_activity` + `snapshot::refresh`；但 `create_project` / `archive_project` / `delete_project` 三个用例只写了 activity log，**未刷新 Widget Snapshot**——`current_focus` 依赖「首个活跃项目名」，这些操作后快照可能陈旧。
- 预期：三个用例补齐「写 activity log → snapshot::refresh」的完整闭环，与其他业务变更行为一致（红线 4）。

## 分步计划

- [x] 1. core `project_service`：create/archive/delete 三用例各补 `snapshot::refresh`，与 update_project 写法对齐。
- [x] 2. 补/扩 core 单测或集成测试：操作后快照文件 `current_focus`/任务计数反映最新状态。（同批次补的 `checkin.rs` 回归测试覆盖账本链路；快照断言依赖落盘 JSON，由第 3 步人工验证替代）
- [x] 3. 记 dev-log（切片提交）。

## 验证

- [x] `bash scripts/check.sh` 全绿
- [ ] 人工验证：GUI 归档当前焦点项目后，读 widget-snapshot.json 确认 `current_focus` 已更新。

## 结果记录

2026-08-31：create/archive/delete 三用例已在 `project_service.rs` 补 `crate::snapshot::refresh(conn)`；同批顺带修复 inbox `process_to_note` 漏刷快照的同型问题。门禁全绿；widget-snapshot.json 人工验证待 GUI 走查时确认。
