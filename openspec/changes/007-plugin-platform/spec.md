# Spec 007: 插件平台演进（plugin-platform/v1.1）

## ADDED

### ADDED-1：公共 SDK 与开发模板

- **要求**：提供 `@aiodashboard/plugin-sdk` 的 TypeScript 类型、运行时辅助函数和 `@aiodashboard/plugin-test` 测试桩；插件不得打包宿主 React 实例。
- **要求**：`dashboard plugin new` 支持 JS/TS 模板，生成 `manifest.json`、入口、README、LICENSE、AGENTS.md、构建和测试配置。
- **验收**：新模板可构建、可通过 `plugin dev` 校验，并能在测试桩中注册命令、事件和 KV。

### ADDED-2：Manifest 兼容性与能力声明

- **要求**：manifest 支持 `api_version`、`min_host_version`、作者/许可证/主页、`permissions.core`、`permissions.ui`、`permissions.storage.quota_bytes` 和可选完整性哈希；旧 v1 字段继续有效。
- **要求**：未知字段向前兼容；破坏性 API 变化必须升级协议版本。
- **验收**：Rust 与 TypeScript 双端校验规则一致；缺少必需能力、版本不兼容、超出配额或非法权限时给出明确错误。

### ADDED-3：细粒度权限与调用方绑定

- **要求**：Core API 按读写能力拆分；未声明能力拒绝调用并记录 `plugin.denied`。
- **要求**：Tauri 命令不得把前端任意传入的 actor 当作可信身份；插件调用必须绑定宿主建立的插件上下文或 capability token。
- **验收**：伪造 actor、跨插件 KV、未授权 Core 写入和未授权 UI 注册均被拒绝；合法调用仍记录 `plugin:<id>`。

### ADDED-4：网络与存储安全边界

- **要求**：HTTP 代理限制方法、请求体大小、响应体大小、超时、重定向目标，并拒绝 localhost、回环、链路本地和私有地址；每次请求审计。
- **要求**：KV 配额按插件强制执行；插件停用后数据仍保留但不可被其他插件读取。
- **验收**：白名单命中、重定向越权、内网地址、超额写入和超时均有对应测试。

### ADDED-5：安装、导入、校验与回滚

- **要求**：支持从本地目录或 zip 导入插件到 staging 目录，校验目录名、manifest、入口、版本和哈希后原子安装。
- **要求**：保留上一版本，升级失败可回滚；安装/升级/回滚写 activity log。
- **验收**：损坏包、路径穿越、重复 id、入口缺失、哈希不匹配不会覆盖现有插件；成功升级可重载，失败可恢复。

### ADDED-6：声明式设置与生命周期清理

- **要求**：插件可注册宿主渲染的设置定义；注册事件、定时器、视图和命令均获得 disposer，停用/重载时自动清理。
- **验收**：重复重载不产生重复注册；停用后无残留事件、cron、视图、卡片或命令。

### ADDED-7：插件开发手册与验收流程

- **要求**：更新 `docs/PLUGIN_API.md`，覆盖 Vibe Coding 提示、SDK、manifest、权限、UI、测试、调试、发布、安全和故障排查。
- **要求**：新增插件验收清单：构建、权限、审计、双主题、重载、停用、数据隔离和回滚。
- **验收**：echo、pomodoro 和一个网络集成示例按手册完成开发并通过自动测试。

## MODIFIED

- `docs/PLUGIN_API.md`：从 v1 API 参考扩展为公共开发手册。
- `apps/desktop/src/plugins/bridge.ts`：增加能力矩阵和上下文校验。
- `apps/desktop/src-tauri/src/lib.rs`：插件命令改为拒绝非法 actor，不再回退到 user。
- `crates/dashboard-core/src/plugin_manifest.rs`：兼容性、权限和完整性校验。
- `apps/cli`：增加 `plugin pack/install/rollback`（保留已有命令兼容）。
- `openspec/README.md`：增加本变更包索引。

## 非本变更范围

- 不实现 Worker/WASM/外部进程沙箱；只记录协议边界和迁移方向。
- 不实现公共插件市场、在线账号体系或跨设备同步。
- 不允许插件绕过 Core 新增 SQL、直接修改 SQLite 或直接调用未公开 Tauri command。
- 不新增领域实体；新实体仍须遵守 AGENTS.md 的六层 Checklist。
