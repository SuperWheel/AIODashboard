# Tasks 007: 插件平台演进

> 历史切片记录。2026-09-08：安全、权限与完整验收由 [008](../008-plugin-security-hardening/tasks.md) 接续；本页勾选只代表 007 当时交付范围，不代表当前安全保证。

## P1：协议、SDK 与权限

- [x] T-S1 manifest 双端兼容：旧 v1 manifest 继续通过，新字段校验一致。
- [ ] T-S2 API 版本不兼容、缺失权限和未知 capability 明确拒绝。
- [ ] T-S3 Core 细粒度权限矩阵：读写分别允许/拒绝。
- [ ] T-S4 capability token 与 plugin id 绑定；伪造 actor 不得回退 user。
- [x] T-S5 SDK 类型、JS/TS 脚手架和 plugin-test mock 可用。

## P2：安全与生命周期

- [ ] T-S6 网络代理重定向、localhost/私网、超时、大小和方法限制。
- [ ] T-S7 KV 配额和跨插件隔离。
- [ ] T-S8 register disposer、重复重载和停用清理。
- [ ] T-S9 声明式设置注册与持久化。
- [ ] T-S10 CSP/Trusted Plugin 警示和安全模式回归。

## P3：安装与分发

- [x] T-S11 zip staging 安装、路径穿越拒绝、哈希校验。
- [ ] T-S12 版本升级失败原子回滚。历史实现缺少完整故障恢复证明；由 008 的 journal/提交标记与注入故障测试完成。
- [x] T-S13 CLI `plugin pack/install/rollback` JSON 信封和审计。
- [x] T-S14 插件管理 UI 已有部分只读元数据；完整权限/完整性状态及管理操作由 008 接续。

## P4：文档与示例

- [ ] T-S15 重写 `docs/PLUGIN_API.md` 为完整开发手册。
- [ ] T-S16 echo/pomodoro 迁移到 SDK 模板并通过构建与测试。
- [ ] T-S17 新增一个网络集成示例，验证白名单和审计。
- [ ] T-S18 更新 README、openspec 索引和 docs/dev-log.md。

## 实施记录

- 2026-09-05：完成 manifest 可选 `api_version`、`min_host_version`、Core/UI capability 和 KV 配额字段；Rust/TS 双端校验及向后兼容测试通过。
- 2026-09-05：桥接层开始执行 Core capability；旧 manifest 保持兼容，新 manifest 未声明的 Core 写入会被拒绝；前端插件测试 24 项通过。
- 2026-09-05：Tauri actor 非法值改为显式拒绝；HTTP 代理关闭自动重定向并拒绝 localhost/私有 IP；CLI JS 脚手架同步新 manifest 字段和语义主题 token。
- 2026-09-05：CLI 新增 `plugin new <id> --template ts`，生成 `main.ts`、`plugin-sdk.d.ts` 和可用的 esbuild `package.json`；集成测试覆盖 JS/TS 两种脚手架。
- 2026-09-05：新增 `packages/plugin-sdk` 与 `packages/plugin-test`，提供公共 `PluginApi` 类型和离线测试桩；两包 TypeScript 构建通过。
- 2026-09-05：CLI 新增 `plugin pack/install/rollback`；安装先做 zip 路径检查，再解压到 staging 并校验 manifest，现有版本先备份后替换；集成测试通过。哈希校验仍待补充。
- 2026-09-05：补充 `shasum -a 256` 完整性校验；`pack` 输出 SHA-256，`install --sha256` 校验后才落盘；pack/install/rollback 写入 activity log，错误哈希回归测试通过。
- 2026-09-05：插件管理 UI 和列表读模型新增来源、协议版本、信任模式展示；SHA-256 持久化展示和安装状态操作仍待完成。
- 2026-09-05：新增 SCHEMA_V7 插件安装元数据（source、sha256、installed_version、previous_version）；安装后写入并在插件页显示截断 SHA-256。安装状态操作 UI 仍待完成。
- 2026-09-05：V7 迁移兼容手工 V4/V6 测试数据库；workspace cargo check 与 storage 迁移测试通过。
- 2026-09-05：插件管理页补充安装版本与上一版本标签；已完成元数据只读展示，尚未在 GUI 内直接执行安装/回滚。

## 2026-09-08 状态校正

- T-S1 的 v1 宽权限兼容已由 008 明确撤销；v1 仅保留诊断，不再运行。
- T-S2–10、12、15–18 的实施和验收证据统一记录在 008，不追溯伪造 007 完成日期。
- T-S11 原有路径/摘要检查不等于安全安装全部完成；symlink/hardlink、限额、崩溃恢复和加载后摘要检查属于 008。
- 示例仍为可直接运行的单文件 JS，使用 SDK 契约并通过共享测试桩；没有声称它们依赖尚未发布的 npm 包。
- GUI 安装/回滚明确保留 CLI-only，符合修复计划的备选决策。
