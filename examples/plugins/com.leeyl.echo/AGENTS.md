# Echo 插件（AI 开发说明）

最小单文件 ESM 示例，协议 `plugin.protocol/v2`。复制后同步修改目录名、manifest.id 与贡献点声明。

- `context.read` 授权 today；`ui: ["today_card"]` 与 contributions.today_cards 共同授权卡片；显式 quota 授权插件独立 KV。
- 未声明能力即拒绝。Core 写入由宿主上下文派生 actor；不得向 API 传入 actor/plugin_id。
- 从 `api.react` 使用宿主 React。所有注册返回 disposer，卸载由宿主清理 SDK 资源。
- 领域事件只允许声明后订阅（panel 生命周期事件除外）；自定义事件只用 `plugin.<id>:<topic>`。
- 开发脚手架：`dashboard plugin new com.example.demo --template ts`；`plugin dev 路径 --run` 执行构建和测试。
- 安装默认停用；在 GUI 审阅启用，修改后重新确认。CLI 停用会自动同步到 GUI。

权威 API 与安全边界见 [开发手册](../../../docs/PLUGIN_API.md)。
