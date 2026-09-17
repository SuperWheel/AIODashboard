# 参与 AIODashboard

欢迎提交问题、文档改进和可验证的代码变更。提问时请提供应用版本、macOS 版本、芯片架构、复现步骤、预期结果与实际结果；附截图前请移除个人信息。不要上传自己的数据库、访问令牌或未脱敏日志。

开始开发前阅读 [AGENTS.md](AGENTS.md) 与 [架构说明](docs/architecture.md)。业务规则放 Rust Core，SQL 仅放 storage；GUI 与 CLI 共用业务逻辑。协议或数据库变化须先提交 Spec 提案。

```bash
npm ci
npm ci --prefix apps/desktop
bash scripts/check.sh frontend  # 纯前端
bash scripts/check.sh rust      # 纯 Rust
bash scripts/check.sh full      # 跨层、协议、迁移与发行
```

PR 请说明问题、变更后的行为、验证结果和已知限制。保持一次变更一个意图；有新行为时补对应测试。涉及数据的本地验证应设置独立的 `DASHBOARD_DB_PATH`、`DASHBOARD_PLUGINS_DIR` 和 `DASHBOARD_WIDGET_SNAPSHOT_PATH`。

插件贡献请同时阅读 [插件开发手册](docs/PLUGIN_API.md)。当前运行时只适合可信插件，勿把它描述为恶意代码沙箱。
