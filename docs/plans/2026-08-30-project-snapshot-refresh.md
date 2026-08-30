# Plan：project create/archive/delete 补刷 Widget Snapshot

> 级别：Plan · 状态：提案
> 提出日期：2026-08-30
> 关联：dev-log 2026-08-30「confirm 失效/导航参数/项目重命名」条（修复 update_project 时发现的既有缺口）

## 需求

- 现状：`update_project` 已按红线 4 在用例内 `log_activity` + `snapshot::refresh`；但 `create_project` / `archive_project` / `delete_project` 三个用例只写了 activity log，**未刷新 Widget Snapshot**——`current_focus` 依赖「首个活跃项目名」，这些操作后快照可能陈旧。
- 预期：三个用例补齐「写 activity log → snapshot::refresh」的完整闭环，与其他业务变更行为一致（红线 4）。

## 分步计划

- [ ] 1. core `project_service`：create/archive/delete 三用例各补 `snapshot::refresh`，与 update_project 写法对齐。
- [ ] 2. 补/扩 core 单测或集成测试：操作后快照文件 `current_focus`/任务计数反映最新状态。
- [ ] 3. 记 dev-log（切片提交）。

## 验证

- [ ] `bash scripts/check.sh` 全绿
- [ ] 人工验证：GUI 归档当前焦点项目后，读 widget-snapshot.json 确认 `current_focus` 已更新。

## 结果记录

（完成后填写）
