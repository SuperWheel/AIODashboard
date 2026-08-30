# Tasks 001: 插件系统

> 复选框约定：`[x]` 已完成（2026-08-30 全部完成）；测试清单为 TDD 先行确认项。

## P1 协议与宿主

- [x] T-S1 storage：SCHEMA_V2（plugin_registry + plugin_kv）+ 两个 repo + 迁移/隔离测试（a3e38a1）
- [x] T-S2 manifest 结构 + Rust 校验器 + 单测（a544ec2）
- [x] T-S3 tauri commands（plugin_list/set_enabled/load_source/kv_*/http_fetch 白名单）+ Actor::Plugin 贯通（56d4399）
- [x] T-S4 CLI `plugin list/enable/disable` + 集成测试（a544ec2）
- [x] T-S5 前端 vitest 基建 + check.sh/CI 增步（b090419）
- [x] T-S6 前端插件宿主：loader + ModuleRegistry + API 桥 + 权限执行 + 单测（b090419）
- [x] T-S7 echo 示例插件 + 端到端人工验证（b090419）
- [x] T-S8 dev-log + check.sh 全绿 + 切片提交（5dc2688）

## P2 UI 与生命周期

- [x] T-S9 槽位渲染：插件视图页签 / Today 卡片 / ⌘K 命令接入（ea7df42）
- [x] T-S10 托盘常驻 + 关窗隐藏 + 重开恢复（ea7df42；show_panel 修复 f739d6a）
- [x] T-S11 插件管理最小 UI（列表/启用开关/权限展示）（a212993）
- [x] T-S12 pomodoro 插件（kv 持久化 + 事件联动 + 卡片 + 视图）（a212993）
- [x] T-S13 dev-log + 门禁 + 切片提交（ed39f0e）

## P3 生态

- [x] `plugin new/dev` 脚手架（内嵌 AGENTS.md，纯 JS 模板）+ docs/PLUGIN_API.md（7a0fca8）
- [x] cron 全链路（croner → Tauri 事件 → CronRegistry）+ 权限确认 UI（首次发现逐项展示）（7a0fca8）
- [x] 插件加载看门狗（onload/onunload 超时/抛错自动停用）+ PluginErrorBoundary（7a0fca8 / 8aa749e）

## 测试用例清单（TDD：先确认后实现，全部通过）

**Rust（cargo）**
- [x] T1 plugin_kv repo：写入/覆盖/删除/列出；命名空间隔离 —— `kv_namespaced_by_plugin`
- [x] T2 迁移 V1→V2：user_version=2；旧表数据完整；新表存在 —— `migration_v1_to_v2_preserves_data`
- [x] T3 plugin_registry repo：默认启用；enable/disable 持久化往返 —— `registry_idempotent_and_persisted`
- [x] T4 CLI 集成：`plugin list --json` 信封形状；enable/disable 落库 + activity log；未知 id → exit 3 —— `plugin_list_enable_disable_roundtrip` / `plugin_unknown_id_exit_code_3`
- [x] T5 http 白名单：命中放行、未命中拒绝 —— `network_allowlist_check`（core 纯逻辑半边；宿主侧拦截由前端 T10 锁定）
- [x] T6 manifest 校验：缺字段 / 非法 id / entry 缺失 → 明确错误码 —— `manifest_validate_rejects_bad_fields` / `load_from_dir_checks_entry_exists` / 集成 `plugin_list_reports_invalid_manifest`

**前端（vitest）**
- [x] T7 manifest 校验（loader 侧）与 Rust 规则一致 —— `validateManifest` describe
- [x] T8 onunload 反注册全部贡献点 —— `unregisterOwner` / `offOwner` 用例
- [x] T9 ModuleRegistry：注册→查询→注销；id 冲突拒绝
- [x] T10 权限桥：未声明 network 的 fetch / 未声明事件 / kv 越权命名空间 → 拒绝
- [x] T11 事件总线：emit→handler 收到；handler 抛错不炸宿主

**端到端 / 人工**
- [x] T12 echo：装载 → Today 卡片显示 → disable 后卡片消失（真机走查 + 启停热加载修复 b972abb）
- [x] T13 pomodoro：kv 持久化 + task.completed 联动 + 托盘后台运行（真机走查）
- [x] T14 托盘：关窗 → 进程留存 → 托盘/Dock 重开恢复（真机走查，含 show_panel 修复）
- [x] T15 门禁：check.sh 六步全绿；CI 同构

## 实施记录

- P1：a3e38a1 / a544ec2 / 56d4399 / b090419（切片 1–4）；P2：ea7df42 / a212993（切片 5–7）；P3：7a0fca8（切片 8–9）。
- 实施中修复：storage 桥接对齐 `api.storage.kv.*` 协议 + 渲染错误边界（8aa749e）；插件启停热加载 + 重载按钮（b972abb）；托盘 show_panel 统一（f739d6a）。
- 偏离：脚手架改纯 JS 零工具链模板（见 design 决策 6）；协议文档/桥/示例三方锁死纪律由此确立（教训见 dev-log 2026-08-30）。
